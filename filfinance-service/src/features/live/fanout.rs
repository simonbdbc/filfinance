//! Fan-out Redis du WebSocket `live` pour le pilote Bunny DEV.
//!
//! Le transport Bunny reste stateless: Redis porte le snapshot et relaie chaque evenement entre
//! les replicas Rust. Une panne Redis ne bloque jamais l'ingestion ni la diffusion locale.

use std::{
    io,
    sync::atomic::{AtomicU64, Ordering},
};

use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader},
    net::{
        TcpStream,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
    sync::{broadcast, watch},
    task::JoinHandle,
    time::{Duration, sleep},
};
use tracing::{debug, info, warn};

use crate::{
    config::LiveFanoutConfig,
    errors::AppError,
    features::live::dto::{EdgeLiveMessage, EdgeLiveSnapshot},
};

const TRANSLATED_HISTORY_LIMIT: usize = 50;
const AUDIO_HISTORY_LIMIT: usize = 20;
static INSTANCE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Client leger Redis; chaque commande courte ouvre une connexion afin qu'une indisponibilite ne
/// puisse ni epuiser le pool HTTP ni retenir le chemin d'ingestion.
#[derive(Clone)]
pub(crate) struct LiveFanout {
    target: RedisTarget,
    channel: String,
    snapshot_key: String,
    instance_id: String,
}

#[derive(Clone)]
struct RedisTarget {
    host: String,
    port: u16,
    username: Option<String>,
    password: Option<String>,
    database: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct FanoutEnvelope {
    publisher_id: String,
    message: EdgeLiveMessage,
}

impl LiveFanout {
    /// Construit le relais apres la validation stricte de `LiveFanoutConfig`.
    pub(crate) fn from_config(config: &LiveFanoutConfig) -> Result<Option<Self>, AppError> {
        if !config.enabled {
            return Ok(None);
        }
        let raw_url = config
            .redis_url
            .as_deref()
            .ok_or_else(|| AppError::Config("NEWS_LIVE_REDIS_URL is missing".to_owned()))?;
        let url = url::Url::parse(raw_url)
            .map_err(|error| AppError::Config(format!("invalid NEWS_LIVE_REDIS_URL: {error}")))?;
        let host = url.host_str().ok_or_else(|| {
            AppError::Config("NEWS_LIVE_REDIS_URL must contain a host".to_owned())
        })?;
        let database = url
            .path()
            .trim_start_matches('/')
            .parse::<u32>()
            .unwrap_or(0);
        Ok(Some(Self {
            target: RedisTarget {
                host: host.to_owned(),
                port: url.port().unwrap_or(6379),
                username: if url.username().is_empty() {
                    None
                } else {
                    Some(url.username().to_owned())
                },
                password: url.password().map(str::to_owned),
                database,
            },
            channel: config.redis_channel.clone(),
            snapshot_key: config.redis_snapshot_key.clone(),
            instance_id: format!(
                "{}-{}-{}",
                std::process::id(),
                crate::shared::time::now_rfc3339(),
                INSTANCE_SEQUENCE.fetch_add(1, Ordering::Relaxed),
            ),
        }))
    }

    /// Publie sans attendre Redis. Le caller a deja diffuse localement, ce qui preserve
    /// l'ingestion et les sockets de ce replica pendant une panne du relais.
    pub(crate) fn publish_best_effort(&self, message: EdgeLiveMessage) {
        let fanout = self.clone();
        tokio::spawn(async move {
            if let Err(error) = fanout.publish_to_redis(&message).await {
                warn!(%error, "live Redis fan-out unavailable; local delivery was preserved");
            }
        });
    }

