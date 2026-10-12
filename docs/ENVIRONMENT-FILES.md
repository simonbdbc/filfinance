# Fichiers d'environnement

## Objectif

Clarifier l'emplacement et l'utilite de chaque fichier `.env*` du repo Fil Finance.

Regle generale:

- les fichiers `.env` reels contiennent des secrets ou des valeurs machine et ne sont pas versionnes;
- les fichiers `.env.example` et `.env.server.example` sont versionnes;
- les exemples doivent avoir les memes cles que leur fichier reel associe, avec des placeholders pour les valeurs critiques.

## Synthese

| Fichier | Versionne | Utilite | Utilise par |
| --- | --- | --- | --- |
| `.env.local.example` | Oui | Template du lancement Docker local complet | `compose.yaml` + `compose.local.yaml` via copie vers `.env.local` |
| `.env.local` | Non | Secrets et config runtime locaux pour backend, collector navigateur et frontend Docker | `docker compose --env-file .env.local` |
| `.env.server.example` | Oui | Template du fichier serveur Docker Compose | `compose.yaml` + `compose.prod.yaml` via copie vers `.env.server.prod` |
| `.env.server.prod` | Non | Secrets et config runtime de la production OVH | `/srv/filfinance` et `docker compose --env-file .env.server.prod` |
| `filfinance-service/.env.example` | Oui | Template du backend Rust hors Docker | copie vers `filfinance-service/.env` |
| `filfinance-service/.env` | Non | Config locale du backend Rust quand on lance `cargo run` depuis `filfinance-service` | lancement hors Docker uniquement |
| `apps/pwa/.env.example` | Oui | Template du client Expo Web local | copie vers `apps/pwa/.env` |
| `apps/pwa/.env` | Non | Variables publiques Expo en local | Expo, Metro, export web local |
| Variables GitLab de build landing | Non, hors repo | Origines publiques injectees dans l'artefact Astro | GitLab CI |
| Variables GitLab de build PWA | Non, hors repo | Variables publiques Expo injectees dans l'artefact Web | GitLab CI |

La production active est le VPS OVH : le clone est `/srv/filfinance` et le
fichier runtime est `/srv/filfinance/.env.server.prod`. Les anciens chemins
`/raid/app/filfinance` conserves plus loin sont des exemples historiques du
palier `sbalgo-srv`.

## Racine: `.env.local.example`

Emplacement:

```text
.env.local.example
```

Utilite:

- template pour lancer toute la stack locale Docker;
- sert a creer `.env.local` a la racine du repo;
- alimente le backend Rust, le collector navigateur FinancialJuice et le frontend Expo Web Docker;
- utilise Better Auth/PostgreSQL pour l'identite, les roles, profils et preferences.

Creation locale:

```bash
cp .env.local.example .env.local
chmod 600 .env.local
```

Lancement local complet:

```bash
docker compose --env-file .env.local -f compose.yaml -f compose.local.yaml up --build
```

Services exposes:

- `filfinance-client`: `http://localhost:8052`
- `filfinance-service`: `http://localhost:8088`
- `financialjuice-browser-collector`: non expose; communication interne avec le backend.

Valeurs critiques a remplacer dans `.env.local`:

- `NEWS_SERVER_ENV=local`
- `POSTGRES_PASSWORD`, `FILFINANCE_AUTH_DB_PASSWORD`, `FILFINANCE_APP_DB_PASSWORD`
- `BETTER_AUTH_SECRET`, `BETTER_AUTH_URL`, `BETTER_AUTH_JWT_AUDIENCE`
- `BETTER_AUTH_REQUIRE_EMAIL_VERIFICATION=true`, ainsi qu'un SMTP local reel; sans transport, l'envoi echoue explicitement et aucun JWT non verifie n'est accepte
- `AUTH_DATABASE_URL`, `DATABASE_URL`
- `NEWS_AUTH_ISSUER`, `NEWS_AUTH_JWKS_URL`, `NEWS_AUTH_AUDIENCE`
- `EXPO_PUBLIC_API_URL`, `EXPO_PUBLIC_BETTER_AUTH_URL`, `EXPO_PUBLIC_FILFINANCE_PWA_URL`
- `NEWS_FINANCIAL_JUICE_EMAIL`
- `NEWS_FINANCIAL_JUICE_PASSWORD`
- `NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN`
- `FILFINANCE_CALENDAR_DAY_ENABLED=true` pour activer le calendrier Rust natif
- `FILFINANCE_CALENDAR_DAY_SCHEDULE_TIME=01:30`
- `FILFINANCE_CALENDAR_DAY_RETRY_END_TIME=03:00`
- `FILFINANCE_CALENDAR_WEEK_GENERATION_WINDOW_START=02:00`
- `FILFINANCE_CALENDAR_WEEK_GENERATION_WINDOW_END=04:00`
- `FILFINANCE_CALENDAR_WEEK_VERIFICATION_WINDOW_START=02:00`
- `FILFINANCE_CALENDAR_WEEK_VERIFICATION_WINDOW_END=04:00`
- `FILFINANCE_CALENDAR_ALERT_OFFSETS_SECS=300,60`
- `FILFINANCE_CALENDAR_NATIVE_REQUEST_TIMEOUT_MS=60000`
- `FILFINANCE_CALENDAR_NATIVE_RETRY_ATTEMPTS=2`
- `FILFINANCE_CALENDAR_NATIVE_RETRY_BACKOFF_MS=500`
- `FILFINANCE_CALENDAR_NATIVE_MAX_CONCURRENCY=1`
- `FILFINANCE_CALENDAR_NATIVE_QUEUE_CAPACITY=4`
- `FILFINANCE_CALENDAR_NATIVE_MAX_RESPONSE_BYTES=1000000`
- `OPENROUTER_API_KEY`, `OPENROUTER_MODEL`
- `NEWS_TELEGRAM_BOT_TOKEN`, `NEWS_TELEGRAM_CHAT_IDS` si les notifications Telegram sont actives
- les secrets Telegram/Edge si ces integrations sont activees.

