# Fil Finance Service

Module autonome en Rust `filfinance-service` pour l'ingestion ultra low-latency de news et d'evenements de marche.

Le service est volontairement decouple de `deltalytix-main` et `deltalytix-api-main`. Il expose un flux temps reel en WebSocket, persiste l'historique et peut relayer les news vers Telegram.

Il expose aussi un historique traduit EN+FR separe entre feed reel et `test_run`, pour alimenter le client journalier et le rejeu audio.

## Cible

- polling concurrent de plusieurs sources
- normalisation et deduplication des headlines
- stockage local des evenements
- diffusion temps reel via WebSocket
- fanout Telegram

## Architecture

```text
collectors (FinancialJuice site, calendrier, navigateur fallback, X)
        |
        v
bounded ingest queue
        |
        v
dedup hot path
        |
        +--> SQLite WAL
        |
        +--> WebSocket broadcast
        |
        +--> Telegram relay
```

## Stack

- `tokio` pour le runtime async
- `axum` pour l'API et le WebSocket
- `reqwest` pour les fetchs HTTP et la session cookie-store
- `sqlx` pour la persistence SQLite du pipeline, des profils et des reglages runtime
- `tokio-tungstenite` pour le live SignalR WebSocket
- `tracing` pour l'observabilite

Les crates `quick-xml` et `scraper` servent maintenant a parser les enveloppes XML `.asmx` et le bootstrap HTML `/home` de FinancialJuice.

## Structure

```text
filfinance-service/
  Cargo.toml
  Dockerfile
  src/
    main.rs
    app_state.rs
    config.rs
    errors.rs
    telemetry.rs
    features/
      mod.rs
      router.rs
      workers.rs
      health/
      auth/
      profiles/
      news/
      calendar/
      ingestion/
      translation/
      audio/
      live/
      notifications/
    shared/
      auth_context.rs
      pagination.rs
      text.rs
      time.rs
```

Le backend Rust est organise en feature-first dans un seul crate. Chaque feature porte ses
handlers HTTP/WebSocket, workers, adapters et futurs ports locaux; `shared/` reste limite aux
types vraiment transverses.

`errors.rs` reste volontairement a la racine du crate: `AppError` est une erreur applicative
globale utilisee par le bootstrap, la configuration, les workers et les adapters HTTP. Elle ne vit
plus dans `shared/`, pour que `shared/` ne devienne pas une zone fourre-tout.

## Refonte backend feature-first

Etat au 2026-05-06: la refonte feature-first du backend Rust est cloturee dans le crate
`filfinance-service`, sans passage en Cargo workspace et sans changement fonctionnel volontaire.

Ce qui a ete fait:

- `main.rs` est recentre sur le bootstrap: chargement `.env`, configuration, stores, channels,
  workers, routeur Axum et graceful shutdown
- `app_state.rs`, `config.rs`, `telemetry.rs`, `errors.rs`, `features/` et `shared/` portent les
  frontieres racine du crate
- `features::router(...)` compose les routes HTTP par feature, et `features::spawn_workers(...)`
  centralise le lancement des workers
- les anciennes responsabilites racine ont ete redistribuees dans les features `health`, `auth`,
  `profiles`, `news`, `calendar_day`, `ingestion`, `translation`, `audio`, `live` et `notifications`
- les stores SQLite, DTO HTTP, DTO WebSocket, adapters HLS/TTS/traduction, collecteurs et
  workers sont maintenant proches de leur feature
- `shared/` est limite a `auth_context`, `pagination`, `text` et `time`

Contrats conserves pendant la refonte:

- memes endpoints HTTP: `/health`, `/api/news`, `/api/translations`, `/api/me`
- health admin detaille: `/api/admin/health`, reserve au role `admin`
- memes WebSockets backend: `/ws/news`, `/ws/test-runs`
- memes variables `NEWS_*`
- memes tables SQLite et migrations compatibles avec les bases existantes
- memes payloads JSON publics pour les flux live, historique, calendrier du jour, profils, traduction,
  audio/HLS et Telegram
- meme contrat `/api/me`, avec profils et preferences locaux

Etat de validation au moment de la cloture:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

La suite de tests Rust compte 119 tests unitaires/integration dans le crate.

Points explicitement restants hors refonte:

- l'extraction en workspace Cargo est repoussee tant que les frontieres de crate ne sont pas un
  besoin operationnel
- la suppression des chemins legacy de compatibilite ne doit se faire que dans un ticket dedie avec
  validation produit, car certains contrats existent pour lire d'anciennes donnees persistantes

## Documentation

- feuille de route partagee: [../docs/ROADMAP.md](../docs/ROADMAP.md)
- cadrage audio/traduction: [../docs/TOPIC-001-flux-audio-francais-news.md](../docs/TOPIC-001-flux-audio-francais-news.md)
- refonte acquisition FinancialJuice: ../docs/epics/EPIC-011-refonte-acquisition-financialjuice.md (référence interne non exportée)
- niveau FinancialJuice des news: ../docs/epics/EPIC-015-financialjuice-news-level.md (référence interne non exportée)
- auth Better Auth + PostgreSQL: ../docs/MIGRATION-BETTER-AUTH-POSTGRES.md (référence interne non exportée)
- runbook auth: [../docs/AUTH-RUNBOOK.md](../docs/AUTH-RUNBOOK.md)
- standard de commentaires Rust pedagogiques: [../docs/RUST-COMMENTING-STANDARD.md](../docs/RUST-COMMENTING-STANDARD.md)
- checklist de review Rust pedagogique: [../docs/RUST-REVIEW-CHECKLIST.md](../docs/RUST-REVIEW-CHECKLIST.md)
- audit de couverture Rust: ../docs/tickets/NEWS-047-audit-couverture-rust.md (référence interne non exportée)
- option workspace audio/traduction non prioritaire: ../docs/tickets/NEWS-153-option-workspace-audio-translation.md (référence interne non exportée)

