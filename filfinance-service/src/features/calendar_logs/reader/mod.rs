//! Adapted from local-logs-server 0.3.0: file protocol unchanged, Filfinance admin auth.
mod files;
mod management;
mod protocol;
use crate::{
    app_state::AppState,
    features::auth::{self, CurrentUser},
};
use axum::{
    Json,
    body::Body,
    extract::{
        Request, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, Method, header},
    response::{IntoResponse, Response},
};
use files::FileRoot;
use protocol::*;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::{
    io::AsyncReadExt,
    sync::{Semaphore, watch},
};
use tokio_util::io::ReaderStream;

struct Options {
    poll: Duration,
    ack_timeout: Duration,
    download_ticket_ttl: Duration,
}
struct Ack {
    end: String,
    sent: Instant,
}
struct Data {
    root: Option<FileRoot>,
    selection: Option<Selection>,
    ack: Option<Ack>,
    branches: BTreeMap<String, String>,
    unavailable: Option<String>,
    tree_at: Instant,
    job: Option<Arc<management::Job>>,
}
struct Ticket {
    key: String,
    root: String,
    file: String,
    identity: String,
    expires: Instant,
}
struct Session {
    user_id: String,
    data: Arc<tokio::sync::Mutex<Data>>,
    cancel: watch::Sender<u64>,
    connected: AtomicBool,
    touched: AtomicU64,
    closed: AtomicBool,
    ticket: Mutex<Option<Ticket>>,
    transfer: Arc<Semaphore>,
    root_epoch: AtomicU64,
}
pub(crate) struct Reader {
    options: Options,
    directory: PathBuf,
    connections: Arc<tokio::sync::Semaphore>,
    id: String,
    sessions: Mutex<HashMap<String, Arc<Session>>>,
    start: Instant,
    management: management::Hub,
    transfers: Arc<Semaphore>,
}
impl Reader {
    fn now(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
    fn session(&self, headers: &HeaderMap, user_id: &str) -> Result<Arc<Session>> {
        let token = headers
            .get("x-local-session")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let session = self
            .sessions
            .lock()
            .unwrap()
            .get(token)
            .cloned()
            .ok_or_else(|| {
                Error::new(
                    "SESSION_EXPIRED",
                    "Session locale expirée. Rouvrez le dossier.",
                    401,
                )
            })?;
        if session.user_id != user_id {
            return Err(Error::new("FORBIDDEN", "Session refusée.", 403));
        }
        session.touched.store(self.now(), Ordering::Relaxed);
        Ok(session)
    }
}

impl Reader {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            options: Options {
                poll: Duration::from_millis(250),
                ack_timeout: Duration::from_secs(10),
                download_ticket_ttl: Duration::from_secs(30),
            },
            connections: Arc::new(tokio::sync::Semaphore::new(16)),
            id: id(),
            sessions: Mutex::new(HashMap::new()),
            start: Instant::now(),
            management: management::Hub::default(),
            transfers: Arc::new(Semaphore::new(4)),
        }
    }
    fn expire_sessions(&self) {
        self.sessions.lock().unwrap().retain(|_, session| {
            let keep = session.connected.load(Ordering::Relaxed)
                || self
                    .now()
                    .saturating_sub(session.touched.load(Ordering::Relaxed))
                    < 120_000;
            if !keep {
                session.closed.store(true, Ordering::Relaxed);
                session.cancel.send_modify(|v| *v += 1);
                if let Ok(data) = session.data.try_lock()
                    && let Some(job) = &data.job
                {
                    job.kick(false);
                }
            }
            keep
        });
    }
}

pub(crate) async fn endpoint(State(state): State<AppState>, request: Request) -> Response {
    let Some(user) = request.extensions().get::<CurrentUser>().cloned() else {
        return Error::new("FORBIDDEN", "Authentification admin requise.", 403).into_response();
    };
    if !user.can_access_admin_routes() {
        return Error::new("FORBIDDEN", "Accès admin requis.", 403).into_response();
    }
    let app = state.calendar_logs.reader.clone();
    app.expire_sessions();
    let mut response = match handle(app, request, user).await {
        Ok(r) => r,
        Err(e) => e.into_response(),
    };
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    response
}

