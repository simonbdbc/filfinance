# NEWS-154 - Collector navigateur FinancialJuice MVP

## Objectif

Permettre de continuer le developpement MVP et la finalisation v1 meme si `FinancialJuice` exige une verification navigateur que le backend Rust `reqwest` ne peut pas satisfaire.

## Scope

- Creer un composant separe du backend Rust principal, base sur `Playwright`/`Chromium`.
- Ouvrir `FinancialJuice` comme un vrai navigateur.
- Extraire les evenements `news` et, si necessaire, `calendar`.
- Publier les evenements vers une entree backend stable.
- Garder le pipeline Rust canonique pour normalisation, persistence, traduction, audio et diffusion.
- Exposer un statut source explicite: ok, blocked, degraded, stopped.

## Hors scope

- Pas de lecture automatique des profils Brave/Chrome personnels.
- Pas de decryption de cookies utilisateur.
- Pas de rotation d'IP.
- Pas de contournement automatise de challenge Cloudflare.
- Pas de melange `Playwright` dans `filfinance-service/src`.
- Pas de decision de production long terme dans ce ticket.

## Approche technique

Structure retenue:

```text
financialjuice-browser-collector/
  Dockerfile
  .dockerignore
  package.json
  .env.example
  README.md
  src/
    config.js
    browser.js
    session.js
    extract-news.js
    publish.js
    main.js
  test/
    config.test.js
    extract-news.test.js
```

Contrat de sortie vers le backend:

```json
{
  "events": [
    {
      "external_id": "optional-source-id",
      "title": "Headline text",
      "description": "Optional detail",
      "url": "https://www.financialjuice.com/news/example",
      "published_at": "2026-05-06T12:00:00Z",
      "news_level": "active active-critical"
    }
  ]
}
```

Endpoints backend ajoutes:

```text
POST /api/ingest/financial-juice/browser/news
POST /api/ingest/financial-juice/browser/status
GET  /api/ingest/financial-juice/browser/control
GET  /api/runtime/financial-juice-browser
```

Les deux endpoints `POST` utilisent `Authorization: Bearer <NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN>`.
Ils ne dependent pas d'une session Supabase utilisateur, car ils sont appeles par un process machine-to-machine separe.

## Regle fallback

Le collecteur Rust classique reste prioritaire:

```text
NEWS_FINANCIAL_JUICE_ENABLED=true
NEWS_FINANCIAL_JUICE_BROWSER_ENABLED=true
```

Dans cette configuration, `financial_juice_browser` est seulement arme. Le process navigateur appelle
`GET /api/ingest/financial-juice/browser/control` avant d'ouvrir Chromium.

- si `financial_juice` est actif, `fallback_active=false` et le navigateur reste en veille;
- si `financial_juice` est desactive manuellement ou automatiquement apres challenge Cloudflare,
  `fallback_active=true` et le navigateur commence a collecter;
- le backend refuse aussi l'injection browser tant que `financial_juice` est actif, pour eviter deux sources concurrentes.

Le backend Rust doit rester responsable de:

- dedup;
- mapping vers `NewsEvent`;
- persistence SQLite;
- traduction;
- queue audio;
- WebSocket;
- Telegram;
- retention J/J-1.

## Mode Docker local/prod

Le fonctionnement retenu pour garder le meme comportement partout est un container
`financialjuice-browser-collector` separe, demarre par Docker Compose avec le backend.

```text
Docker Compose
  -> demarre filfinance-service
  -> demarre financialjuice-browser-collector
  -> demarre filfinance-client seulement en local

filfinance-service
  -> decide si financial_juice classique est actif
  -> expose fallback_active via /api/ingest/financial-juice/browser/control

financialjuice-browser-collector
  -> reste idle si fallback_active=false
  -> ouvre Chromium si fallback_active=true
  -> ferme Chromium quand fallback_active repasse a false
```

Le backend Rust ne lance pas Docker et n'a pas acces a `/var/run/docker.sock`.
Docker gere le cycle de vie container; Rust gere l'etat applicatif.

Commandes retenues:

```bash
# Local complet: backend + collector + frontend
docker compose --env-file .env.local -f compose.yaml -f compose.local.yaml up --build

# Production: backend + collector, frontend externe
docker compose --env-file .env.server -f compose.yaml -f compose.prod.yaml up -d --build
```

Lancement local du collecteur permanent hors Docker:

```bash
cd financialjuice-browser-collector
FILFINANCE_ENV_FILE=../.env.local \
NEWS_FINANCIAL_JUICE_BROWSER_BACKEND_URL=http://127.0.0.1:8088 \
NEWS_FINANCIAL_JUICE_BROWSER_USER_DATA_DIR=/Users/simon/.filfinance/financialjuice-browser-profile \
npm start
```