## Standard de developpement Rust

Le backend `filfinance-service` suit maintenant une regle projet explicite: tout code Rust doit etre commente en francais de facon pedagogique, ligne par ligne, pour rester lisible par des developpeurs qui viennent surtout de `JavaScript`, `PHP` et `Java`.

Point de vocabulaire a conserver:

- pour les narrations de news, le prefixe de reference est maintenant `HH:MM. Nouvelle information` ou `HH:MM. Information importante` en heure locale de la machine
- pour les textes de warmup hors news, utiliser un texte neutre du type `Test audio`

Ce standard est formalise dans [../docs/RUST-COMMENTING-STANDARD.md](../docs/RUST-COMMENTING-STANDARD.md) et s'applique aux futurs tickets Rust.

## Garde-fous de review Rust

Les futurs changements Rust dans `filfinance-service` ne doivent pas etre clos ni consideres comme `done` sans satisfaire la checklist [../docs/RUST-REVIEW-CHECKLIST.md](../docs/RUST-REVIEW-CHECKLIST.md).

En pratique, cela veut dire au minimum:

- commentaires pedagogiques en francais sur toute ligne Rust ajoutee ou modifiee
- explication des mots-cles et specificites Rust introduits
- explication des choix d'architecture locaux
- meme exigence sur les tests Rust
- mention explicite de `cargo check` et `cargo test` dans le close-out du ticket

## Traduction locale OPUS-MT

Le worker de traduction supporte maintenant trois modes:

- `NEWS_TRANSLATION_PROVIDER=heuristic_fr`
- `NEWS_TRANSLATION_PROVIDER=local_command` ou `opus_mt_local_command`
- `NEWS_TRANSLATION_PROVIDER=local_command_session` ou `opus_mt_local_session`

Le mode `local_command` permet de brancher un runtime local `OPUS-MT` sans coupler le service Rust a un framework Python ou ML specifique. Dans ce workspace, le checkpoint retenu est `OPUS-MT tc-big en-fr`. Le service appelle une commande locale configuree via:

```env
NEWS_TRANSLATION_ENABLED=true
NEWS_TRANSLATION_SOURCE_LANGUAGE=en
NEWS_TRANSLATION_TARGET_LANGUAGE=fr
NEWS_TRANSLATION_PROVIDER=opus_mt_local_session
NEWS_TRANSLATION_PROFILE=quality
NEWS_TRANSLATION_COMMAND=python3
NEWS_TRANSLATION_COMMAND_ARGS=/Users/simon/Projets/PlateformeDev/dev-trade-analyse/filfinance/filfinance-service/scripts/opus_mt_local_command.py,--ct2-model-dir,/absolute/path/to/opus-mt-tc-big-en-fr-ct2-int8,--sentencepiece-model,/absolute/path/to/source.tokenizer.model,--target-sentencepiece-model,/absolute/path/to/target.tokenizer.model,--device,cpu,--compute-type,int8,--translation-mode,opus_mt_tc_big_local_session
NEWS_TRANSLATION_COMMAND_TIMEOUT_MS=5000
NEWS_TRANSLATION_WARMUP_ENABLED=true
NEWS_TRANSLATION_WARMUP_SOURCE_TEXT="Federal Reserve signals rates stay higher for longer"
```

Preparation minimale du modele local:

```bash
cd /Users/simon/Projets/PlateformeDev/dev-trade-analyse/filfinance/filfinance-service
python3 -m pip install -r scripts/requirements-opus-mt.txt
ct2-opus-mt-converter --model_dir /absolute/path/to/original-opus-mt-tc-big-en-fr --output_dir /absolute/path/to/opus-mt-tc-big-en-fr-ct2-int8 --quantization int8
```

Le helper fourni dans [scripts/opus_mt_local_command.py](scripts/opus_mt_local_command.py) consomme ensuite le modele converti CTranslate2 et les tokenizers SentencePiece associes. En mode `*_session`, le service ajoute automatiquement `--session` et garde le modele charge en memoire entre plusieurs traductions.

Contrat attendu de la commande locale:

- entree JSON sur `stdin`
- sortie JSON sur `stdout`
- code de retour non nul en cas d'echec

Payload envoye:

```json
{
  "source_language": "en",
  "target_language": "fr",
  "title": "Federal Reserve signals rates stay higher for longer",
  "body": null
}
```

Payload attendu en retour:

```json
{
  "translated_title": "La Reserve federale signale que les taux resteront plus eleves plus longtemps",
  "translated_body": null,
  "translation_mode": "opus_mt_local_command"
}
```

Le service protege automatiquement les tokens finance sensibles avant traduction, par exemple les tickers, les paires FX, les pourcentages et les valeurs numeriques. Si la commande locale echoue, le pipeline degrade proprement vers le fallback `heuristic_fr` sans bloquer Telegram ni le WebSocket.

Point important:

