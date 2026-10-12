# NEWS-113 - Bootstrap serveur et premier deploy backend

> **Archive historique — ne pas executer.** Les chemins, domaines, services et
> variables de ce bootstrap sont supersedes. Utiliser
> [le runbook GitLab actuel](../GITLAB-RUNBOOK.md) et
> [les fichiers d'environnement actuels](../ENVIRONMENT-FILES.md).

## Objectif

Preparer le premier deploiement backend Fil Finance sur le serveur personnel `home-srv`.

Le frontend reste heberge sur Cloudflare Pages. Ce ticket ne deploie donc que le service Rust `filfinance_service`.

Voir aussi [Fichiers d'environnement](../ENVIRONMENT-FILES.md) pour le role exact de `.env.server`, `filfinance-service/.env` et `filfinance-client/.env`.

## Statut

Prepare cote repo le 2 mai 2026.

Execution serveur et GitLab validee en live le 2 mai 2026.

Les erreurs runtime profil/historique observees apres le premier deploiement ont ete resolues.

## Prerequis

- La route Traefik API `filfinance-api.sbcenter.fr` existe dans `home-srv/traefik-docker`.
- Le DNS `filfinance-api.sbcenter.fr` pointe vers le serveur personnel.
- Le reseau Docker externe `traefik_network` existe sur le serveur.
- Le repo GitLab `git@gitlab.com:simonbdbc/filfinance.git` est accessible depuis le serveur.
- Les secrets Supabase et FinancialJuice sont disponibles.

Verifier le reseau Docker sur le serveur:

```bash
docker network inspect traefik_network
```

## Bootstrap du dossier serveur

Chemin de deploiement retenu: `/home/traefik/filfinance-docker`.

```bash
cd /home/traefik
git clone git@gitlab.com:simonbdbc/filfinance.git filfinance-docker
cd filfinance-docker
git checkout main
```

Creer le fichier d'environnement serveur:

```bash
ls -la .env.server.example
cp .env.server.example .env.server
chmod 600 .env.server
```

Puis renseigner au minimum:

```env
NEWS_SUPABASE_PROJECT_URL=https://project-ref.supabase.co
NEWS_FINANCIAL_JUICE_EMAIL=...
NEWS_FINANCIAL_JUICE_PASSWORD=...
```

Le premier deploiement sans assets modele OPUS-MT garde volontairement:

```env
NEWS_TRANSLATION_PROVIDER=heuristic_fr
NEWS_TRANSLATION_COMMAND=
NEWS_TRANSLATION_COMMAND_ARGS=
NEWS_TTS_ENABLED=false
```

Raison: le premier deploy doit rester valide avant le bootstrap des voix Piper. Depuis `NEWS-125`, l'image Docker contient le runtime Python/Piper; depuis `NEWS-126`, elle contient aussi le runtime Python OPUS-MT et installe le modele CTranslate2 dans l'image pendant le build. Les assets voix Piper restent a installer dans le volume serveur avant d'activer `NEWS_TTS_ENABLED=true`, mais `NEWS_TRANSLATION_PROVIDER=opus_mt_local_session` ne demande plus de copie manuelle du modele OPUS-MT.

## Variables GitLab CI/CD

Dans `Settings > CI/CD > Variables`, configurer:

```text
SSH_PRIVATE_KEY
USER
HOST
PORT
DEPLOY_PATH
```

Valeur attendue de `DEPLOY_PATH` si le chemin ci-dessus est retenu:

```text
/home/traefik/filfinance-docker
```

`SSH_PRIVATE_KEY` doit contenir la cle privee de deploiement encodee en base64, sur une seule ligne.

Commande macOS utile:

```bash
base64 -i ~/.ssh/id_rsa_home_srv_gitlab_ci | tr -d '\n'
```

La cle publique correspondante doit etre autorisee sur le serveur dans `~/.ssh/authorized_keys` de l'utilisateur de deploiement.

## Validation manuelle avant CI

Sur le serveur:

```bash
cd /home/traefik/filfinance-docker
docker compose --env-file .env.server -f compose.yaml -f compose.prod.yaml config
docker compose --env-file .env.server -f compose.yaml -f compose.prod.yaml up -d --build --force-recreate
docker compose --env-file .env.server -f compose.yaml -f compose.prod.yaml ps
```

Verifier le healthcheck local:

```bash
docker compose --env-file .env.server -f compose.yaml -f compose.prod.yaml exec filfinance-service curl -fsS http://127.0.0.1:8088/health
```

Verifier le routage Traefik public:

```bash
curl -fsS https://filfinance-api.sbcenter.fr/health
```

## Deploiement GitLab

Apres validation manuelle, le job `deploy:home-srv` execute sur `main`:

```bash
echo 'Deploying Fil Finance backend'
echo 'Deploy path: $DEPLOY_PATH'
cd $DEPLOY_PATH
git checkout main
git pull origin main
if [ ! -f .env.server ]; then
  echo 'Missing .env.server in deploy directory'
  echo 'Create it from .env.server.example on the server before rerunning the pipeline.'
  pwd
  ls -la
  exit 1
fi
echo 'Validating docker compose configuration'
docker compose --env-file .env.server -f compose.yaml -f compose.prod.yaml config
echo 'Starting backend containers'
docker compose --env-file .env.server -f compose.yaml -f compose.prod.yaml up -d --build --force-recreate
docker compose --env-file .env.server -f compose.yaml -f compose.prod.yaml ps
bash scripts/smoke_backend_runtime.sh
```

## Smoke tests apres deploy

Backend:

- `https://filfinance-api.sbcenter.fr/health` retourne 200.
- `wss://filfinance-api.sbcenter.fr/ws/news` accepte une session avec JWT Supabase valide.
- `/api/translations` repond avec JWT Supabase valide.
- `docker compose ps` montre `filfinance_service` healthy.
- `scripts/smoke_backend_runtime.sh` passe apres le deploy.

Smoke runtime serveur:

- le script verifie toujours `/health` depuis le conteneur;
- si `NEWS_TTS_ENABLED=false`, les checks Piper sont ignores;
- si `NEWS_TTS_ENABLED=true`, le deploy echoue si Python/Piper/helper/modele/voix ou synthese courte ne fonctionnent pas;
- si `NEWS_HLS_ENABLED=false`, les checks HLS sont ignores;
- si `NEWS_HLS_ENABLED=true`, le deploy echoue si TTS n'est pas actif, si `ffmpeg` manque, si le dossier HLS n'est pas writable ou si les playlists live/replay ne retournent pas `#EXTM3U`.

Frontend Cloudflare Pages:

- `https://fil-finance.pages.dev` charge l'app tant que le domaine custom n'est pas actif.
- Les variables Pages pointent vers `wss://filfinance-api.sbcenter.fr/ws/news`.
- L'ecran auth Supabase charge sans erreur de configuration.

Script commun:

```bash
bash scripts/smoke_deploy.sh
```

Script runtime serveur, a lancer depuis `/home/traefik/filfinance-docker`:

```bash
bash scripts/smoke_backend_runtime.sh
```

Avec un token Supabase:

```bash
FILFINANCE_AUTH_TOKEN="<supabase-access-token>" bash scripts/smoke_deploy.sh
```

## Rollback premier deploy

Rollback simple:

```bash
cd /home/traefik/filfinance-docker
git log --oneline -5
git checkout <commit-stable>
docker compose --env-file .env.server -f compose.yaml -f compose.prod.yaml up -d --build --force-recreate
```

Revenir ensuite a `main` ou pousser un revert propre:

```bash
git checkout main
git pull origin main
```

## Points de vigilance

- Ne jamais committer `.env.server`.
- `DEPLOY_PATH` doit etre configure avec `/home/traefik/filfinance-docker`.
- `.env.server.example` est a la racine du repo et commence par un point: utiliser `ls -la` pour le voir sur le serveur.
- Si le job GitLab s'arrete juste apres `git pull origin main`, verifier d'abord que `/home/traefik/filfinance-docker/.env.server` existe: la commande CI bloque volontairement avant Docker si ce fichier manque et liste le dossier distant.
- Ne pas ajouter `traefik-auth` sur l'API Fil Finance.
- Ne pas mettre les variables `EXPO_PUBLIC_*` dans `.env.server`; elles appartiennent a Cloudflare Pages.
- Si `NEWS_AUTH_ENABLED=true`, Supabase doit etre configure avant le lancement.
- Si FinancialJuice refuse de demarrer faute d'identifiants, corriger `.env.server` avant de relancer Compose.
- Si `NEWS_TTS_ENABLED=true` ou `NEWS_HLS_ENABLED=true`, le job GitLab refuse le deploy tant que le runtime Piper/HLS n'est pas operationnel.

## Correction `dubious ownership`

Si Git retourne:

```text
fatal: detected dubious ownership in repository at '/home/traefik/filfinance-docker'
```

Cela signifie que l'utilisateur courant, par exemple `gitlab-ci`, execute Git dans un dossier possede par un autre utilisateur.

Correction recommandee si `gitlab-ci` est l'utilisateur de deploiement:

```bash
sudo chown -R gitlab-ci:gitlab-ci /home/traefik/filfinance-docker
```

Puis reprendre:

```bash
cd /home/traefik/filfinance-docker
git remote add origin git@gitlab.com:simonbdbc/filfinance.git
git fetch origin main
git checkout -B main origin/main
```

Alternative si le dossier doit rester possede par `traefik` ou `root`:

```bash
git config --global --add safe.directory /home/traefik/filfinance-docker
```

## Suite

- Committer et pousser les changements EPIC-020.
- Configurer les variables GitLab CI/CD.
- Executer le bootstrap serveur.
- Declencher le premier deploy backend depuis `main`.
- Ouvrir un ticket dedie pour l'image Docker TTS/Piper si l'audio serveur doit etre active.