Le calendrier utilise directement la file SQLite Rust, OpenRouter et les workers Piper/HLS deja presents dans la stack ; aucun compose supplementaire n'est requis:

```bash
docker compose --env-file .env.local -f compose.yaml -f compose.local.yaml up -d --build
```

## Racine: `.env.server.example`

Emplacement:

```text
.env.server.example
```

Utilite:

- template de production serveur;
- sert a creer `.env.server.prod` dans `/srv/filfinance` pour la production
  OVH ; les fichiers DEV historiques restent documentes plus loin ;
- alimente le backend Rust et le container `financialjuice-browser-collector` lances par `compose.yaml` + `compose.prod.yaml`;
- ne concerne pas les frontends statiques publies sur Bunny.

Creation sur le serveur prod:

```bash
cd /srv/filfinance
cp .env.server.example .env.server.prod
chmod 600 .env.server.prod
```

Les fichiers reels attendus par le deploy sont:

```text
/srv/filfinance/.env.server.prod
```

`compose.prod.yaml` le charge via:

```yaml
env_file:
  - ${FILFINANCE_ENV_FILE:-.env.server}
```

Lancement production backend + collector:

```bash
docker compose --env-file .env.server.prod -f compose.yaml -f compose.prod.yaml up -d --build
```

Valeurs critiques a remplacer dans `.env.server.prod`:

- `NEWS_SERVER_ENV=production`
- `POSTGRES_PASSWORD`, `FILFINANCE_AUTH_DB_PASSWORD`, `FILFINANCE_APP_DB_PASSWORD`
- `BETTER_AUTH_SECRET`, `BETTER_AUTH_URL`, `BETTER_AUTH_JWT_AUDIENCE`
- `BETTER_AUTH_TRUSTED_ORIGINS`, `SMTP_HOST`, `SMTP_FROM_EMAIL`, `SMTP_FROM_NAME`
- `AUTH_DATABASE_URL`, `DATABASE_URL`
- `NEWS_AUTH_PROVIDER=better_auth`
- `NEWS_AUTH_ISSUER`
- `NEWS_AUTH_JWKS_URL`
- `NEWS_AUTH_AUDIENCE`
- `NEWS_FINANCIAL_JUICE_EMAIL`
- `NEWS_FINANCIAL_JUICE_PASSWORD`
- `NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN` si le collecteur navigateur FinancialJuice est active
- `FILFINANCE_CALENDAR_DAY_ENABLED=true` pour activer le calendrier Rust natif
- `NEWS_HLS_ACCESS_TICKET_SECRET`
- `NEWS_TRANSLATION_WORKERS=1`
- `NEWS_TTS_WORKERS=1`
- `NEWS_HTTP_SLOW_REQUEST_MS=750`
- `FILFINANCE_CALENDAR_DAY_SCHEDULE_TIME=01:30`
- `FILFINANCE_CALENDAR_DAY_RETRY_END_TIME=03:00`
- `FILFINANCE_CALENDAR_WEEK_GENERATION_WINDOW_START=02:00`
- `FILFINANCE_CALENDAR_WEEK_GENERATION_WINDOW_END=04:00`
- `FILFINANCE_CALENDAR_WEEK_VERIFICATION_WINDOW_START=02:00`
- `FILFINANCE_CALENDAR_WEEK_VERIFICATION_WINDOW_END=04:00`
- `FILFINANCE_CALENDAR_ALERT_OFFSETS_SECS=300,60`
- `FILFINANCE_CALENDAR_NATIVE_REQUEST_TIMEOUT_MS=60000`
- `FILFINANCE_CALENDAR_NATIVE_RETRY_ATTEMPTS=2`
- `FILFINANCE_CALENDAR_NATIVE_RETRY_BACKOFF_MS=500`
- `FILFINANCE_CALENDAR_NATIVE_MAX_CONCURRENCY=1`
- `FILFINANCE_CALENDAR_NATIVE_QUEUE_CAPACITY=4`
- `FILFINANCE_CALENDAR_NATIVE_MAX_RESPONSE_BYTES=1000000`
- `OPENROUTER_API_KEY`, `OPENROUTER_MODEL`
- `NEWS_TELEGRAM_BOT_TOKEN`, `NEWS_TELEGRAM_CHAT_IDS` si Telegram est active
- `NEWS_X_BEARER_TOKEN` si le collecteur X est active
- `NEWS_X_ACCOUNT_USERNAMES` ou `NEWS_X_QUERY` si le collecteur X est active
- `NEWS_X_KEYWORDS` si un filtrage par mots cles est souhaite
- `NEWS_TRANSLATION_PROVIDER`
- `NEWS_TRANSLATION_PROFILE`
- `NEWS_TRANSLATION_COMMAND`
- `NEWS_TRANSLATION_COMMAND_ARGS`
- `NEWS_EDGE_PUBLISH_URL` si le publisher metadata edge est active
- `NEWS_EDGE_PUBLISH_SECRET` si le publisher metadata edge est active
- `NEWS_EDGE_TICKET_SECRET` si les tickets edge sont actifs
- `NEWS_HLS_ENABLED` si les clips HLS audio/replay sont actifs
- `NEWS_HLS_SEGMENTS_DIR` si les clips HLS audio/replay sont actifs
- `NEWS_HLS_FFMPEG_COMMAND` si le binaire n'est pas `ffmpeg`
- `NEWS_HLS_ACCESS_CONTROL_ENABLED=true` en production durcie HLS/CDN
- `NEWS_HLS_ACCESS_TICKET_SECRET` si le controle HLS est actif
- `NEWS_TELEGRAM_BOT_TOKEN` si Telegram est active
- `NEWS_TELEGRAM_CHAT_IDS` si Telegram est active