    /// Lit le snapshot pour le `hello`; l'appelant peut basculer vers un snapshot vide si Redis est
    /// indisponible.
    pub(crate) async fn read_snapshot(&self) -> Result<EdgeLiveSnapshot, io::Error> {
        let (mut reader, mut writer) = self.open().await?;
        let response = command(
            &mut reader,
            &mut writer,
            &[b"GET".to_vec(), self.snapshot_key.as_bytes().to_vec()],
        )
        .await?;
        match response {
            Resp::Bulk(Some(bytes)) => match serde_json::from_slice::<EdgeLiveSnapshot>(&bytes) {
                Ok(snapshot) => Ok(normalize_snapshot(snapshot)),
                Err(error) => {
                    warn!(%error, "stored live Redis snapshot is invalid; ignoring it");
                    Ok(empty_snapshot())
                }
            },
            Resp::Bulk(None) => Ok(empty_snapshot()),
            other => Err(io::Error::other(format!(
                "unexpected Redis GET response: {other:?}"
            ))),
        }
    }

    /// Abonnement permanent au canal Pub/Sub des autres replicas.
    pub(crate) fn spawn_subscriber(
        self,
        tx: broadcast::Sender<EdgeLiveMessage>,
        mut shutdown_rx: watch::Receiver<bool>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                if *shutdown_rx.borrow() {
                    info!("live Redis fan-out subscriber stopped");
                    return;
                }
                match self.subscribe_once(&tx, &mut shutdown_rx).await {
                    Ok(()) => return,
                    Err(error) => {
                        warn!(%error, "live Redis subscriber disconnected; retrying");
                        tokio::select! {
                            _ = sleep(Duration::from_secs(2)) => {}
                            changed = shutdown_rx.changed() => {
                                if changed.is_err() || *shutdown_rx.borrow() { return; }
                            }
                        }
                    }
                }
            }
        })
    }

    async fn publish_to_redis(&self, message: &EdgeLiveMessage) -> Result<(), io::Error> {
        let envelope = serde_json::to_vec(&FanoutEnvelope {
            publisher_id: self.instance_id.clone(),
            message: message.clone(),
        })
        .map_err(io::Error::other)?;

        // WATCH/MULTI/EXEC evite qu'un replica ecrase le snapshot mis a jour par un autre.
        for attempt in 0..3 {
            let (mut reader, mut writer) = self.open().await?;
            expect_ok(
                command(
                    &mut reader,
                    &mut writer,
                    &[b"WATCH".to_vec(), self.snapshot_key.as_bytes().to_vec()],
                )
                .await?,
            )?;
            let stored = command(
                &mut reader,
                &mut writer,
                &[b"GET".to_vec(), self.snapshot_key.as_bytes().to_vec()],
            )
            .await?;
            let snapshot = match stored {
                Resp::Bulk(Some(value)) => serde_json::from_slice::<EdgeLiveSnapshot>(&value)
                    .map(normalize_snapshot)
                    .unwrap_or_else(|error| {
                        warn!(%error, "stored live Redis snapshot is invalid; rebuilding it");
                        empty_snapshot()
                    }),
                Resp::Bulk(None) => empty_snapshot(),
                other => {
                    return Err(io::Error::other(format!(
                        "unexpected Redis GET response: {other:?}"
                    )));
                }
            };
            let next_snapshot = apply_snapshot(snapshot, message);
            let next_snapshot = serde_json::to_vec(&next_snapshot).map_err(io::Error::other)?;

            expect_ok(command(&mut reader, &mut writer, &[b"MULTI".to_vec()]).await?)?;
            expect_queued(
                command(
                    &mut reader,
                    &mut writer,
                    &[
                        b"SET".to_vec(),
                        self.snapshot_key.as_bytes().to_vec(),
                        next_snapshot,
                    ],
                )
                .await?,
            )?;
            expect_queued(
                command(
                    &mut reader,
                    &mut writer,
                    &[
                        b"PUBLISH".to_vec(),
                        self.channel.as_bytes().to_vec(),
                        envelope.clone(),
                    ],
                )
                .await?,
            )?;
            match command(&mut reader, &mut writer, &[b"EXEC".to_vec()]).await? {
                Resp::Array(Some(_)) => return Ok(()),
                Resp::Array(None) if attempt < 2 => {
                    debug!(
                        attempt,
                        "live Redis snapshot transaction raced another replica"
                    );
                }
                Resp::Array(None) => {
                    return Err(io::Error::other(
                        "live Redis snapshot transaction conflicted",
                    ));
                }
                other => {
                    return Err(io::Error::other(format!(
                        "unexpected Redis EXEC response: {other:?}"
                    )));
                }
            }
        }
        Err(io::Error::other(
            "live Redis snapshot transaction did not complete",
        ))
    }

    async fn subscribe_once(
        &self,
        tx: &broadcast::Sender<EdgeLiveMessage>,
        shutdown_rx: &mut watch::Receiver<bool>,
    ) -> Result<(), io::Error> {
        let (mut reader, mut writer) = self.open().await?;
        let confirmation = command(
            &mut reader,
            &mut writer,
            &[b"SUBSCRIBE".to_vec(), self.channel.as_bytes().to_vec()],
        )
        .await?;
        if !matches!(confirmation, Resp::Array(Some(_))) {
            return Err(io::Error::other(
                "Redis did not acknowledge live subscription",
            ));
        }

        loop {
            tokio::select! {
                changed = shutdown_rx.changed() => {
                    if changed.is_err() || *shutdown_rx.borrow() { return Ok(()); }
                }
                received = read_resp(&mut reader) => {
                    let received = received?;
                    let Some(bytes) = pubsub_message_bytes(received) else { continue; };
                    let envelope: FanoutEnvelope = match serde_json::from_slice(&bytes) {
                        Ok(value) => value,
                        Err(error) => { warn!(%error, "live Redis Pub/Sub payload is invalid"); continue; }
                    };
                    if envelope.publisher_id != self.instance_id {
                        let _ = tx.send(envelope.message);
                    }
                }
            }
        }
    }

    async fn open(&self) -> Result<(BufReader<OwnedReadHalf>, OwnedWriteHalf), io::Error> {
        let stream = TcpStream::connect((self.target.host.as_str(), self.target.port)).await?;
        let (read, mut write) = stream.into_split();
        let mut reader = BufReader::new(read);
        if let Some(password) = &self.target.password {
            let mut args = vec![b"AUTH".to_vec()];
            if let Some(username) = &self.target.username {
                args.push(username.as_bytes().to_vec());
            }
            args.push(password.as_bytes().to_vec());
            expect_ok(command(&mut reader, &mut write, &args).await?)?;
        }
        if self.target.database != 0 {
            expect_ok(
                command(
                    &mut reader,
                    &mut write,
                    &[
                        b"SELECT".to_vec(),
                        self.target.database.to_string().into_bytes(),
                    ],
                )
                .await?,
            )?;
        }
        Ok((reader, write))
    }
}