- `local_command` relance la commande a chaque traduction
- `local_command_session` garde un helper long-vivant et evite de recharger le modele a chaque news
- `NEWS_TRANSLATION_PROFILE=fast|balanced|quality` pilote le `--beam-size` effectif `1|2|4`
- `NEWS_TRANSLATION_COMMAND_ARGS` ne doit pas contenir `--beam-size`; le profil est la seule source de verite pour ce parametre

Le mode session est maintenant le mode recommande pour `OPUS-MT` tant que le runtime n'est pas embarque directement dans le service Rust.

Dans ce workspace, le branchement reel local est prepare autour de:

- venv Python: `/Users/simon/Projets/PlateformeDev/dev-trade-analyse/filfinance/filfinance-service/.venv-opus-mt`
- modele CTranslate2 `OPUS-MT tc-big en-fr`: `/Users/simon/Projets/PlateformeDev/dev-trade-analyse/filfinance/filfinance-service/data/models/opus-mt-tc-big-en-fr-ct2-int8`
- helper: [scripts/opus_mt_local_command.py](scripts/opus_mt_local_command.py)
- smoke test: [scripts/smoke_test_opus_mt_local.sh](scripts/smoke_test_opus_mt_local.sh)
- benchmark: [scripts/benchmark_opus_mt_local.py](scripts/benchmark_opus_mt_local.py)
- fixtures benchmark: [scripts/fixtures/benchmark_translation_headlines_en.txt](scripts/fixtures/benchmark_translation_headlines_en.txt)

Le fichier `.env` local pointe maintenant sur `OPUS-MT tc-big en-fr` avec `NEWS_TRANSLATION_PROFILE=quality`, soit `beam-size=4`, en mode `opus_mt_local_session`.

En Docker serveur, le runtime Python OPUS-MT est package dans l'image:

```env
NEWS_TRANSLATION_PROVIDER=opus_mt_local_session
NEWS_TRANSLATION_PROFILE=quality
NEWS_TRANSLATION_COMMAND=/opt/filfinance/opus-mt-venv/bin/python
NEWS_TRANSLATION_COMMAND_ARGS=/app/scripts/opus_mt_local_command.py,--ct2-model-dir,/opt/filfinance/models/opus-mt-tc-big-en-fr-ct2-int8,--sentencepiece-model,/opt/filfinance/models/opus-mt-tc-big-en-fr-ct2-int8/source.tokenizer.model,--target-sentencepiece-model,/opt/filfinance/models/opus-mt-tc-big-en-fr-ct2-int8/target.tokenizer.model,--device,cpu,--compute-type,int8,--translation-mode,opus_mt_tc_big_local_session
```

Le modele preconverti est installe pendant le build Docker dans `/opt/filfinance/models/opus-mt-tc-big-en-fr-ct2-int8`.

### Observabilite runtime de traduction

Le service expose maintenant un snapshot runtime sur:

- `/api/runtime/translation`

Ce payload permet de verifier rapidement:

- le provider actif
- le profil OPUS-MT actif
- le nombre d'items recus, traduits, ignores, passes en fallback et echoues
- le dernier `translation_mode`
- la derniere latence de traduction
- l'etat du warmup
- la derniere erreur utile au debug

### Benchmark local du checkpoint retenu

Commande:

```bash
cd /Users/simon/Projets/PlateformeDev/dev-trade-analyse/filfinance/filfinance-service
./.venv-opus-mt/bin/python scripts/benchmark_opus_mt_local.py --all-profiles --json
```

Mesure de reference relevee dans ce workspace le 3 mai 2026 sur `macOS arm64`, `cpu`, `int8`, lot de `12` headlines:

- `fast`: moyenne `296.9 ms`, p95 `761.95 ms`, debit `3.37 headlines/s`
- `balanced`: moyenne `322.4 ms`, p95 `413.4 ms`, debit `3.1 headlines/s`
- `quality`: moyenne `379.0 ms`, p95 `683.38 ms`, debit `2.64 headlines/s`

Ces chiffres sont indicatifs et servent de base MVP pour `NEWS-129`; ils dependent directement de la machine, du profil, du `compute-type` et du volume simultane.

## TTS serveur et clips HLS audio

Le service expose l'audio live via le control plane `audio_ready` et des clips HLS finalises par news:

- `/ws/news` pour le flux texte EN+FR user-facing
- `/ws/test-runs` pour le suivi E2E des runs de test
- `/audio/clips/{stream_id}/playlist.m3u8` pour lire le clip HLS finalise annonce
- `/audio/replay/today/playlist.m3u8` pour le replay journalier

Le worker TTS serveur est pilote par `Piper` via une session Python locale long-vivante. Il produit une intro courte, ajoute le corps audio, finalise la playlist HLS avec `#EXT-X-ENDLIST`, puis annonce `audio_ready`. `audio_finalized` et `audio_failed` restent publies pour l'observabilite et les erreurs.

Le WebSocket audio binaire historique `/ws/audio` est retire du chemin nominal et n'est plus expose.

Configuration minimale:

