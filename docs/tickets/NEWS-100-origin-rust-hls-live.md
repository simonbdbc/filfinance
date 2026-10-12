# NEWS-100 - Rust origin HLS live

## Objectif

Produire un premier flux HLS live depuis le backend Rust, en gardant `/ws/audio` comme fallback transitoire.

## Implementation

Fichiers principaux:

- `filfinance-service/src/hls.rs`
- `filfinance-service/src/config.rs`
- `filfinance-service/src/main.rs`
- `filfinance-service/src/state.rs`
- `filfinance-service/src/tts.rs`

Le worker TTS continue de produire un WAV interne via Piper. Quand `NEWS_HLS_ENABLED=true`, ce WAV est encode par `ffmpeg` en segments AAC-LC/MPEG-TS `.ts`.

Le backend sert ensuite:

```http
GET /audio/live/playlist.m3u8
GET /audio/live/segments/{sequence}.ts
GET /audio/replay/today/playlist.m3u8
GET /audio/replay/today/segments/{local_day}/{sequence}.ts
```

Le chemin `/ws/audio` reste disponible pendant la migration client.

## Comportement runtime

- HLS est desactive par defaut.
- Si HLS est desactive, le worker TTS conserve le comportement historique.
- Si HLS est active, la publication HLS est tentee avant l'envoi WebSocket audio.
- Une erreur HLS est loguee et tracee, mais ne bloque pas le pipeline news/traduction.
- La playlist live expose une fenetre glissante.
- Les segments sont nommes par sequence monotone.
- Les anciens dossiers de jour sont supprimes en best-effort pour respecter l'absence d'historique replay.

## Variables

Variables ajoutees dans `filfinance-service/.env.example` et `.env.server.example`:

```env
NEWS_HLS_ENABLED=false
NEWS_HLS_SEGMENTS_DIR=data/hls
NEWS_HLS_FFMPEG_COMMAND=ffmpeg
NEWS_HLS_LIVE_SEGMENT_TARGET_DURATION_SECS=4
NEWS_HLS_LIVE_WINDOW_SEGMENTS=6
```

Sur le serveur Docker:

```env
NEWS_HLS_SEGMENTS_DIR=/app/data/hls
```

`NEWS_HLS_ENABLED=true` exige aussi:

```env
NEWS_TTS_ENABLED=true
NEWS_HLS_FFMPEG_COMMAND=ffmpeg
```

Le Dockerfile installe maintenant `ffmpeg` dans l'image runtime.

## Headers HTTP

Playlist live:

```http
Content-Type: application/vnd.apple.mpegurl
Cache-Control: public, max-age=1, must-revalidate
Cloudflare-CDN-Cache-Control: max-age=1
X-Content-Type-Options: nosniff
```

Segments live:

```http
Content-Type: video/MP2T
Cache-Control: public, max-age=86400, immutable
Cloudflare-CDN-Cache-Control: max-age=86400
X-Content-Type-Options: nosniff
```

## Validation

Validation automatique:

```bash
cd filfinance-service
cargo test
```

Resultat local:

```text
91 passed
```

Validation live a faire apres activation Piper/HLS:

```bash
curl -fsS http://127.0.0.1:8088/audio/live/playlist.m3u8
curl -I http://127.0.0.1:8088/audio/live/segments/1.ts
```

Puis verifier avec un player HLS web apres emission d'au moins une news TTS.

## Limites connues

- Le replay HLS du jour est ajoute par `NEWS-102`.
- Le client Web utilise encore `/ws/audio` en mode nominal jusqu'a `NEWS-103`.
- Le cache CDN Cloudflare est durci par `NEWS-101`, mais le cache hit ratio reste a mesurer dans `NEWS-104`.
- La validation HLS reelle depend de Piper actif et de `ffmpeg` disponible dans l'environnement cible.