/// Le point d'entree unique conserve la compatibilite `/ws/news`, Redis DEV et Worker production.
pub(crate) fn publish_local(state: &crate::app_state::AppState, message: EdgeLiveMessage) {
    let _ = state.live_metadata_tx.send(message.clone());
    if let Some(fanout) = &state.live_fanout {
        fanout.publish_best_effort(message);
    }
}

pub(crate) fn empty_snapshot() -> EdgeLiveSnapshot {
    EdgeLiveSnapshot {
        latest_event_id: None,
        now_playing: None,
        audio_state: None,
        translated_news_items: Vec::new(),
        audio_events: Vec::new(),
        source_statuses: Vec::new(),
        calendar_day_summary: None,
    }
}

fn normalize_snapshot(snapshot: EdgeLiveSnapshot) -> EdgeLiveSnapshot {
    EdgeLiveSnapshot {
        latest_event_id: snapshot.latest_event_id,
        now_playing: snapshot.now_playing,
        audio_state: snapshot.audio_state,
        translated_news_items: bound_history(
            snapshot.translated_news_items,
            TRANSLATED_HISTORY_LIMIT,
        ),
        audio_events: bound_history(snapshot.audio_events, AUDIO_HISTORY_LIMIT),
        source_statuses: snapshot.source_statuses,
        calendar_day_summary: snapshot.calendar_day_summary,
    }
}

