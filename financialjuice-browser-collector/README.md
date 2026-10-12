# FinancialJuice Browser Collector

Collecteur navigateur MVP pour `NEWS-154`.

Ce composant est volontairement separe du backend Rust. Il ouvre FinancialJuice avec Playwright/Chromium, extrait les headlines visibles, puis les publie vers l'entree backend machine-to-machine:

```text
POST /api/ingest/financial-juice/browser/news
POST /api/ingest/financial-juice/browser/status
```

Le backend Rust conserve la responsabilite de la deduplication, de SQLite, de la traduction, de l'audio, des WebSockets et de Telegram.
Un meme collector peut publier vers plusieurs backends FilFinance quand la prod
heberge le navigateur partage et que la dev consomme le meme flux fallback.

## Mode fallback

Le collecteur navigateur n'est pas une source parallele par defaut.

Le fonctionnement cible est:

```text
financial_juice actif
  -> financialjuice-browser-collector reste en veille

financial_juice desactive
  -> financialjuice-browser-collector ouvre Chromium et publie les news
```

Concretement, le collector appelle d'abord:

```text
GET /api/ingest/financial-juice/browser/control
```

Si `fallback_active=false`, il n'ouvre pas Chromium. Si `fallback_active=true`, il lance la session navigateur et commence l'extraction.
En mode multi-target, Chromium est ouvert si au moins un backend renvoie
`fallback_active=true`. Les statuts sont envoyes a tous les backends, mais les
news ne sont envoyees qu'aux backends dont le fallback est actif.

## Lancement Docker recommande

Le mode recommande est le meme en local Docker et en prod: le container reste demarre,
mais n'ouvre Chromium que lorsque le backend Rust expose `fallback_active=true`.

Local complet, avec frontend Docker:

```bash
cp .env.local.example .env.local
# Completer .env.local.
docker compose --env-file .env.local -f compose.yaml -f compose.local.yaml up --build
```

Production backend + collector, frontend externe:

```bash
cp .env.server.example .env.server
# Completer .env.server.
docker compose --env-file .env.server -f compose.yaml -f compose.prod.yaml up -d --build
```

Le Dockerfile backend est autonome: il construit la base runtime OPUS-MT/Piper
dans un stage interne. Il n'y a plus d'image `filfinance-service-runtime-base:bookworm`
a construire avant `docker compose`.

Dans ce mode:

- Docker gere le cycle de vie du process Node/Playwright;
- le backend Rust gere l'etat fonctionnel du fallback;
- le collector ferme Chromium quand `fallback_active=false`;
- le collector rouvre Chromium automatiquement quand `fallback_active=true`.

Le backend et le collector doivent partager le meme `NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN`.
Pour un collector prod partage avec dev, configurer des targets nommees:

```env
NEWS_FINANCIAL_JUICE_BROWSER_TARGETS=prod,dev
NEWS_FINANCIAL_JUICE_BROWSER_BACKEND_URL_PROD=http://filfinance-service:8088
NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN_PROD=prod-browser-collector-token
NEWS_FINANCIAL_JUICE_BROWSER_BACKEND_URL_DEV=http://host.docker.internal:18088
NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN_DEV=dev-browser-collector-token
```

Les variables historiques `NEWS_FINANCIAL_JUICE_BROWSER_BACKEND_URL` et
`NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN` restent supportees pour un seul
backend.

## Lancement local hors Docker

```bash
cd financialjuice-browser-collector
npm install
npx playwright install chromium
NEWS_FINANCIAL_JUICE_BROWSER_ENABLED=true npm start
```

Variables minimales:

- `NEWS_FINANCIAL_JUICE_BROWSER_BACKEND_URL`
- `NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN`
- `NEWS_FINANCIAL_JUICE_BROWSER_TARGETS` et les paires
  `NEWS_FINANCIAL_JUICE_BROWSER_BACKEND_URL_<TARGET>` /
  `NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN_<TARGET>` pour le fanout
- `NEWS_FINANCIAL_JUICE_EMAIL`
- `NEWS_FINANCIAL_JUICE_PASSWORD`

Le meme token doit etre configure cote backend avec `NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN`.
Pour armer le fallback sans remplacer la source primaire, garder:

```env
NEWS_FINANCIAL_JUICE_ENABLED=true
NEWS_FINANCIAL_JUICE_BROWSER_ENABLED=true
```

Le navigateur ne deviendra actif que si le runtime backend `financial_juice` passe a `enabled=false`, manuellement ou apres desactivation automatique.

## Deblocage challenge via NoMachine

Si le statut indique `FinancialJuice page is blocked by a browser challenge`, valider le
challenge depuis le bureau NoMachine du serveur avec l'utilisateur `web-srv`. Le profil
Chromium a utiliser est le profil persistant du collector prod; ne pas utiliser un profil
local different.

Pendant le deblocage manuel, garder `financialjuice_browser_collector_prod` arrete.
Si le collector permanent reste actif avec une ancienne configuration
`NEWS_FINANCIAL_JUICE_BROWSER_HEADLESS=true`, il peut republier un statut `blocked`
et masquer les publications reussies du collector manuel. Le pipeline normalise
maintenant le collector permanent en `NEWS_FINANCIAL_JUICE_BROWSER_HEADLESS=false`.