Ce mode appelle le backend comme le container Compose: il reste idle tant que
`fallback_active=false`, puis ouvre Chromium avec le profil persistant quand le fallback
navigateur devient actif.
En local, `compose.local.yaml` monte par defaut
`${HOME}/.filfinance/financialjuice-browser-profile` vers
`/app/data/financialjuice-browser-profile`; `NEWS_FINANCIAL_JUICE_BROWSER_PROFILE_DIR`
permet de choisir un autre dossier host.

Si le profil macOS n'est pas accepte par Chromium Linux Docker, lancer seulement backend +
frontend en Docker puis le collecteur sur le host macOS:

```bash
docker compose --env-file .env.local -f compose.yaml -f compose.local.yaml up --build --scale financialjuice-browser-collector=0

cd financialjuice-browser-collector
FILFINANCE_ENV_FILE=../.env.local \
NEWS_FINANCIAL_JUICE_BROWSER_BACKEND_URL=http://127.0.0.1:8088 \
NEWS_FINANCIAL_JUICE_BROWSER_USER_DATA_DIR=/Users/simon/.filfinance/financialjuice-browser-profile \
npm start
```

Le Dockerfile backend est autonome. Il n'y a pas d'image
`filfinance-service-runtime-base:bookworm` a construire manuellement avant Compose.

## Variables

Backend:

- `NEWS_FINANCIAL_JUICE_BROWSER_ENABLED`
- `NEWS_FINANCIAL_JUICE_BROWSER_HOME_URL`
- `NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN`
- `NEWS_FINANCIAL_JUICE_BROWSER_POLL_INTERVAL_MS`
- `NEWS_FINANCIAL_JUICE_BROWSER_REQUEST_TIMEOUT_MS`
- `NEWS_FINANCIAL_JUICE_BROWSER_RATE_LIMIT_BACKOFF_MS`
- `NEWS_FINANCIAL_JUICE_BROWSER_MAX_BATCH_SIZE`

Collecteur navigateur:

- `NEWS_FINANCIAL_JUICE_BROWSER_ENABLED`
- `NEWS_FINANCIAL_JUICE_BROWSER_BACKEND_URL`
- `NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN`
- `NEWS_FINANCIAL_JUICE_BROWSER_HOME_URL`
- `NEWS_FINANCIAL_JUICE_BROWSER_HEADLESS`
- `NEWS_FINANCIAL_JUICE_BROWSER_USER_DATA_DIR`
- `NEWS_FINANCIAL_JUICE_BROWSER_RATE_LIMIT_BACKOFF_MS`
- `NEWS_FINANCIAL_JUICE_EMAIL`
- `NEWS_FINANCIAL_JUICE_PASSWORD`

## Etat d'implementation

- Backend: entree d'ingestion batch et statut runtime ajoutes.
- Backend: mapping vers `NewsEvent` avec source `financial_juice_browser`.
- Backend: aucun package Playwright ajoute au crate Rust.
- Collecteur: composant Node/Playwright separe, extraction DOM MVP, detection de challenge, publication batch et status.
- Docker: service Compose separe, toujours demarre, avec profil Chromium persistant.
- Docker local: `compose.yaml` + `compose.local.yaml` lance backend, collector et frontend.
- Docker prod: `compose.yaml` + `compose.prod.yaml` lance backend et collector, le frontend reste externe.
- Tests: `cargo test` couvre le mapping backend et le runtime; `npm run check` couvre config/extraction du collecteur.

## Criteres d'acceptation

- Le composant est dans un dossier separe du crate Rust.
- Le backend peut tourner sans le composant navigateur.
- Le composant peut etre arrete sans casser `/health`, `/api/news`, `/ws/news` ou `/ws/test-runs`.
- Les evenements injectes suivent le meme pipeline que les autres sources.
- Les erreurs Cloudflare ou session sont visibles dans les logs/status.
- Aucune dependance `Playwright` n'est ajoutee au backend Rust principal.

## Tests attendus

- Test local avec `NEWS_FINANCIAL_JUICE_ENABLED=false` cote collecteur Rust historique.
- Injection d'une news navigateur vers le backend.
- Verification `/api/news`.
- Verification `/api/translations`.
- Verification emission `audio_ready` si la news est eligible.
- Verification que le collector navigateur peut etre stoppe puis relance.

## Dependances

- `EPIC-025`
- Pipeline ingestion Rust existant
- Queue audio `EPIC-024` si elle est implementee avant ce ticket