pub(crate) fn apply_snapshot(
    mut snapshot: EdgeLiveSnapshot,
    message: &EdgeLiveMessage,
) -> EdgeLiveSnapshot {
    snapshot = normalize_snapshot(snapshot);
    let event_id = message_event_id(message);
    snapshot.latest_event_id = Some(event_id);
    match message {
        EdgeLiveMessage::NowPlaying { payload } => {
            snapshot.now_playing = payload.now_playing.clone()
        }
        EdgeLiveMessage::AudioState { payload } => {
            snapshot.audio_state = Some(payload.state.clone())
        }
        EdgeLiveMessage::TranslatedNewsItem { .. } => append_history(
            &mut snapshot.translated_news_items,
            message,
            TRANSLATED_HISTORY_LIMIT,
        ),
        EdgeLiveMessage::AudioReady { .. }
        | EdgeLiveMessage::AudioFailed { .. }
        | EdgeLiveMessage::AudioFinalized { .. }
        | EdgeLiveMessage::CalendarAlertReady { .. } => {
            append_history(&mut snapshot.audio_events, message, AUDIO_HISTORY_LIMIT)
        }
        EdgeLiveMessage::SourceStatus { payload } => {
            snapshot.source_statuses = payload.sources.clone();
            snapshot.calendar_day_summary = payload.calendar_day_summary.clone();
        }
        EdgeLiveMessage::Hello { .. }
        | EdgeLiveMessage::Heartbeat { .. }
        | EdgeLiveMessage::NewsCard { .. }
        | EdgeLiveMessage::SystemNotice { .. }
        | EdgeLiveMessage::Lagged { .. } => {}
    }
    snapshot
}

fn append_history(history: &mut Vec<serde_json::Value>, message: &EdgeLiveMessage, limit: usize) {
    let value = match serde_json::to_value(message) {
        Ok(value) => value,
        Err(_) => return,
    };
    let event_id = message_event_id(message);
    history.retain(|existing| {
        existing
            .pointer("/payload/event_id")
            .and_then(serde_json::Value::as_str)
            != Some(event_id.as_str())
    });
    history.push(value);
    *history = bound_history(std::mem::take(history), limit);
}

fn bound_history(history: Vec<serde_json::Value>, limit: usize) -> Vec<serde_json::Value> {
    let mut deduplicated = Vec::new();
    for value in history.into_iter().rev() {
        let duplicate = value
            .pointer("/payload/event_id")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|event_id| {
                deduplicated.iter().any(|item: &serde_json::Value| {
                    item.pointer("/payload/event_id")
                        .and_then(serde_json::Value::as_str)
                        == Some(event_id)
                })
            });
        if !duplicate {
            deduplicated.push(value);
        }
        if deduplicated.len() == limit {
            break;
        }
    }
    deduplicated.reverse();
    deduplicated
}

fn message_event_id(message: &EdgeLiveMessage) -> String {
    match message {
        EdgeLiveMessage::Hello { .. } => "hello".to_owned(),
        EdgeLiveMessage::Heartbeat { .. } => "heartbeat".to_owned(),
        EdgeLiveMessage::NewsCard { payload } => payload.event_id.clone(),
        EdgeLiveMessage::TranslatedNewsItem { payload } => payload.event_id.clone(),
        EdgeLiveMessage::NowPlaying { payload } => payload.event_id.clone(),
        EdgeLiveMessage::AudioState { payload } => payload.event_id.clone(),
        EdgeLiveMessage::AudioReady { payload } => payload.event_id.clone(),
        EdgeLiveMessage::CalendarAlertReady { payload } => payload.event_id.clone(),
        EdgeLiveMessage::AudioFailed { payload } => payload.event_id.clone(),
        EdgeLiveMessage::AudioFinalized { payload } => payload.event_id.clone(),
        EdgeLiveMessage::SystemNotice { payload } => payload.event_id.clone(),
        EdgeLiveMessage::SourceStatus { payload } => payload.event_id.clone(),
        EdgeLiveMessage::Lagged { .. } => "lagged".to_owned(),
    }
}