pub(crate) async fn websocket(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
    headers: HeaderMap,
) -> Response {
    let app = state.calendar_logs.reader.clone();
    let Ok(permit) = app.connections.clone().try_acquire_owned() else {
        return Error::new("LIMIT", "Trop de connexions.", 429).into_response();
    };
    let Some(protocol) = headers
        .get("sec-websocket-protocol")
        .and_then(|h| h.to_str().ok())
        .filter(|p| p.starts_with("local-logs.v1.") && p.len() < 160)
        .map(str::to_owned)
    else {
        return Error::new("FORBIDDEN", "Session du lecteur requise.", 403).into_response();
    };
    ws.protocols([protocol.clone()]).max_message_size(BODY).max_frame_size(BODY)
        .write_buffer_size(0).max_write_buffer_size(1024 * 1024)
        .on_upgrade(move |mut socket| async move {
            let _permit = permit;
            let Ok(user) = crate::features::calendar_day::authenticate_calendar_day_socket(&mut socket, &state).await else { return; };
            if !user.can_access_admin_routes() {
                let _ = socket.send(Message::Close(Some(axum::extract::ws::CloseFrame {
                    code: 1008,
                    reason: "admin role required".into(),
                }))).await;
                return;
            }
            let session = app.sessions.lock().unwrap().get(&protocol[14..]).cloned();
            let Some(session) = session.filter(|s| s.user_id == user.id && !s.closed.load(Ordering::Relaxed)) else {
                let _ = socket.send(Message::Close(Some(axum::extract::ws::CloseFrame { code: 1008, reason: "session expired".into() }))).await;
                return;
            };
            if session.connected.swap(true, Ordering::SeqCst) {
                let _ = socket.send(Message::Close(Some(axum::extract::ws::CloseFrame { code: 1013, reason: "reader already connected".into() }))).await;
                return;
            }
            let mut stop = state.shutdown_rx.clone();
            if !*stop.borrow() {
                let expires_at = user.access_token_expires_at;
                let completed = auth::complete_before_access_token_expiration(expires_at, async {
                    tokio::select! { _ = stop.changed() => {}, _ = live(app.clone(), session.clone(), &mut socket) => {} }
                }).await;
                if completed.is_none() { let _ = socket.send(auth::access_token_expired_close_message()).await; }
            }
            session.connected.store(false, Ordering::SeqCst);
            session.touched.store(app.now(), Ordering::Relaxed);
        }).into_response()
}
fn root<'a>(data: &'a mut Data, id: &str) -> Result<&'a mut FileRoot> {
    data.root
        .as_mut()
        .filter(|r| r.id == id)
        .ok_or_else(|| Error::new("ROOT_CHANGED", "La racine active a changé.", 409))
}
async fn disk<T: Send + 'static>(
    session: Arc<Session>,
    f: impl FnOnce(&mut Data) -> Result<T> + Send + 'static,
) -> Result<T> {
    if session.closed.load(Ordering::Relaxed) {
        return Err(Error::new(
            "SESSION_EXPIRED",
            "Session locale expirée.",
            401,
        ));
    }
    let mut data = session
        .data
        .clone()
        .try_lock_owned()
        .map_err(|_| Error::new("LIMIT", "Lecture en cours, réessayez.", 429))?;
    tokio::task::spawn_blocking(move || f(&mut data))
        .await
        .map_err(|_| Error::new("INTERNAL", "Lecture interrompue.", 500))?
}
async fn download_disk<T: Send + 'static>(
    session: Arc<Session>,
    operation: impl FnOnce(&mut Data) -> Result<T> + Send + 'static,
) -> Result<T> {
    let mut data = tokio::time::timeout(Duration::from_secs(2), session.data.clone().lock_owned())
        .await
        .map_err(|_| {
            Error::new(
                "LIMIT",
                "Lecture indisponible, préparez un nouveau téléchargement.",
                429,
            )
        })?;
    if session.closed.load(Ordering::Relaxed) {
        return Err(Error::new(
            "SESSION_EXPIRED",
            "Session locale expirée.",
            401,
        ));
    }
    tokio::task::spawn_blocking(move || operation(&mut data))
        .await
        .map_err(|_| Error::new("INTERNAL", "Lecture interrompue.", 500))?
}
async fn handle(app: Arc<Reader>, request: Request, user: CurrentUser) -> Result<Response> {
    let method = request.method().clone();
    let pathname = request
        .uri()
        .path()
        .replacen("/api/admin/calendar-day/logs", "/api/v1", 1);
    if pathname == "/api/v1/health" && method == Method::GET {
        return Ok(Json(json!({"version":1,"serverId":app.id})).into_response());
    }
    if pathname == "/api/v1/session" && method == Method::POST {
        let mut sessions = app.sessions.lock().unwrap();
        if sessions.len() >= 4 {
            return Err(Error::new(
                "LIMIT",
                "Quatre lecteurs sont déjà ouverts.",
                429,
            ));
        }
        let token = format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        let (cancel, _) = watch::channel(0);
        sessions.insert(
            token.clone(),
            Arc::new(Session {
                user_id: user.id.clone(),
                data: Arc::new(tokio::sync::Mutex::new(Data {
                    root: None,
                    selection: None,
                    ack: None,
                    branches: BTreeMap::new(),
                    unavailable: None,
                    tree_at: Instant::now() - Duration::from_secs(1),
                    job: None,
                })),
                cancel,
                connected: AtomicBool::new(false),
                touched: AtomicU64::new(app.now()),
                closed: AtomicBool::new(false),
                ticket: Mutex::new(None),
                transfer: Arc::new(Semaphore::new(1)),
                root_epoch: AtomicU64::new(0),
            }),
        );
        return Ok(Json(json!({"version":1,"serverId":app.id,"token":token})).into_response());
    }
    if pathname == "/api/v1/session" {
        let session = app.session(request.headers(), &user.id)?;
        if method == Method::GET {
            return Ok(Json(json!({"version":1,"serverId":app.id})).into_response());
        }
        if method == Method::DELETE {
            session.closed.store(true, Ordering::Relaxed);
            session.cancel.send_modify(|v| *v += 1);
            session.ticket.lock().unwrap().take();
            session.root_epoch.fetch_add(1, Ordering::Relaxed);
            app.sessions
                .lock()
                .unwrap()
                .remove(request.headers()["x-local-session"].to_str().unwrap());
            return Ok(Json(json!({"closed":true})).into_response());
        }
    }
    if method == Method::GET
        && let Some(key) = pathname.strip_prefix("/api/v1/downloads/")
    {
        return download(&app, key, &user.id).await;
    }
    if pathname.starts_with("/api/") {
        let session = app.session(request.headers(), &user.id)?;
        let query: HashMap<String, String> =
            url::form_urlencoded::parse(request.uri().query().unwrap_or("").as_bytes())
                .into_owned()
                .collect();
        if pathname == "/api/v1/roots" && method == Method::POST {
            let path = app.directory.clone();
            let manager = app.clone();
            let current = session.clone();
            return Ok(Json(
                disk(session, move |d| {
                    let absolute = std::fs::canonicalize(&path)?;
                    let opened = FileRoot::create(&absolute.to_string_lossy())?;
                    let dto = opened.dto();
                    let job = manager.management.get(opened.absolute().to_owned());
                    job.kick(false);
                    d.root = Some(opened);
                    d.job = Some(job);
                    current.ticket.lock().unwrap().take();
                    current.root_epoch.fetch_add(1, Ordering::Relaxed);
                    d.selection = None;
                    d.ack = None;
                    d.unavailable = None;
                    d.branches.clear();
                    Ok(dto)
                })
                .await?,
            )
            .into_response());
        }
        let parts: Vec<_> = pathname.trim_start_matches('/').split('/').collect();
        if parts.len() >= 4 && parts[..3] == ["api", "v1", "roots"] {
            let root_id = parts[3].to_owned();
            if parts.len() == 4 && method == Method::DELETE {
                let current = session.clone();
                return Ok(Json(
                    disk(session, move |d| {
                        root(d, &root_id)?;
                        d.root = None;
                        d.job = None;
                        current.ticket.lock().unwrap().take();
                        current.root_epoch.fetch_add(1, Ordering::Relaxed);
                        d.selection = None;
                        d.ack = None;
                        d.branches.clear();
                        Ok(json!({"closed":true}))
                    })
                    .await?,
                )
                .into_response());
            }
            if parts.len() == 5 && parts[4] == "statistics" && method == Method::GET {
                return Ok(Json(
                    disk(session, move |d| {
                        root(d, &root_id)?;
                        let job = d.job.as_ref().ok_or_else(|| {
                            Error::new("ROOT_CHANGED", "Racine de gestion indisponible.", 409)
                        })?;
                        job.kick(query.get("refresh").is_some_and(|value| value == "1"));
                        Ok(job.snapshot())
                    })
                    .await?,
                )
                .into_response());
            }
            if parts.len() >= 6 && parts[4] == "files" {
                let file_id = parts[5].to_owned();
                if parts.len() == 6 && method == Method::GET {
                    return Ok(Json(
                        disk(session, move |d| {
                            let root = root(d, &root_id)?;
                            root.relocate(&file_id)?;
                            root.entry(&file_id)
                        })
                        .await?,
                    )
                    .into_response());
                }
                if (method == Method::DELETE && parts.len() == 6)
                    || (method == Method::POST && parts.len() == 7 && parts[6] == "download")
                {
                    let expected = request
                        .headers()
                        .get("if-match")
                        .and_then(|value| value.to_str().ok())
                        .ok_or_else(|| {
                            Error::new("PRECONDITION_REQUIRED", "Identité du fichier requise.", 428)
                        })?
                        .to_owned();
                    let deleting = method == Method::DELETE;
                    let current = session.clone();
                    let ticket_ttl = app.options.download_ticket_ttl.min(Duration::from_secs(30));
                    let value = disk(session, move |d| {
                        let root = root(d, &root_id)?;
                        let relative = root.management_path(&file_id)?;
                        let (_, eligible, reason) =
                            management::file_state(root.absolute(), &relative, management::now());
                        if !eligible {
                            return Err(Error::new("FILE_PROTECTED", reason, 409));
                        }
                        let _gate = ordered_local_logger::storage::gate(root.absolute())?;
                        let file = management::action_file(
                            root.absolute(),
                            &relative,
                            &expected,
                            deleting,
                        )?;
                        if deleting {
                            ordered_local_logger::storage::remove_beneath(
                                root.absolute(),
                                &relative,
                                false,
                                Some(&expected),
                            )?;
                            drop(file);
                            if d.selection.as_ref().is_some_and(|selection| selection.file_id == file_id) {
                                d.selection = None;
                                d.ack = None;
                            }
                            if let Some(job) = &d.job {
                                job.kick(true);
                            }
                            Ok(json!({"deleted": true}))
                        } else {
                            drop(file);
                            if current.transfer.available_permits() == 0 {
                                return Err(Error::new(
                                    "FILE_BUSY",
                                    "Téléchargement en cours.",
                                    409,
                                ));
                            }
                            let key = format!(
                                "{}{}",
                                uuid::Uuid::new_v4().simple(),
                                uuid::Uuid::new_v4().simple()
                            );
                            *current.ticket.lock().unwrap() = Some(Ticket {
                                key: key.clone(),
                                root: root_id,
                                file: file_id,
                                identity: expected,
                                expires: Instant::now() + ticket_ttl,
                            });
                            Ok(json!({"url": format!("/api/admin/calendar-day/logs/downloads/{key}")}))
                        }
                    })
                    .await?;
                    return Ok(Json(value).into_response());
                }
            }
            if parts.len() == 5 && parts[4] == "entries" && method == Method::GET {
                return Ok(Json(
                    disk(session, move |d| {
                        let r = root(d, &root_id)?;
                        let parent = query
                            .get("parentId")
                            .cloned()
                            .unwrap_or_else(|| r.node_id.clone());
                        r.entries(&parent, query.get("cursor").map(String::as_str))
                    })
                    .await?,
                )
                .into_response());
            }
            if parts.len() == 7
                && parts[4] == "files"
                && parts[6] == "content"
                && method == Method::GET
            {
                let file_id = parts[5].to_owned();
                return Ok(Json(
                    disk(session, move |d| {
                        root(d, &root_id)?.content(
                            &file_id,
                            ContentOptions {
                                generation: query.get("generation").cloned(),
                                before: query.get("before").cloned(),
                                after: query.get("after").cloned(),
                                limit: None,
                            },
                        )
                    })
                    .await?,
                )
                .into_response());
            }
        }
        return Err(Error::new("NOT_FOUND", "Opération inconnue.", 404));
    }
    Err(Error::new("NOT_FOUND", "Opération inconnue.", 404))
}
async fn download(app: &Arc<Reader>, key: &str, user_id: &str) -> Result<Response> {
    let sessions: Vec<_> = app.sessions.lock().unwrap().values().cloned().collect();
    for session in sessions {
        if session.user_id != user_id {
            continue;
        }
        let ticket = {
            let mut ticket = session.ticket.lock().unwrap();
            if ticket.as_ref().is_some_and(|entry| entry.key == key) {
                ticket.take()
            } else {
                None
            }
        };
        let Some(ticket) = ticket else {
            continue;
        };
        if ticket.expires <= Instant::now() || session.closed.load(Ordering::Relaxed) {
            break;
        }
        let local = session
            .transfer
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::new("FILE_BUSY", "Téléchargement en cours.", 409))?;
        let global = app
            .transfers
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::new("LIMIT", "Trop de téléchargements.", 429))?;
        let current = session.clone();
        let (file, name, epoch) = download_disk(session.clone(), move |data| {
            if ticket.expires <= Instant::now() {
                return Err(Error::new(
                    "DOWNLOAD_EXPIRED",
                    "Lien de téléchargement expiré.",
                    410,
                ));
            }
            let root = root(data, &ticket.root)?;
            let path = root.management_path(&ticket.file)?;
            let _gate = ordered_local_logger::storage::gate(root.absolute())?;
            let file = management::action_file(root.absolute(), &path, &ticket.identity, false)?;
            Ok((
                file,
                path.file_name().unwrap().to_string_lossy().into_owned(),
                current.root_epoch.load(Ordering::Relaxed),
            ))
        })
        .await?;
        let length = file.metadata()?.len();
        let (mut output, transfer) = tokio::io::duplex(CHUNK);
        let cancelled = session.clone();
        tokio::spawn(async move {
            let _permits = (local, global);
            let mut source = tokio::fs::File::from_std(file);
            source.set_max_buf_size(CHUNK);
            let mut source = source.take(length);
            let cancellation = async {
                loop {
                    if cancelled.closed.load(Ordering::Relaxed)
                        || cancelled.root_epoch.load(Ordering::Relaxed) != epoch
                    {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            };
            tokio::select! {
                _ = tokio::io::copy(&mut source, &mut output) => {},
                _ = cancellation => {},
            }
        });
        let mut response =
            Body::from_stream(ReaderStream::with_capacity(transfer, CHUNK)).into_response();
        let headers = response.headers_mut();
        headers.insert(
            header::CONTENT_TYPE,
            "application/octet-stream".parse().unwrap(),
        );
        headers.insert(header::CONTENT_LENGTH, length.to_string().parse().unwrap());
        headers.insert(
            header::CONTENT_DISPOSITION,
            format!(
                "attachment; filename=\"log.log\"; filename*=UTF-8''{}",
                percent_encoding::utf8_percent_encode(&name, percent_encoding::NON_ALPHANUMERIC)
            )
            .parse()
            .unwrap(),
        );
        headers.insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
        headers.insert("referrer-policy", "no-referrer".parse().unwrap());
        return Ok(response);
    }
    Err(Error::new(
        "DOWNLOAD_EXPIRED",
        "Lien de téléchargement expiré ou déjà utilisé.",
        410,
    ))
}
async fn send(socket: &mut WebSocket, message: Value) -> bool {
    tokio::time::timeout(
        Duration::from_secs(2),
        socket.send(Message::Text(message.to_string().into())),
    )
    .await
    .is_ok_and(|r| r.is_ok())
}
async fn live(app: Arc<Reader>, session: Arc<Session>, socket: &mut WebSocket) {
    let mut cancel = session.cancel.subscribe();
    let generation = {
        let mut data = session.data.lock().await;
        data.selection = None;
        data.ack = None;
        data.unavailable = None;
        session.cancel.send_modify(|v| *v += 1);
        *session.cancel.borrow()
    };
    cancel.borrow_and_update();
    let mut tick = tokio::time::interval(app.options.poll);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut heartbeat = Instant::now() - Duration::from_secs(5);
    loop {
        if session.closed.load(Ordering::Relaxed) || *cancel.borrow() != generation {
            break;
        }
        session.touched.store(app.now(), Ordering::Relaxed);
        let message = tokio::select! {_=cancel.changed()=>break,_=tick.tick()=>None,message=socket.recv()=>match message{Some(Ok(Message::Text(text)))=>Some(text),Some(Ok(Message::Ping(bytes)))=>{if socket.send(Message::Pong(bytes)).await.is_err(){break;}continue;},Some(Ok(Message::Pong(_)))=>continue,_=>break}};
        if let Some(message) = message {
            let parsed = serde_json::from_str::<ClientMessage>(&message)
                .map_err(|_| Error::new("INVALID_REQUEST", "Message invalide.", 400));
            let result = match parsed {
                Ok(msg) => apply(&session, msg).await,
                Err(e) => Err(e),
            };
            let reply = match result {
                Ok(value) => value,
                Err(e) => Some(json!({"type":"error","code":e.code,"message":e.message})),
            };
            if let Some(value) = reply
                && !send(socket, value).await
            {
                break;
            }
        }
        if heartbeat.elapsed() >= Duration::from_secs(5) {
            if !send(socket, json!({"type":"heartbeat"})).await {
                break;
            }
            heartbeat = Instant::now();
        }
        let timeout = app.options.ack_timeout;
        match disk(session.clone(), move |d| poll(d, timeout)).await {
            Ok(messages) => {
                let mut close = false;
                for message in messages {
                    close |= message["type"] == "resync-required";
                    if !send(socket, message).await {
                        close = true;
                        break;
                    }
                }
                if close {
                    break;
                }
            }
            Err(e) if e.status == 429 => {}
            Err(e) => {
                if !send(
                    socket,
                    json!({"type":"error","code":e.code,"message":e.message}),
                )
                .await
                {
                    break;
                }
            }
        }
    }
    if *session.cancel.borrow() == generation {
        session.touched.store(app.now(), Ordering::Relaxed);
        let mut data = session.data.lock().await;
        data.selection = None;
        data.ack = None;
        data.branches.clear();
    }
    let _ = tokio::time::timeout(
        Duration::from_millis(200),
        socket.send(Message::Close(None)),
    )
    .await;
}
async fn apply(session: &Session, message: ClientMessage) -> Result<Option<Value>> {
    let mut data = session.data.lock().await;
    match message {
        ClientMessage::Subscribe { selection } => {
            root(&mut data, &selection.root_id)?.check_node(&selection.file_id, Kind::File)?;
            offset(&selection.offset)?;
            if selection.revision > MAX_OFFSET || selection.generation.len() > 80 {
                return Err(Error::new("INVALID_REQUEST", "Sélection invalide.", 400));
            }
            let message = json!({"type":"subscribed","revision":selection.revision,"generation":selection.generation,"offset":selection.offset});
            data.selection = Some(selection);
            data.ack = None;
            data.unavailable = None;
            Ok(Some(message))
        }
        ClientMessage::Unsubscribe => {
            data.selection = None;
            data.ack = None;
            Ok(None)
        }
        ClientMessage::Ack {
            revision,
            generation,
            end,
        } => {
            if data
                .selection
                .as_ref()
                .is_some_and(|s| s.revision == revision && s.generation == generation)
                && data.ack.as_ref().is_some_and(|a| a.end == end)
            {
                data.selection.as_mut().unwrap().offset = end;
                data.ack = None;
                Ok(None)
            } else {
                Err(Error::new("CURSOR_INVALID", "Acquittement périmé.", 409))
            }
        }
        ClientMessage::Branches { root_id, ids } => {
            if ids.len() > BRANCHES {
                return Err(Error::new("LIMIT", "Trop de dossiers ouverts.", 400));
            }
            let root = root(&mut data, &root_id)?;
            for id in &ids {
                root.check_node(id, Kind::Directory)?;
            }
            data.branches = ids
                .into_iter()
                .map(|id| {
                    let rev = data.branches.get(&id).cloned().unwrap_or_default();
                    (id, rev)
                })
                .collect();
            Ok(None)
        }
    }
}
fn poll(data: &mut Data, ack_timeout: Duration) -> Result<Vec<Value>> {
    let mut messages = Vec::new();
    if data
        .ack
        .as_ref()
        .is_some_and(|ack| ack.sent.elapsed() > ack_timeout)
    {
        return Ok(vec![
            json!({"type":"resync-required","revision":data.selection.as_ref().map(|s|s.revision).unwrap_or(0),"reason":"Client lent : reprise requise."}),
        ]);
    }
    if let Some(selected) = data.selection.clone().filter(|_| data.ack.is_none()) {
        let renamed = {
            let reader = root(data, &selected.root_id)?;
            if reader.relocate(&selected.file_id)? {
                Some(reader.entry(&selected.file_id)?)
            } else {
                None
            }
        };
        if let Some(entry) = renamed {
            messages.push(json!({
                "type": "file-renamed",
                "revision": selected.revision,
                "entry": entry,
            }));
        }
        let result = root(data, &selected.root_id)?.content(
            &selected.file_id,
            ContentOptions {
                generation: Some(selected.generation.clone()),
                after: Some(selected.offset.clone()),
                limit: Some(CHUNK),
                ..Default::default()
            },
        );
        match result {
            Ok(chunk) => {
                if data.unavailable.take().is_some() {
                    messages.push(json!({"type":"subscribed","revision":selected.revision,"generation":selected.generation,"offset":selected.offset}));
                }
                if chunk.end != chunk.start {
                    data.ack = Some(Ack {
                        end: chunk.end.clone(),
                        sent: Instant::now(),
                    });
                    let mut value = serde_json::to_value(chunk).unwrap();
                    value["type"] = json!("chunk");
                    value["revision"] = json!(selected.revision);
                    messages.push(value);
                }
            }
            Err(e) if e.code == "GENERATION_CHANGED" || e.code == "CURSOR_INVALID" => {
                data.selection = None;
                data.ack = None;
                messages
                    .push(json!({"type":"reset","revision":selected.revision,"reason":e.message}));
            }
            Err(e) => {
                if e.code != "NOT_FOUND" && e.code != "PERMISSION" {
                    data.selection = None;
                }
                let key = format!("{}:{}", selected.revision, e.code);
                if data.unavailable.as_deref() != Some(&key) {
                    messages.push(json!({"type":"file-unavailable","revision":selected.revision,"reason":e.message}));
                }
                data.unavailable = Some(key);
            }
        }
    }
    if data.tree_at.elapsed() >= Duration::from_secs(1) {
        data.tree_at = Instant::now();
        if let Some(root) = &data.root {
            let mut changed = vec![];
            let mut removed = vec![];
            for (id, prior) in &mut data.branches {
                match root.directory_revision(id) {
                    Ok(rev) => {
                        if *prior != rev {
                            changed.push(id.clone());
                        }
                        *prior = rev;
                    }
                    Err(_) => {
                        changed.push(id.clone());
                        removed.push(id.clone());
                    }
                }
            }
            for id in removed {
                data.branches.remove(&id);
            }
            if !changed.is_empty() {
                messages.push(json!({"type":"tree-changed","ids":changed}));
            }
        }
    }
    Ok(messages)
}

#[cfg(test)]
mod tests;