```env
NEWS_TTS_ENABLED=true
NEWS_TTS_PROVIDER=piper_local_session
NEWS_TTS_VOICE=fr_FR-upmc-medium-jessica
NEWS_TTS_COMMAND=/absolute/path/to/.venv-piper/bin/python
NEWS_TTS_COMMAND_ARGS=/absolute/path/to/scripts/piper_local_session.py
NEWS_TTS_MODELS_DIR=/absolute/path/to/data/models/piper
NEWS_TTS_RATE_WORDS_PER_MINUTE=185
NEWS_TTS_OUTPUT_SAMPLE_RATE_HZ=22050
NEWS_TTS_COMMAND_TIMEOUT_MS=12000
NEWS_TTS_WARMUP_ENABLED=true
NEWS_TTS_WARMUP_TEXT="Test audio. Les clips HLS audio sont operationnels."
```

Origin HLS clips/replay optionnel:

```env
NEWS_HLS_ENABLED=false
NEWS_HLS_SEGMENTS_DIR=data/hls
NEWS_HLS_FFMPEG_COMMAND=ffmpeg
NEWS_HLS_REPLAY_SEGMENT_TARGET_DURATION_SECS=4
NEWS_HLS_CLIP_SEGMENT_TARGET_DURATION_SECS=2
NEWS_HLS_ACCESS_CONTROL_ENABLED=false
NEWS_HLS_ACCESS_TICKET_SECRET=replace-with-at-least-32-random-characters
NEWS_HLS_ACCESS_TICKET_TTL_SECS=3600
```

Quand `NEWS_HLS_ENABLED=true`, le service encode le WAV interne en segments AAC-LC/MPEG-TS via `ffmpeg`.
Le live nominal passe par des evenements `audio_ready` qui pointent vers des clips finalises:

- `/audio/clips/{stream_id}/playlist.m3u8`
- `/audio/clips/{stream_id}/segment-000.ts`

Les routes historiques `/audio/live/playlist.m3u8` et `/audio/live/segments/{sequence}.ts`
ne font plus partie du contrat runtime.
Le replay du jour reste une fonctionnalite separee:

- `/audio/replay/today/playlist.m3u8`
- `/audio/replay/today/segments/{local_day}/{sequence}.ts`

Les playlists sont servies avec un cache tres court, tandis que les segments numerotes sont servis comme ressources immuables avec `Cloudflare-CDN-Cache-Control`.

En production durcie, `NEWS_HLS_ACCESS_CONTROL_ENABLED=true` doit etre active avec un secret aleatoire d'au moins 32 caracteres. Le client authentifie appelle alors `GET /api/hls-ticket`, ajoute `hls_ticket` a la playlist, et le backend recopie ce ticket dans les URLs de segments rendues dans le manifeste. Le ticket est partage par bucket temporel pour eviter un cache CDN par utilisateur.

Le replay HLS du jour reutilise une publication replay explicite produite depuis minuit. Il ne cree pas de radio permanente et ne conserve pas d'historique multi-jours.

En Docker, l'image backend lance maintenant un bootstrap automatique au demarrage:
si `NEWS_TTS_ENABLED=true`, `NEWS_TTS_PROVIDER=piper_local_session` et que la voix
Piper attendue manque dans `NEWS_TTS_MODELS_DIR`, elle est installee dans le volume
persistant `/app/data` avant le lancement du binaire Rust.

Bootstrap local hors Docker de la voix par defaut:

```bash
cd /Users/simon/Projets/PlateformeDev/dev-trade-analyse/filfinance/filfinance-service
./scripts/bootstrap_piper_local.sh fr_FR-upmc-medium
```

Bootstrap serveur Docker de la voix par defaut, depuis `/home/traefik/filfinance-docker`:

```bash
docker compose --env-file .env.server -f docker-compose.prod.yaml exec filfinance-service \
  env PIPER_VENV_DIR=/opt/filfinance/piper-venv \
      PIPER_DATA_DIR=/app/data/models/piper \
      PIPER_INSTALL_DEPS=false \
  /app/scripts/bootstrap_piper_local.sh fr_FR-upmc-medium
```

Configuration Docker attendue:

```env
NEWS_TTS_COMMAND=/opt/filfinance/piper-venv/bin/python
NEWS_TTS_COMMAND_ARGS=/app/scripts/piper_local_session.py
NEWS_TTS_MODELS_DIR=/app/data/models/piper
```

Point important sur les voix multi-speaker:

- le dossier modele reste `fr_FR-upmc-medium`
- le selecteur serveur/client expose des IDs plats de type `Language-Voice-Quality-Speaker`
- exemple: `fr_FR-upmc-medium-jessica`, `fr_FR-upmc-medium-pierre`

Suppression d'une voix locale:

```bash
cd /Users/simon/Projets/PlateformeDev/dev-trade-analyse/filfinance/filfinance-service
./scripts/remove_piper_voice.sh fr_FR-mls-medium
```

Si `.env` contient `NEWS_TTS_VOICE` avec cette meme voix, ou une selection derivee du type `fr_FR-upmc-medium-jessica`, le script supprime aussi cette ligne pour eviter un redemarrage avec une voix devenue invalide.

Observabilite runtime:

- `/api/runtime/tts`

Ce snapshot permet de verifier rapidement:

- le provider actif
- la voix courante
- le nombre d'items recus, ignores, generes, en erreur et produits sans client audio
- la derniere latence de synthese
- le dernier volume audio produit, en bytes et en chunks
- l'etat du warmup
- la derniere erreur TTS utile au debug

Normalisation de narration:

- le worker TTS normalise maintenant `narration_text` avant synthese pour rendre la lecture plus intelligible
- exemples: `EUR/USD` devient `E U R contre U S D`, `+3%` devient `plus 3 pour cent`, `NVDA` devient `N V D A`