Note sur `NEWS_TRANSLATION_MAX_BODY_CHARS`:

- cette variable borne uniquement les payloads live compacts envoyes aux clients;
- elle ne tronque plus les textes persistés dans `news_events` / `news_translations`;
- elle ne tronque plus le corps envoye au provider de traduction.

Lancement production backend + collector + calendrier Rust:

```bash
docker compose --env-file .env.server -f compose.yaml -f compose.prod.yaml up -d --build
```

Ne pas mettre dans `.env.server`:

- `EXPO_PUBLIC_FILFINANCE_WS_URL`
- `EXPO_PUBLIC_FILFINANCE_EDGE_WS_URL`
- `EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL`
- `EXPO_PUBLIC_FILFINANCE_PWA_URL`
- `EXPO_PUBLIC_API_URL`
- `EXPO_PUBLIC_BETTER_AUTH_URL`

Ces variables appartiennent au frontend et doivent etre configurees dans GitLab CI
pour les artefacts deployes, ou dans `apps/pwa/.env` en local. Bunny ne construit
pas les artefacts : la CI synchronise les builds deja produits dans Storage.

## Collecteur navigateur FinancialJuice Docker

Le service `financialjuice-browser-collector` est declare dans `compose.yaml`.
Il est demarre avec le backend, mais reste en veille tant que le backend renvoie
`fallback_active=false` sur:

```text
GET /api/ingest/financial-juice/browser/control
```

Le backend Docker ne depend plus d'une image locale `filfinance-service-runtime-base:bookworm`.
Le runtime OPUS-MT/Piper est construit dans un stage interne du Dockerfile backend,
ce qui rend `docker compose ... up --build` suffisant en local comme en production.

Variables issues de `.env.local` ou `.env.server` par interpolation `docker compose --env-file ...`
et consommees par le collector:

- `NEWS_FINANCIAL_JUICE_BROWSER_ENABLED`
- `NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN`
- `NEWS_FINANCIAL_JUICE_BROWSER_TARGETS`
- `NEWS_FINANCIAL_JUICE_BROWSER_BACKEND_URL_PROD`
- `NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN_PROD`
- `NEWS_FINANCIAL_JUICE_BROWSER_BACKEND_URL_DEV`
- `NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN_DEV`
- `NEWS_FINANCIAL_JUICE_BROWSER_HOME_URL`
- `NEWS_FINANCIAL_JUICE_BROWSER_HEADLESS` (`false` par defaut dans Docker Compose; l'image Playwright lance alors Xvfb automatiquement si aucun display n'est fourni)
- `NEWS_FINANCIAL_JUICE_BROWSER_POLL_INTERVAL_MS`
- `NEWS_FINANCIAL_JUICE_BROWSER_REQUEST_TIMEOUT_MS`
- `NEWS_FINANCIAL_JUICE_BROWSER_MAX_BATCH_SIZE`
- `NEWS_FINANCIAL_JUICE_EMAIL`
- `NEWS_FINANCIAL_JUICE_PASSWORD`

Variables fixees par `compose.yaml` pour le contexte container:

- `NEWS_FINANCIAL_JUICE_BROWSER_BACKEND_URL=http://filfinance-service:8088` si aucun fanout n'est configure
- `NEWS_FINANCIAL_JUICE_BROWSER_USER_DATA_DIR=/app/data/financialjuice-browser-profile`

Topologie prod/dev actuelle:

- prod cree et garde le seul container `financialjuice-browser-collector`;
- prod configure `NEWS_FINANCIAL_JUICE_BROWSER_TARGETS=prod,dev`;
- la target `prod` pointe vers `http://filfinance-service:8088`;
- la target `dev` pointe vers `http://host.docker.internal:18088`;
- le deploy dev scale `financialjuice-browser-collector` a `0`;
- prod garde `NEWS_FINANCIAL_JUICE_ENABLED=true`;
- dev garde `NEWS_FINANCIAL_JUICE_ENABLED=false` pour activer le fallback browser partage.

Le profil Chromium persistant vit dans:

```text
local Docker: volume Docker `financialjuice_browser_data`
production: `./volumes/financialjuice-browser-collector/data`
```

### Intervention manuelle FinancialJuice via NoMachine