#[derive(Debug)]
enum Resp {
    Simple(String),
    Error(String),
    Integer,
    Bulk(Option<Vec<u8>>),
    Array(Option<Vec<Resp>>),
}

async fn command<R, W>(reader: &mut R, writer: &mut W, args: &[Vec<u8>]) -> Result<Resp, io::Error>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    writer
        .write_all(format!("*{}\r\n", args.len()).as_bytes())
        .await?;
    for arg in args {
        writer
            .write_all(format!("${}\r\n", arg.len()).as_bytes())
            .await?;
        writer.write_all(arg).await?;
        writer.write_all(b"\r\n").await?;
    }
    writer.flush().await?;
    read_resp(reader).await
}

async fn read_resp<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Resp, io::Error> {
    let mut marker = [0_u8; 1];
    reader.read_exact(&mut marker).await?;
    let line = read_line(reader).await?;
    match marker[0] {
        b'+' => Ok(Resp::Simple(String::from_utf8_lossy(&line).into_owned())),
        b'-' => Ok(Resp::Error(String::from_utf8_lossy(&line).into_owned())),
        b':' => {
            parse_len(&line)?;
            Ok(Resp::Integer)
        }
        b'$' => {
            let length = parse_len(&line)?;
            if length < 0 {
                return Ok(Resp::Bulk(None));
            }
            let mut bytes = vec![0; length as usize];
            reader.read_exact(&mut bytes).await?;
            let mut suffix = [0_u8; 2];
            reader.read_exact(&mut suffix).await?;
            if suffix != *b"\r\n" {
                return Err(io::Error::other("invalid Redis bulk terminator"));
            }
            Ok(Resp::Bulk(Some(bytes)))
        }
        b'*' => {
            let length = parse_len(&line)?;
            if length < 0 {
                return Ok(Resp::Array(None));
            }
            let mut items = Vec::with_capacity(length as usize);
            for _ in 0..length {
                items.push(Box::pin(read_resp(reader)).await?);
            }
            Ok(Resp::Array(Some(items)))
        }
        _ => Err(io::Error::other("invalid Redis response marker")),
    }
}

async fn read_line<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Vec<u8>, io::Error> {
    let mut value = Vec::new();
    loop {
        let mut byte = [0_u8; 1];
        reader.read_exact(&mut byte).await?;
        if byte[0] == b'\r' {
            reader.read_exact(&mut byte).await?;
            if byte[0] == b'\n' {
                return Ok(value);
            }
            return Err(io::Error::other("invalid Redis line terminator"));
        }
        value.push(byte[0]);
    }
}

fn parse_len(value: &[u8]) -> Result<i64, io::Error> {
    std::str::from_utf8(value)
        .ok()
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| io::Error::other("invalid Redis length"))
}

fn expect_ok(response: Resp) -> Result<(), io::Error> {
    match response {
        Resp::Simple(value) if value == "OK" => Ok(()),
        Resp::Error(error) => Err(io::Error::other(error)),
        other => Err(io::Error::other(format!(
            "unexpected Redis status: {other:?}"
        ))),
    }
}

fn expect_queued(response: Resp) -> Result<(), io::Error> {
    match response {
        Resp::Simple(value) if value == "QUEUED" => Ok(()),
        Resp::Error(error) => Err(io::Error::other(error)),
        other => Err(io::Error::other(format!(
            "unexpected Redis transaction status: {other:?}"
        ))),
    }
}