Benchmark local du provider nominal:

```bash
cd /Users/simon/Projets/PlateformeDev/dev-trade-analyse/filfinance/filfinance-service
python3 scripts/benchmark_tts_local.py --json
```

Point de cadrage:

- les voix Piper ne sont pas versionnees dans Git; elles sont bootstrappees localement dans `data/models/piper`
- le helper Python reste local au service et produit les WAV sources que l'origin HLS segmente

Le worker consomme exactement le meme flux traduit que Telegram et `/ws/news`. Pour un run de test E2E, la sequence attendue est donc:

```text
/ws/test-runs -> pipeline -> traduction EN/FR -> Telegram -> /ws/news -> TTS/HLS finalise -> audio_ready -> /audio/clips/{stream_id}/playlist.m3u8
```

Point important:

- le serveur ne depend plus de `macos_say` ni d'une voix systeme macOS
- le client detruit le player HLS quand la queue de clips est vide
- le changement de voix live a chaud via WebSocket audio est retire; la voix releve de la configuration serveur/profil
- le client charge son historique via `/api/translations` sans autoplay, puis ne lit automatiquement que les news live nouvelles absentes de cet historique local

## Consigne permanente de developpement

- commenter le code en francais
- commenter tous les ajouts, modifications et logiques nouvelles ou modifiees

## Installation complete de l'environnement Rust sur macOS avec Homebrew

### 1. Installer les prerequis macOS

```bash
xcode-select --install
```

### 2. Installer `rustup` avec Homebrew

```bash
brew update
brew install rustup
```

Si tu as deja installe la formule Homebrew `rust`, place le binaire `rustup` avant le prefixe Homebrew global dans ton `PATH`:

```bash
echo 'export PATH="$(brew --prefix rustup)/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### 3. Initialiser la toolchain Rust stable

```bash
rustup default stable
rustup update
```

### 4. Charger Cargo dans le shell courant et rendre le chargement persistant

```bash
source "$HOME/.cargo/env"
echo 'source "$HOME/.cargo/env"' >> ~/.zshrc
source ~/.zshrc
```

### 5. Installer les composants utiles au developpement

```bash
rustup component add rustfmt clippy
```

### 6. Verifier l'installation

```bash
rustc --version
cargo --version
rustup --version
rustfmt --version
cargo clippy --version
```

## Installation et usage avec VSCodium

### 1. Installer VSCodium avec Homebrew

```bash
brew install --cask vscodium
```

### 2. Ouvrir le projet dans VSCodium

```bash
cd /Users/simon/Projets/PlateformeDev/dev-trade-analyse/filfinance/filfinance-service
open -a VSCodium .
```

Si la CLI `codium` est disponible dans ton `PATH`, tu peux aussi utiliser:

```bash
codium .
```

### 3. Installer les extensions recommandees pour Rust

```bash
codium --install-extension rust-lang.rust-analyzer
codium --install-extension vadimcn.vscode-lldb
```

Si `codium` n'est pas trouve alors que `VSCodium.app` est bien present dans `/Applications`, ajoute sa CLI au `PATH`:

```bash
echo 'export PATH="/Applications/VSCodium.app/Contents/Resources/app/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
which codium
```

### 4. Reglages et verification rapide dans VSCodium

- ouvrir le dossier `filfinance-service`
- verifier que `rust-analyzer` indexe correctement le workspace
- lancer `cargo check` dans le terminal integre
- utiliser `cargo fmt` pour le formatage
- utiliser `cargo clippy --all-targets --all-features` pour les lints
- utiliser l'extension `CodeLLDB` pour le debug Rust sur macOS

## Depannage

### `cargo check` echoue avec `reqwest` et la feature `rustls-tls`

Le manifeste de ce module utilise `reqwest` 0.13 avec la feature `rustls`.

```toml
reqwest = { version = "0.13", default-features = false, features = ["json", "rustls", "gzip", "brotli"] }
```

Si tu vois encore une erreur mentionnant `rustls-tls`, verifie que tu lances bien les commandes depuis:

```bash
cd /Users/simon/Projets/PlateformeDev/dev-trade-analyse/filfinance/filfinance-service
```

Puis relance:

```bash
cargo check
cargo clippy --all-targets --all-features
```

### `FinancialJuice` ne remonte pas de news en temps reel

Le collecteur principal n'utilise plus le RSS comme chemin nominal. Il attend maintenant:

- `NEWS_FINANCIAL_JUICE_HOME_URL=https://www.financialjuice.com/home`
- `NEWS_FINANCIAL_JUICE_EMAIL`
- `NEWS_FINANCIAL_JUICE_PASSWORD`

Puis il ouvre le chemin:

- login WebForms sur `/home`
- bootstrap JS (`MainURLData`, `info`, `feedtoken`)
- backlog via `FJService.asmx/GetPreviousNews`
- live via SignalR 2.x

Si l'authentification echoue, le service reste en mode degrade sans fallback RSS.

### `codium: command not found`

Sur cette machine, `VSCodium.app` est installe dans `/Applications`, mais la CLI n'est pas encore dans le `PATH`.

```bash
echo 'export PATH="/Applications/VSCodium.app/Contents/Resources/app/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
codium --list-extensions
```

## Lancement local

```bash
cd /Users/simon/Projets/PlateformeDev/dev-trade-analyse/filfinance/filfinance-service
cp .env.example .env
cargo run
```