La procédure canonique est dans [le README du collector](../financialjuice-browser-collector/README.md#deblocage-challenge-via-nomachine). Elle exige l’autorisation d’intervention sur l’instance privée et l’accès amont autorisé. Elle protège les variables Docker avant extraction avec umask077, un répertoire temporaire700/fichiers600 et un trap ; le cookie Xauthority évite l’ouverture X11 générale.

Ne pas rétablir l’ancien fichier prévisible dans /tmp ni xhost +local:. Les profils Chromium et configurations privées ne doivent pas être copiés dans un export public. Les ports, chemins, UID et image de la procédure restent à vérifier sur l’instance avant exécution. L’échec du collector manuel après arrêt exige une reprise explicite contrôlée ; aucun changement production n’est exécuté par cette documentation.

Le backend Rust ne pilote pas directement ce container Compose permanent. Il expose
uniquement l'etat fonctionnel du fallback via `/api/ingest/financial-juice/browser/control`
et lit les heartbeats du collector via `/api/ingest/financial-juice/browser/status`;
Docker ou `npm start` gerent le cycle de vie du process Node.

Le collector ne charge pas tout `.env.server` en `env_file`: seules les variables
necessaires au navigateur sont transmises au container, afin d'eviter de lui
exposer les secrets Better Auth, PostgreSQL, SMTP, Telegram ou Edge.

## Collecteur permanent FinancialJuice

Le mode recommande reste Docker Compose: le service `financialjuice-browser-collector`
est demarre en permanence, reste idle tant que `fallback_active=false`, puis ouvre
Chromium quand le primaire `financial_juice` est coupe. Quand le primaire est reactive,
le collector permanent ferme Chromium et attend le prochain basculement.

Hors Docker, le meme modele s'applique avec `npm start` depuis
`financialjuice-browser-collector`. Pour un seul backend, celui-ci doit rester
joignable via `NEWS_FINANCIAL_JUICE_BROWSER_BACKEND_URL`, et le token doit
correspondre a `NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN` cote backend. Pour
plusieurs backends, utiliser `NEWS_FINANCIAL_JUICE_BROWSER_TARGETS` et les
variables suffixees par target.

`NEWS_FINANCIAL_JUICE_BROWSER_HEADLESS=false` reste une configuration du collector
permanent. Dans l'image Playwright Docker, l'entrypoint fournit Xvfb automatiquement
quand aucun display n'est disponible.

`FILFINANCE_BACKEND_HOST_BIND` et `FILFINANCE_BACKEND_PORT` publient le gateway
`filfinance-gateway`, pas directement le backend Rust. Le gateway route `/api/auth/*`
et `/api/me` vers `filfinance-auth`, puis toutes les autres routes vers
`filfinance-service`, WebSocket compris. Utiliser `127.0.0.1` quand le proxy d'origine
tourne sur le meme hote. Utiliser `0.0.0.0` uniquement quand Cloudflare atteint
directement le port hote, avec filtrage pare-feu de l'origine. Les services Docker se
parlent toujours sur `filfinance_internal`.

## Calendrier économique Rust natif

Le calendrier est exécuté par le backend Rust. Il collecte séquentiellement
ForexFactory (export JSON), BEA (JSON/ICS), Federal Reserve (FOMC/RSS), Eurostat
(ICS euro-indicateurs) et BCE (calendrier statistique), qualifie les événements via OpenRouter, persiste le
résultat dans SQLite puis réutilise Piper/HLS et les alertes existantes. Ses
files, limites et retries sont dédiés : une lenteur ou une saturation calendrier
ne bloque jamais les news live.

Les huit endpoints sont fermés dans Rust, sans nouvelle variable d'URL ni clé
fournisseur. Les RSS Fed sont seulement du contexte ; seuls les calendriers
peuvent fournir les événements. Cache mémoire borné d'une heure, corps source
jamais persistés. Une date sans heure explicite ne déclenche pas d'alerte T−5/T−1.
Investing et le collecteur HTML ForexFactory sont retires des types, des liens
admin/PWA et du script d'observation ; seules les huit sources fermees sont
acceptees. Les lignes SQLite historiques ne sont pas purgees. Ce changement
de sources est local tant qu'aucune nouvelle release n'a été livrée.

Variables consommées par le backend :

- `FILFINANCE_CALENDAR_DAY_ENABLED`: active l'orchestrateur Rust et requiert alors `OPENROUTER_API_KEY`.
- `FILFINANCE_CALENDAR_DAY_SCHEDULE_TIME=01:30` et `FILFINANCE_CALENDAR_DAY_RETRY_END_TIME=03:00`: fenêtre automatique quotidienne administrée par le scheduler Rust.
- `FILFINANCE_CALENDAR_WEEK_GENERATION_WINDOW_START=02:00` et `FILFINANCE_CALENDAR_WEEK_GENERATION_WINDOW_END=04:00`: fenêtre du samedi qui prépare les jours ouvrés de la semaine ISO suivante.
- `FILFINANCE_CALENDAR_WEEK_VERIFICATION_WINDOW_START=02:00` et `FILFINANCE_CALENDAR_WEEK_VERIFICATION_WINDOW_END=04:00`: fenêtre du dimanche qui vérifie et relance les jours incomplets.
- `FILFINANCE_CALENDAR_DAY_TIMEZONE=Europe/Paris`: seule timezone métier admise.
- `FILFINANCE_CALENDAR_DAY_MAX_RETRIES=12` et `FILFINANCE_CALENDAR_DAY_RETRY_BASE_SECS=1800`: budget et délai des retries du scheduler.
- `FILFINANCE_CALENDAR_ALERT_OFFSETS_SECS=300,60`: alertes audio avant les événements `high` ; une à quatre valeurs entre 1 et 86 400 secondes.
- `FILFINANCE_CALENDAR_NATIVE_REQUEST_TIMEOUT_MS=60000`: timeout de chaque requête OpenRouter, entre 1 et 60 000 ms.
- `FILFINANCE_CALENDAR_NATIVE_RETRY_ATTEMPTS=2` et `FILFINANCE_CALENDAR_NATIVE_RETRY_BACKOFF_MS=500`: retries OpenRouter bornés à trois et backoff borné à cinq secondes.
- `FILFINANCE_CALENDAR_NATIVE_MAX_CONCURRENCY=1`: une seule requête IA calendrier active.
- `FILFINANCE_CALENDAR_NATIVE_QUEUE_CAPACITY=4`: capacité de signalement mémoire dédiée, entre 1 et 16 ; SQLite reste la source de vérité.
- `FILFINANCE_CALENDAR_NATIVE_MAX_RESPONSE_BYTES=1000000`: plafond de réponse OpenRouter, entre 1 et 1 000 000 octets.
- `OPENROUTER_API_URL`: HTTPS obligatoire sauf mock loopback sans credentials, query ni fragment.
- `OPENROUTER_MODEL`: identifiant fournisseur/modèle court et validé ; cible demandée
  `dots-studio/dots-3-note-preview:free`, en JSON Schema strict avec validations
  Rust inchangées. L'exception JSON simple MiniMax est retirée.
  Le migrateur CI remplace les anciens Nemotron et MiniMax M3 free uniquement
  hors DEV, avec sauvegarde du fichier et sans afficher ni modifier la clé API.
- `OPENROUTER_API_KEY`: secret serveur du calendrier, exclu des diagnostics et rapports.
- `NEWS_TTS_PROVIDER=piper_local_session` et `NEWS_TTS_VOICE=fr_FR-upmc-medium-jessica`: configuration Piper utilisée par les clips calendrier.

Les audits de sources visibles dans l'admin ne contiennent que le verdict, le
statut HTTP, le hash et la taille. La provenance de chaque item conserve seulement
la source, son identifiant externe, son URL canonique datée et son titre brut. Le
HTML, les cookies, headers, prompts, réponses IA brutes et JSON fournisseur restent
hors SQLite et hors WebSocket.

Exception explicitement autorisée pour le diagnostic OpenRouter : l'événement
serveur `calendar_openrouter_error_response` conserve le statut HTTP, le code
numérique fournisseur (`provider_code`), son message (`provider_message`) et le
corps d'erreur (`error_body`). Les réponses réussies ne sont pas journalisées.
Le corps de log est limité à 65 536 octets et le message à 4 096 octets avant
échappement ; `truncated`, `captured_bytes` et `captured_sha256` décrivent les
limites. La lecture d'erreur respecte aussi le plafond de réponse configuré et
le timeout existant. Les clés API, credentials reconnus et prompts complets
réfléchis sont masqués ; aucun header de requête n'est ajouté au log. Les champs
détaillés ne rejoignent pas les artefacts calendrier admin, SQLite ou WebSocket.
Ils suivent les accès et la rétention des logs serveur existants, y compris le
viewer de logs administrateur lorsqu'il est activé.

Un refus source est persisté avant un éventuel retry OpenRouter et met cette
source en quarantaine pour le reste du jour. Lors d'une réingestion partielle,
la provenance validée d'une autre source reste conservée dans `sources_json` et
`calendar_day_item_sources`; elle n'est jamais remplacée par un tableau vide.
La réparation HLS d'un résumé passe par `SummaryAudio` durable avec son
`stream_id` réservé, y compris lorsque seul `current.m3u8` manque.

## Service local: `filfinance-service/.env.example`

Emplacement:

```text
filfinance-service/.env.example
```

Utilite:

- template pour lancer le backend Rust en local hors Docker;
- sert a creer `filfinance-service/.env`;
- peut referencer des chemins locaux OPUS-MT et Piper;
- peut activer TTS, Telegram ou providers locaux differents du serveur Docker.

Ce fichier n'est pas necessaire pour le lancement Docker unifie. En Docker local,
la source de configuration est `.env.local`; en production, la source de
configuration est `.env.server`.

Creation locale:

```bash
cd filfinance-service
cp .env.example .env
```

Le fichier reel:

```text
filfinance-service/.env
```

est charge quand on lance le backend depuis le dossier `filfinance-service`.
Le binaire Rust charge les fichiers locaux disponibles sans remplacer les
variables deja exportees par le shell:

1. `FILFINANCE_ENV_FILE` si la variable pointe vers un fichier explicite;
2. `.env` dans le repertoire courant;
3. `.env.local` dans le repertoire courant;
4. `../.env.local` quand `cargo run` est lance depuis `filfinance-service/`.

Cette resolution permet de reutiliser la configuration Docker locale quand le
backend est lance directement en dev. Par exemple:

```bash
cd filfinance-service
FILFINANCE_ENV_FILE=../.env.local cargo run
```

En Docker Compose, `FILFINANCE_ENV_FILE` sert seulement a interpoler le chemin
`env_file` cote Compose. Le conteneur `filfinance-service` remet cette variable
a vide pour eviter que le binaire cherche a rouvrir `.env.local` ou `.env.server`
dans `/app`, ou ces fichiers ne sont pas montes.

Valeurs critiques a remplacer:

- `NEWS_SERVER_ENV=local` pour un lancement hors Docker en dev; `production` refuse un backend sans auth
- `NEWS_FINANCIAL_JUICE_EMAIL`
- `NEWS_FINANCIAL_JUICE_PASSWORD`
- `NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN` si `NEWS_FINANCIAL_JUICE_BROWSER_ENABLED=true`
- `NEWS_X_BEARER_TOKEN` si `NEWS_X_ENABLED=true`
- `NEWS_X_ACCOUNT_USERNAMES` ou `NEWS_X_QUERY` si `NEWS_X_ENABLED=true`
- `NEWS_X_KEYWORDS` si un filtrage par mots cles est souhaite
- `NEWS_TELEGRAM_BOT_TOKEN`
- `NEWS_TELEGRAM_CHAT_IDS`
- `NEWS_EDGE_PUBLISH_URL` si `NEWS_EDGE_PUBLISH_ENABLED=true`
- `NEWS_EDGE_PUBLISH_SECRET` si `NEWS_EDGE_PUBLISH_ENABLED=true`
- `NEWS_EDGE_TICKET_SECRET` si `NEWS_EDGE_TICKET_ENABLED=true`
- `NEWS_HLS_ENABLED`
- `NEWS_HLS_SEGMENTS_DIR`
- `NEWS_HLS_FFMPEG_COMMAND`
- `NEWS_HLS_REPLAY_SEGMENT_TARGET_DURATION_SECS`
- `NEWS_HLS_CLIP_SEGMENT_TARGET_DURATION_SECS`
- `NEWS_HLS_ACCESS_CONTROL_ENABLED`
- `NEWS_HLS_ACCESS_TICKET_SECRET`
- `NEWS_HLS_ACCESS_TICKET_TTL_SECS`
- `NEWS_AUTH_PROVIDER=better_auth`
- `NEWS_AUTH_ISSUER`
- `NEWS_AUTH_JWKS_URL`
- `NEWS_AUTH_AUDIENCE`

Difference avec `.env.server`:

- `filfinance-service/.env` est oriente poste de dev;
- `.env.server` est oriente serveur Docker Compose;
- les deux peuvent avoir les memes familles de variables, mais pas les memes valeurs.

## Client local: `apps/pwa/.env.example`

Emplacement:

```text
apps/pwa/.env.example
```

Utilite:

- template pour lancer le client Expo en local;
- sert a creer `apps/pwa/.env`;
- contient uniquement des variables publiques `EXPO_PUBLIC_*`.

Creation locale:

```bash
cd apps/pwa
cp .env.example .env
```

Variables:

- `EXPO_PUBLIC_FILFINANCE_WS_URL`: URL WebSocket news, par exemple `ws://localhost:8088/ws/news` en local ou `wss://api.filfinance.fr/ws/news` en prod;
- `EXPO_PUBLIC_FILFINANCE_EDGE_WS_URL`: URL WebSocket metadata edge, vide par defaut pour conserver `/ws/news` backend;
- `EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL`: base HLS alternative. En production elle pointe vers Bunny `https://media.filfinance.fr`; en dev elle pointe vers Bunny via `FILFINANCE_DEV_HLS_BASE_URL`. Ne pas pointer vers `live.filfinance.fr` depuis `EPIC-034`;
- `EXPO_PUBLIC_FILFINANCE_PWA_URL`: origine HTTPS canonique de la PWA du meme environnement; elle sert de retour sur mobile pour les emails d'activation et de reinitialisation sans exposer de cookie ou de token dans `filfinance://`;
- `EXPO_PUBLIC_LANDING_ORIGIN`: origine des quatre documents légaux, `http://localhost:4321` en local, `https://dev.filfinance.fr` en DEV et `https://filfinance.fr` en PROD ; requise au prébuild et contrôlée dans l’artefact.
- `EXPO_PUBLIC_API_URL`: URL publique unique de l'API;
- `EXPO_PUBLIC_BETTER_AUTH_URL`: meme origine API, qui expose `/api/auth/*`.

Ces variables sont publiques dans le bundle Web. Elles ne doivent jamais contenir de secret serveur.

## Bunny Storage/Pull Zones production

GitLab construit les artefacts puis les synchronise dans les Storage Zones Bunny.
Cloudflare conserve le DNS : les CNAME `@` et `app` sont DNS-only vers les Pull
Zones, tandis que `api`, `live`, `media` et la messagerie restent inchanges.

| Environnement | Surface | Bunny | Domaine | Artefact GitLab |
| --- | --- | --- | --- | --- |
| PROD | Landing Astro | `filfinance-prod-landing` | `https://filfinance.fr` | `apps/landing/dist` |
| PROD | PWA Expo Web | `filfinance-prod-pwa` | `https://app.filfinance.fr` | `apps/pwa/dist` |
| DEV | Landing Astro | Bunny Storage | `https://dev.filfinance.fr` | `apps/landing/dist` |
| DEV | PWA Expo Web | Bunny Storage | `https://app.dev.filfinance.fr` | `apps/pwa/dist` |

`FILFINANCE_DEV_PWA_URL` est fixe a `https://app.dev.filfinance.fr`; elle ne
doit pas etre remplacee par une valeur implicite dans le depot. Un domaine
un hôte tiers est cross-site avec `api.dev.filfinance.fr` et ne permet pas de
valider de facon fiable le cookie Better Auth `SameSite=Lax` dans le navigateur.

Cloudflare doit lever la politique `banned domain` avant toute association des
domaines PWA. Ne pas essayer une variante de hostname, un CNAME seul ou une
recreation de projet pour contourner ce refus. DEV reste entierement en pause :
ne publier ni artefact, ni DNS, ni backend, Worker ou smoke DEV avant une
decision de reprise explicite.

Les origines publiques canoniques sont fixees par branche dans `.gitlab-ci.yml`,
injectees avant `expo export`, puis l'artefact `dist` est envoye par Direct Upload.
Ne pas definir un override `EXPO_PUBLIC_*` global qui pourrait melanger dev et prod.
Contrat production:

```text
EXPO_PUBLIC_FILFINANCE_WS_URL=wss://api.filfinance.fr/ws/news
EXPO_PUBLIC_FILFINANCE_EDGE_WS_URL=wss://live.filfinance.fr/ws/live
EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL=https://media.filfinance.fr
EXPO_PUBLIC_FILFINANCE_PWA_URL=https://app.filfinance.fr
EXPO_PUBLIC_LANDING_ORIGIN=https://filfinance.fr
EXPO_PUBLIC_API_URL=https://api.filfinance.fr
EXPO_PUBLIC_BETTER_AUTH_URL=https://api.filfinance.fr
```

Ces variables ne sont lues ni depuis `.env.server` ni depuis le dashboard Bunny.
Dans Bunny, relier chaque Pull Zone a sa Storage Zone, ajouter le hostname et
activer Free SSL; la CI est l'unique emetteur des fichiers. Voir
[BUNNY-PRODUCTION-RUNBOOK](./BUNNY-PRODUCTION-RUNBOOK.md).
`live.filfinance.fr` sert le Worker Edge metadata uniquement; le HLS Cloudflare standard
est retire par `EPIC-034`.

Variables canoniques Bunny CDN injectees par la CI:

```text
FILFINANCE_PROD_HLS_BASE_URL=https://media.filfinance.fr
FILFINANCE_DEV_HLS_BASE_URL=https://media.dev.filfinance.fr
```

L'ancien acces direct `filfinancedevhls.b-cdn.net` est une etape de migration
historique. Supprimer tout override GitLab restant: le contrat de build exige le
custom domain `https://media.dev.filfinance.fr`.

Les scripts de charge HLS acceptent `*.b-cdn.net` par defaut. Pour le custom domain
Bunny DNS-only, ajouter explicitement:

```text
FILFINANCE_MEDIA_LOAD_TEST_ALLOWED_HOSTS=media.filfinance.fr,media.dev.filfinance.fr
```

`FILFINANCE_ALLOW_CLOUDFLARE_MEDIA_LOAD_TEST` reste reserve aux tests Cloudflare valides;
ne pas l'utiliser pour Bunny.

Prerequis backend prod/dev pour que Bunny ait un origin HLS:

```text
NEWS_TTS_ENABLED=true
NEWS_HLS_ENABLED=true
NEWS_HLS_ACCESS_CONTROL_ENABLED=true
NEWS_HLS_ACCESS_TICKET_SECRET=<secret aleatoire de 32 caracteres minimum>
NEWS_HLS_ACCESS_TICKET_TTL_SECS=3600
```

Ces valeurs sont a appliquer dans `.env.server.prod` et `.env.server.dev` sur le serveur,
pas dans les variables publiques Expo. Un `404` Bunny avec `hls replay origin is disabled`
confirme que Bunny atteint bien l'origin API, mais que l'origin HLS backend reste desactive.

## Profils utilisateur

Les profils et preferences sont stockes dans PostgreSQL `app.profiles` par
`filfinance-auth`. Le role vient de Better Auth et est signe dans le JWT court.
Le backend Rust ne lit plus la table SQLite historique `user_profiles`.

Configuration Bunny attendue en production :

- une Storage Zone et une Pull Zone distinctes par surface;
- `filfinance.fr` sur la Pull Zone landing, `app.filfinance.fr` sur la Pull Zone
  PWA, tous deux en Free SSL;
- fallback `/index.html` HTTP 200 uniquement pour la PWA;
- Edge Rules Bunny pour les en-tetes et les caches; aucun build Bunny;
- publication par les jobs GitLab depuis `main`.

Les CNAME production attendus sont les hostnames `*.b-cdn.net` attribues par
les deux Pull Zones. Les deux records frontend restent DNS-only dans Cloudflare.

Variables GitLab du contrat Bunny/Edge :

- `CLOUDFLARE_ZONE_ID`, identifiant non secret de la zone `filfinance.fr`;
- `FILFINANCE_PROD_PWA_URL=https://app.filfinance.fr` obligatoire, sans slash final;
- `BUNNY_API_KEY_PROD`, `BUNNY_STORAGE_ZONE_PROD_LANDING`,
  `BUNNY_STORAGE_PASSWORD_PROD_LANDING`, `BUNNY_PULL_ZONE_ID_PROD_LANDING`;
- `BUNNY_STORAGE_ZONE_PROD_PWA`, `BUNNY_STORAGE_PASSWORD_PROD_PWA`,
  `BUNNY_PULL_ZONE_ID_PROD_PWA`, `FILFINANCE_PROD_BUNNY_DEPLOY_ENABLED=true`;
- `CLOUDFLARE_API_TOKEN_WORKERS_PROD`.

Les contrats DEV, conserves pour sa reprise explicite, sont
`PAGES_LANDING_PROJECT_DEV=filfinance-dev`,
`PAGES_PWA_PROJECT_DEV=filfinance-app-dev` et
`FILFINANCE_DEV_PWA_URL=https://app.dev.filfinance.fr`.

Les tokens Pages doivent disposer de `Cloudflare Pages: Edit` sur le compte et
de `DNS: Read` sur la zone. Le preflight full-stack lit l'etat du domaine Pages
et le CNAME cible avant la premiere mutation backend; `CLOUDFLARE_ZONE_ID` peut etre
scope `*` ou duplique dans `development/preflight` et `production/preflight`.
L'API prouve l'acces aux ressources mais n'expose pas la policy du token courant:
le droit `Edit` est controle dans le dashboard puis exerce par Wrangler. Les
tokens Workers doivent etre des Account API tokens et exigent de meme
`Workers Scripts: Edit` sur le compte exact.

La CI exige aussi `FILFINANCE_GITLAB_READ_API_TOKEN` (role `Maintainer`, scope
`read_api`, masque/protege; ce role est requis pour lire les resource groups),
le resource group `production` en mode `oldest_first`, et les six credentials
smoke listener/moderator/admin documentes dans `AUTH-RUNBOOK.md`.

Les valeurs PROD lues lors de l'evaluation des `rules` ou par les builds sans
`environment` restent `protected` et scopees `*` ou `production/*`. Le token
GitLab en lecture peut rester scope `*` ou `production/preflight`; les noms de
projets Pages, tokens Cloudflare et credentials smoke suivent les scopes PROD.
Les variables legacy `PAGES_PROJECT_PROD` et `PAGES_PWA_PROJECT` doivent etre
absentes ou vides : la preflight les refuse sinon. Les noms de projets DEV et
`FILFINANCE_DEV_PWA_URL` restent des contrats de configuration en pause et ne
sont jamais utilises par une release PROD.

### Archive — variables DEV en pause

`PAGES_LANDING_PROJECT_DEV=filfinance-dev`,
`PAGES_PWA_PROJECT_DEV=filfinance-app-dev`,
`FILFINANCE_DEV_PWA_URL=https://app.dev.filfinance.fr`,
`CLOUDFLARE_API_TOKEN_PAGES_DEV`,
`CLOUDFLARE_API_TOKEN_WORKERS_DEV`,
`FILFINANCE_DEV_PAGES_BOOTSTRAP_ENABLED` et
`FILFINANCE_DEV_WORKER_BOOTSTRAP_ENABLED` sont conserves uniquement pour une
remise en service DEV explicitement decidee. Elles ne sont ni requises ni lues
par la release PROD EPIC-037, et les tokens restent distincts sans fallback.

Le pilote DEV utilise une sous-zone Bunny déléguée: `dev.filfinance.fr` et ses
hôtes `app`, `api`, `media`, `live` et `origin`. Les anciennes ressources Pages
DEV et leurs CNAME ne sont plus des contrats de déploiement.

`APP_WEB_URL` est une variable serveur Better Auth, pas une variable de CDN : elle
doit pointer vers la PWA (`https://app.filfinance.fr` en prod et
`FILFINANCE_DEV_PWA_URL` en dev), puis etre autorisee dans
`BETTER_AUTH_TRUSTED_ORIGINS`.

Ne pas utiliser `npx wrangler deploy` pour le frontend Bunny.

Les projets Pages legacy ne peuvent etre supprimes qu'apres activation Bunny,
smokes valides, sauvegarde des CNAME et confirmation explicite. La procedure de
nettoyage et son rollback sont dans `BUNNY-PRODUCTION-RUNBOOK.md`.

## Validation des cles

Comparer les cles du backend local:

```bash
diff -u \
  <(awk -F= '/^[A-Za-z_][A-Za-z0-9_]*=/{print $1}' filfinance-service/.env) \
  <(awk -F= '/^[A-Za-z_][A-Za-z0-9_]*=/{print $1}' filfinance-service/.env.example)
```

Comparer les cles du client local:

```bash
diff -u \
  <(awk -F= '/^[A-Za-z_][A-Za-z0-9_]*=/{print $1}' apps/pwa/.env) \
  <(awk -F= '/^[A-Za-z_][A-Za-z0-9_]*=/{print $1}' apps/pwa/.env.example)
```

Verifier que le compose serveur sait lire l'exemple:

```bash
FILFINANCE_ENV_FILE=.env.server.example docker compose --env-file .env.server.example -f compose.yaml -f compose.prod.yaml config
```

## Regles Git

`.gitignore` ignore:

- `.env`
- `.env.*`

et reautorise explicitement:

- `.env.example`
- `.env.server.example`
- `.env.local.example`

Avant commit, verifier:

```bash
git status --short
git check-ignore .env.server filfinance-service/.env apps/pwa/.env
```

## Informations publiques des pages légales

Les cinq variables `PUBLIC_LEGAL_ENTITY_NAME`, `PUBLIC_LEGAL_REGISTERED_OFFICE`,
`PUBLIC_LEGAL_PUBLICATION_DIRECTOR`, `PUBLIC_LEGAL_PRIVACY_CONTACT` et
`PUBLIC_LEGAL_DATA_RETENTION` sont publiques. Les valeurs approuvées figurent dans
`apps/landing/src/config/legal.config.ts`, `apps/landing/.env.example` et les
variables du YAML CI. La priorité habituelle des variables GitLab s’applique.

La landing et son validateur utilisent les variables **exportées dans le shell** ;
ne pas compter sur une lecture implicite du fichier `.env.example`. Exporter les
mêmes valeurs pour le build et la validation. `PUBLIC_LEGAL_PUBLICATION_ENFORCED=true`
active le refus des valeurs manquantes/provisoires ; ce mode est imposé dans
`landing:check`. Aucun secret ne doit être placé dans ces variables.

La PWA charge ses `.env` selon les règles Expo. Ajouter
`EXPO_PUBLIC_LANDING_ORIGIN=http://localhost:4321` au fichier local utilisé ;
`compose.local.yaml` fournit déjà ce défaut au build et au serveur statique.
Voir la [procédure et la recette](./guides/legal-landing-app/RECETTE-FILFINANCE.md).