Cette procédure concerne un service privé déjà configuré et un accès amont
explicitement autorisé. Elle ne donne aucune permission d’ingestion ou de
redistribution FinancialJuice. Avant toute intervention de production, confirmer
le serveur, les chemins, le compte opérateur et l’autorisation d’arrêt/reprise.
Ne jamais publier le profil Chromium, l’environnement ou les diagnostics bruts.

Le compte opérateur doit avoir accès à Docker et à son bureau X11, avec `xauth`
disponible. Le conteneur doit pouvoir lire le cookie monté et le profil persistant ;
si ses UID/permissions ne le permettent pas, arrêter et corriger cette configuration
privée, sans ouvrir X11 à tous les utilisateurs locaux.

La commande suivante crée les fichiers protégés **avant** d’y écrire les variables
privées. Le cookie X11 authentifie uniquement cette session ; aucune commande
`xhost +local:` n’est nécessaire. Exécuter le bloc dans un même sous-shell, sans
`set -x`, après validation des chemins et de l’image installée :

```bash
(
  set -euo pipefail
  umask 077
  : "${DISPLAY:?Un bureau X11 autorisé est nécessaire}"
  cd /raid/app/filfinance/filfinance-prod/financialjuice-browser-collector

  manual_dir="$(mktemp -d "${TMPDIR:-/tmp}/financialjuice-manual.XXXXXX")"
  chmod 700 "$manual_dir"
  manual_env="$manual_dir/collector.env"
  manual_xauth="$manual_dir/Xauthority"
  : > "$manual_env"
  : > "$manual_xauth"
  chmod 600 "$manual_env" "$manual_xauth"
  cleanup() {
    rm -f -- "$manual_env" "$manual_xauth"
    rmdir -- "$manual_dir"
  }
  trap cleanup EXIT

  xauth nlist "$DISPLAY" | sed 's/^..../ffff/' | xauth -f "$manual_xauth" nmerge -
  test -s "$manual_xauth"

  docker inspect financialjuice_browser_collector_prod \
    --format '{{range .Config.Env}}{{println .}}{{end}}' \
    | grep -E '^(NEWS_FINANCIAL_JUICE_BROWSER_|NEWS_FINANCIAL_JUICE_EMAIL=|NEWS_FINANCIAL_JUICE_PASSWORD=)' \
    | grep -vE '^(NEWS_FINANCIAL_JUICE_BROWSER_BACKEND_URL_PROD|NEWS_FINANCIAL_JUICE_BROWSER_BACKEND_URL_DEV|NEWS_FINANCIAL_JUICE_BROWSER_HEADLESS|NEWS_FINANCIAL_JUICE_BROWSER_USER_DATA_DIR)=' \
    > "$manual_env"

  cat >> "$manual_env" <<'ENV'
NEWS_FINANCIAL_JUICE_BROWSER_BACKEND_URL_PROD=http://127.0.0.1:8088
NEWS_FINANCIAL_JUICE_BROWSER_BACKEND_URL_DEV=http://127.0.0.1:18088
NEWS_FINANCIAL_JUICE_BROWSER_HEADLESS=false
NEWS_FINANCIAL_JUICE_BROWSER_USER_DATA_DIR=/app/data/financialjuice-browser-profile
ENV

  docker stop financialjuice_browser_collector_prod
  docker run --rm -it \
    --name financialjuice_browser_manual_unlock \
    --network host \
    --shm-size=1g \
    --env-file "$manual_env" \
    -e DISPLAY="$DISPLAY" \
    -e XAUTHORITY=/tmp/manual.xauthority \
    -v "$manual_xauth:/tmp/manual.xauthority:ro" \
    -v /tmp/.X11-unix:/tmp/.X11-unix \
    -v /raid/app/filfinance/filfinance-prod/volumes/financialjuice-browser-collector/data:/app/data \
    filfinance_prod-financialjuice-browser-collector \
    npm start

  docker start financialjuice_browser_collector_prod
)
```

Dans Chromium, utiliser uniquement l’accès autorisé et fermer le process manuel
quand la session est prête. Après reprise, vérifier le statut et `headless=false`
sans partager de logs bruts ni de données du feed. Le trap détruit les copies
temporaires même si une étape échoue. Si le conteneur manuel échoue après l’arrêt,
le collector permanent reste arrêté : vérifier l’état/configuration connus avant
sa reprise explicite. Ne pas contourner une erreur X11 par une ouverture générale.

## Statuts

- `ok`: extraction et publication operationnelles.
- `blocked`: login, session, challenge Cloudflare ou acces amont bloquant.
- `degraded`: le navigateur tourne mais aucune news exploitable n'est extraite.
- `stopped`: collecteur desactive ou arrete proprement.

Ce collector ne contourne pas Cloudflare, ne lit pas les profils Brave/Chrome personnels et ne decrypte aucun cookie utilisateur.