Point important: le chemin primaire `FinancialJuice` exige maintenant de renseigner `NEWS_FINANCIAL_JUICE_EMAIL` et `NEWS_FINANCIAL_JUICE_PASSWORD` dans `.env`.

## Variables principales

- `NEWS_LISTEN_ADDR`
- `NEWS_DATABASE_URL`
- `NEWS_STORAGE_MAX_CONNECTIONS`
- `NEWS_DATA_AUDIT_RETENTION_DAYS`
- `NEWS_LOG_FILE_ENABLED`
- `NEWS_LOG_DIR`
- `NEWS_LOG_RETENTION_DAYS`
- `NEWS_LOG_ADMIN_MAX_LINES`
- `NEWS_CALENDAR_LOG_DIR`
- `NEWS_HTTP_CONNECT_TIMEOUT_MS`
- `NEWS_*_POLL_INTERVAL_MS`
- `NEWS_*_REQUEST_TIMEOUT_MS`
- `NEWS_*_RATE_LIMIT_BACKOFF_MS`
- `NEWS_FINANCIAL_JUICE_HOME_URL`
- `NEWS_FINANCIAL_JUICE_EMAIL`
- `NEWS_FINANCIAL_JUICE_PASSWORD`
- `NEWS_FINANCIAL_JUICE_SESSION_REFRESH_INTERVAL_MS`
- `NEWS_FINANCIAL_JUICE_LIVE_RETRY_BACKOFF_MS`
- `NEWS_FINANCIAL_JUICE_STARTUP_BACKFILL_PAGE_LIMIT`
- `NEWS_HLS_*`
- `NEWS_TELEGRAM_*`

## Logs admin

Le backend ecrit les logs applicatifs dans `NEWS_LOG_DIR` quand `NEWS_LOG_FILE_ENABLED=true`.
Les fichiers sont nommes `YYYY-MM-DD.log`, en UTC, et la retention reste bornee a J et J-1
avec `NEWS_LOG_RETENTION_DAYS=2`. Les routes operateur `/api/admin/logs/days` et
`/api/admin/logs` lisent uniquement ces fichiers quotidiens, sans accepter de chemin arbitraire.

## Logs détaillés du workflow calendrier

Le calendrier écrit aussi des diagnostics ordonnés `RLOG/1` sous
`NEWS_CALENDAR_LOG_DIR/rlogger` (par défaut `data/calendar-logs/rlogger`).
`NEWS_CALENDAR_LOG_DIR` désigne le parent applicatif. Ce flux est séparé
des logs applicatifs : il contient les étapes, entrées et sorties des collectes,
appels IA, persistance, Piper, HLS et publication edge. Les événements `workflow.attempt.started`
et `workflow.attempt.finished` indiquent la tentative, sa durée et son issue métier ;
`workflow.final` n'est émis qu'après un état durable `ready` ou `error`. Les collecteurs
indiquent leur durée et le nombre de lignes lues, puis l'ingestion journalise chaque ligne
validée et les nombres de lignes persistées. Les prompts réellement envoyés sont présents
une seule fois dans `openrouter.request`, suivis des statuts HTTP et corps reçus. Un corps
dépassant la limite HTTP porte un événement `*.response_body_incomplete`. Les secrets sont
masqués avant écriture ; les WAV restent représentés par leurs métadonnées.

Le journal calendrier ne recopie plus les traces techniques des bibliothèques, notamment
les millions d'événements des parseurs HTML. En production, lorsque le volume dispose de
moins de 10 Gio libres, les contenus détaillés sont suspendus et chaque omission est
signalée par `diagnostics.degraded` et `payload.suppressed` ; les événements courts de
progression continuent. Le seuil local est de 100 Mio. Le service n'efface jamais de
segment non vide automatiquement : l'exploitant doit surveiller la croissance du dossier.

Les fichiers suivent la forme
`rlogger/YYYY-MM-DD/calendar/workflow-HH-<run-id>-h<heure-UTC>-s<segment>.active.log`.
La rotation horaire clôt les segments en `.log`; un actif abandonné est récupéré
en `.recovered.log` sous bail système. Les fichiers non vides ne sont jamais
supprimés automatiquement. La page admin « Workflow calendrier » donne accès,
aux seuls administrateurs, aux tailles, au suivi en direct, au téléchargement et
à la suppression confirmée des seuls segments finalisés dont l’heure est terminée.
Les anciens dossiers `run-*` restent intacts sur disque et ne sont plus affichés.

## Variables Telegram

- `NEWS_TELEGRAM_API_BASE_URL`
- `NEWS_TELEGRAM_QUEUE_BUFFER`
- `NEWS_TELEGRAM_RATE_LIMIT_MS`
- `NEWS_TELEGRAM_RETRY_ATTEMPTS`
- `NEWS_TELEGRAM_RETRY_BACKOFF_MS`

## Recuperer un `chat_id` Telegram

Pour recuperer l'identifiant du canal ou du chat Telegram, commence par envoyer un message au bot ou par ajouter le bot au canal, puis appelle:

```bash
curl "https://api.telegram.org/bot<TELEGRAM_BOT_TOKEN>/getUpdates"
```

Ou avec une variable d'environnement:

```bash
curl "https://api.telegram.org/bot${NEWS_TELEGRAM_BOT_TOKEN}/getUpdates"
```