fn pubsub_message_bytes(response: Resp) -> Option<Vec<u8>> {
    let Resp::Array(Some(values)) = response else {
        return None;
    };
    if values.len() != 3 {
        return None;
    }
    let kind = match &values[0] {
        Resp::Bulk(Some(value)) => value.as_slice(),
        _ => return None,
    };
    if kind != b"message" {
        return None;
    }
    match &values[2] {
        Resp::Bulk(Some(value)) => Some(value.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AUDIO_HISTORY_LIMIT, LiveFanout, TRANSLATED_HISTORY_LIMIT, apply_snapshot, empty_snapshot,
    };
    use crate::{
        config::LiveFanoutConfig,
        features::live::dto::{EdgeAudioReadyPayload, EdgeLiveMessage, EdgeSourceStatusPayload},
    };
    use tokio::{
        sync::{broadcast, watch},
        time::{Duration, sleep, timeout},
    };

    #[test]
    fn snapshot_keeps_bounded_audio_and_translation_histories() {
        let mut snapshot = empty_snapshot();
        for index in 0..(AUDIO_HISTORY_LIMIT + 4) {
            snapshot = apply_snapshot(
                snapshot,
                &EdgeLiveMessage::AudioReady {
                    payload: EdgeAudioReadyPayload {
                        event_id: format!("audio:{index}"),
                        server_time: "2026-01-01T00:00:00Z".to_owned(),
                        translation_id: index as i64,
                        news_event_id: index as i64,
                        timestamp: "2026-01-01T00:00:00Z".to_owned(),
                        stream_id: format!("s-{index}"),
                        playlist_url: "/audio/test.m3u8".to_owned(),
                        published_at: "2026-01-01T00:00:00Z".to_owned(),
                        target_duration_secs: 2,
                    },
                },
            );
        }
        assert_eq!(snapshot.audio_events.len(), AUDIO_HISTORY_LIMIT);
        assert_eq!(snapshot.translated_news_items.len(), 0);
        assert!(TRANSLATED_HISTORY_LIMIT > AUDIO_HISTORY_LIMIT);
    }

    #[tokio::test]
    async fn unavailable_redis_returns_an_error_without_preventing_local_delivery() {
        let config = LiveFanoutConfig {
            enabled: true,
            redis_url: Some("redis://127.0.0.1:1/0".to_owned()),
            redis_channel: "test:events".to_owned(),
            redis_snapshot_key: "test:snapshot".to_owned(),
        };
        let fanout = LiveFanout::from_config(&config)
            .expect("valid config")
            .expect("enabled");
        let (tx, mut rx) = broadcast::channel(1);
        let message = source_status_message("unavailable");
        let _ = tx.send(message.clone());
        assert!(fanout.publish_to_redis(&message).await.is_err());
        assert!(matches!(
            rx.recv().await,
            Ok(EdgeLiveMessage::SourceStatus { .. })
        ));
    }

    #[tokio::test]
    #[ignore = "requires NEWS_LIVE_TEST_REDIS_URL and a disposable Redis instance"]
    async fn redis_relays_one_replica_message_to_a_websocket_replica() {
        let redis_url = std::env::var("NEWS_LIVE_TEST_REDIS_URL").expect("test Redis URL");
        let config = LiveFanoutConfig {
            enabled: true,
            redis_url: Some(redis_url),
            redis_channel: "filfinance:test:fanout".to_owned(),
            redis_snapshot_key: "filfinance:test:fanout:snapshot".to_owned(),
        };
        let publisher = LiveFanout::from_config(&config)
            .expect("valid config")
            .expect("enabled");
        let subscriber = LiveFanout::from_config(&config)
            .expect("valid config")
            .expect("enabled");
        let (tx, mut rx) = broadcast::channel(8);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let handle = subscriber.spawn_subscriber(tx, shutdown_rx);
        sleep(Duration::from_millis(100)).await;
        publisher
            .publish_to_redis(&source_status_message("two-replicas"))
            .await
            .expect("Redis publish");
        let received = timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("relay timeout")
            .expect("relay message");
        assert!(matches!(received, EdgeLiveMessage::SourceStatus { .. }));
        let _ = shutdown_tx.send(true);
        let _ = handle.await;
    }

    fn source_status_message(event_id: &str) -> EdgeLiveMessage {
        EdgeLiveMessage::SourceStatus {
            payload: EdgeSourceStatusPayload {
                event_id: event_id.to_owned(),
                server_time: "2026-01-01T00:00:00Z".to_owned(),
                sources: Vec::new(),
                calendar_day_summary: None,
            },
        }
    }
}