Le `chat_id` apparait dans le JSON retourne par Telegram.

## Endpoints

- `GET /health`
- `GET /api/sources` (admin uniquement)
- `GET /api/news?limit=100&source=financial_juice&event_kind=headline&before_id=250`
- `GET /api/translations?day=2026-04-23&history_scope=real_feed&source=financial_juice&limit=100`
- `GET /api/translations?day=2026-04-23&history_scope=test_run&limit=100`
- `GET /api/translations/{id}`
- `GET/POST /api/news-blacklist` (moderator/admin)
- `PATCH/DELETE /api/news-blacklist/{id}` (moderator/admin)
- `GET /api/news-blacklist/hits` (moderator/admin)
- `POST /api/news-blacklist/reapply` (moderator/admin)
- `POST /api/glossary/reapply-translations` (moderator/admin)
- `GET /api/news/{id}`
- `GET /api/replay/today`
- `GET /api/hls-ticket`
- `GET /audio/clips/{stream_id}/playlist.m3u8`
- `GET /audio/clips/{stream_id}/{segment_name}`
- `GET /audio/replay/today/playlist.m3u8`
- `GET /audio/replay/today/segments/{local_day}/{sequence}.ts`
- `WS /ws/news`

## Contrat HTTP MVP

### `GET /api/news`

Query params supportes:

- `limit`
- `source`
- `event_kind`
- `before_id`

Exemple de reponse:

```json
{
  "items": [],
  "has_more": false,
  "next_before_id": null
}
```

### `GET /api/news/:id`

Retourne un evenement unique persiste pour le debug ponctuel. Repond `404` si l'identifiant n'existe pas.

### `GET /api/sources`

Retourne les details runtime complets des sources: configuration effective, etat runtime, fallback, diagnostics collecteur, raison de coupure automatique et switches disponibles.

Cette route est reservee au role applicatif `admin`. Les badges sources user-facing ne doivent pas appeler cette route; ils sont alimentes par le resume public diffuse dans `hello`, `heartbeat` et le control plane Edge.

### `GET /api/hls-ticket`

Retourne un ticket HLS court pour un utilisateur authentifie. Quand `NEWS_HLS_ACCESS_CONTROL_ENABLED=false`, la reponse indique `enabled=false` et le client conserve les URLs HLS nues, utile en local. Quand le controle est actif, la playlist et les segments `/audio/*` exigent `hls_ticket`.

### `GET /api/translations`

Retourne l'historique traduit EN+FR pret a afficher ou a rejouer cote client.

Query params supportes:

- `limit`
- `source`
- `day` au format `YYYY-MM-DD`
- `history_scope` avec `all`, `real_feed` ou `test_run`
- `before_id`

Point important:

- `history_scope=real_feed` exclut explicitement les evenements synthetiques `test_run`
- `history_scope=test_run` retourne uniquement les runs E2E
- les items sont projetes dans le meme contrat `LiveNewsItem` que `/ws/news`
- les corps `source_body`, `translated_body` et `narration_text` sont retournes complets pour l'historique HTTP
- `news_level` vient du champ source FinancialJuice `Level`; une chaine vide est exposee comme `null`

### `GET /api/translations/{id}`

Retourne une traduction unique complete dans le meme shape que `TranslatedNewsItem`. Cette route est protegee listener, repond `404` si l'identifiant n'existe pas, et sert aux vues qui doivent charger un detail complet sans alourdir le flux WebSocket live.

### `GET/POST/PATCH/DELETE /api/news-blacklist`

Les routes blacklist sont protegees moderator/admin. Les regles exposent `term`, `match_mode`, `target_scope`, `enabled`, `notes` et les dates techniques. `target_scope` accepte `title`, `content` ou `title_content`; les regles existantes ou payloads sans `target_scope` gardent le defaut `title_content`.

Comportement hot path:

- `title`: teste uniquement le titre source et rejette toute la news si le filtre matche.
- `content`: teste uniquement le contenu source; si le filtre matche, la news reste live avec son titre, mais son contenu est retire avant persistence, traduction, audio et broadcast.
- `title_content`: teste titre puis contenu et rejette toute la news si l'un des deux matche.

`GET /api/news-blacklist/hits` retourne les audits avec `target_scope` et `action`. `action=reject_news` signifie que la news a ete rejetee; `action=strip_content` signifie que seul le contenu a ete retire.

`POST /api/news-blacklist/reapply` applique les regles actives aux news `financial_juice` deja enregistrees dans la table retenue. Comme `news_events` est bornee par retention a J/J-1, la commande scanne toute la table utile cote SQLite, valide ensuite le match exact avec le matcher Rust, puis agit sans diffuser sur le WebSocket live:

- `title` et `title_content` en action `reject_news`: audit blacklist, suppression des qualifications journalieres liees, des traductions, puis de `news_events`.
- `content` en action `strip_content`: audit blacklist, `news_events.body = NULL`, nettoyage de `source_body/translated_body`, puis retraduction sans broadcast live.

La reponse expose `local_day`, `local_days`, `targeted_total`, `rejected_total`, `stripped_total`, `reprocessed_total` et `errors`. `local_day` reste le jour courant pour compatibilite, tandis que `local_days` documente la fenetre de retention attendue `[J, J-1]`. Le retraitement se base sur l'etat reel stocke: une news supprimee ne revient plus dans les candidates, et une news `content` dont `body` a deja ete vide ne matche plus. Un audit `strip_content` existant ne suffit pas a sauter une news si son contenu est encore present, afin de corriger les anciens cas partiellement traites.

### `POST /api/glossary/reapply-translations`

Retraduit les news `financial_juice` deja enregistrees dans la table retenue et qui matchent au moins une regle active du glossaire. Comme `news_events` est bornee par retention a J/J-1, la commande scanne toute la table utile cote SQLite, valide ensuite les matches exacts du pipeline glossaire en Rust, ne diffuse pas sur le WebSocket live et retourne `local_day`, `local_days`, `targeted_total`, `reprocessed_total`, `display_repaired_total` et `errors`. `local_day` reste le jour courant pour compatibilite, tandis que `local_days` documente la fenetre de retention attendue `[J, J-1]`.

La commande est idempotente: elle calcule d'abord les libelles FR attendus depuis la source, repare les traductions existantes quand c'est possible (`BCE's Nagel` ou `Nagel: de la BCE` -> `Nagel de la BCE`) et saute OPUS si les libelles attendus sont deja presents. OPUS-MT n'est relance que pour les lignes sans traduction existante ou dont le texte repare ne satisfait toujours pas les regles source. Le retraitement ne depend pas de `translation.updated_at`: il peut donc reparer les lignes deja traduites quand le code du glossaire change sans modification manuelle des regles.

Le mode glossaire `pattern` supporte les captures `<number>` et `<string>`. `<string>` capture un token lexical unique sans ponctuation terminale comme `:`, ce qui permet par exemple l'entree active `ECB's <string>` -> `<string> de la BCE` pour transformer `ECB's Nagel` en `Nagel de la BCE`. Un motif separe `<string>:` -> `<string> :` peut ensuite appliquer l'espace typographique avant deux-points, y compris apres restauration du motif BCE. Les motifs de pur formatage sans litteral metier, comme `<string>:`, comptent pour `Update glossaire` mais ne protegent pas le texte source avant traduction, afin de ne pas empecher des motifs plus precis comme `ECB's <string>` de capturer leur terme. `Update glossaire` applique ces motifs de formatage directement aux traductions existantes, sans repasser par OPUS-MT.

Le client web appelle cette route en `POST` avec un JWT Better Auth; le CORS serveur autorise donc les methodes `GET,POST,PATCH,DELETE,OPTIONS` et les headers `authorization,content-type` pour laisser passer le preflight navigateur des mutations admin.

## Contrat WebSocket MVP

Le serveur envoie des messages JSON tagues:

- `hello`
- `heartbeat`
- `translated_news_item`
- `lagged`

Exemple minimal:

```json
{
  "type": "hello",
  "payload": {
    "protocol_version": 1,
    "server_time": "2026-04-21T09:30:00Z",
    "broadcast_buffer": 2048,
    "sources": [
      {
        "source": "financial_juice_browser",
        "enabled": true,
        "status": "active",
        "updated_at": "2026-05-09T09:12:00Z"
      }
    ]
  }
}
```

En cas de client lent, le serveur continue a diffuser et emet un message `lagged` avec le nombre de messages perdus.

En l'absence de news, le serveur emet aussi un `heartbeat` periodique pour limiter les expirations de connexions WebSocket inactives sur mobile et rafraichir le resume public des sources.

Exemple de message derive:

```json
{
  "type": "translated_news_item",
  "payload": {
    "id": 12,
    "news_event_id": 7,
    "language": "fr",
    "status": "ready",
    "source": "financial_juice",
    "event_kind": "headline",
    "source_title": "Powell says rate cuts stay data-dependent",
    "source_body": null,
    "translated_title": "Powell indique que les baisses de taux dependent des donnees",
    "translated_body": null,
    "narration_text": "14:37. Powell indique que les baisses de taux dependent des donnees.",
    "translation_mode": "heuristic_fr",
    "error_message": null,
    "news_level": "active active-critical",
    "url": "https://example.test/item",
    "timestamp": "2026-04-21T09:30:00Z",
    "created_at": "2026-04-21T09:30:02Z",
    "updated_at": "2026-04-21T09:30:02Z"
  }
}
```

Ce payload derive compact sert de flux live commun pour les consommateurs temps reel. Le client user-facing affiche le titre FR et charge le detail FR au besoin; l'anglais source reste reserve aux surfaces admin/historique. Les corps complets sont disponibles par HTTP via `/api/translations` et `/api/translations/{id}`.

## Etat actuel

La base Rust est posee, remplace le prototype Python et suit maintenant l'organisation feature-first
documentee plus haut.

- pipeline async borne pour l'ingest
- persistence SQLite en WAL avec bootstrap, migration idempotente et repository pagine
- hub WebSocket avec contrat `hello/heartbeat/translated_news_item/lagged`
- worker Telegram avec queue bornee, rate limit et retry borne
- modele `NewsEvent` commun pour ingestion, persistence et diffusion
- collecteur `FinancialJuice` primaire via session web authentifiee, `GetPreviousNews` et live SignalR
- verification locale des JWT Better Auth sur les routes protegees
- profils et preferences portes par `filfinance-auth` dans PostgreSQL
- traduction locale configurable avec provider heuristique ou commande locale
- TTS Piper et origin HLS clips/replay isoles dans la feature `audio`
- publication metadata live edge optionnelle et WebSocket backend conserve en fallback
- ancien collecteur macro externe retire du scope produit et du runtime
