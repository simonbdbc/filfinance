# NEWS-109 - Secrets, environnements et rollback

## Statut

Livre cote repo le 2 mai 2026.

> **Archive historique.** Ce ticket decrit l'ancienne architecture Supabase,
> `filfinance-client` et le projet Pages unique. Pour l'exploitation actuelle,
> utiliser [le runbook GitLab](../GITLAB-RUNBOOK.md),
> [le runbook Better Auth](../AUTH-RUNBOOK.md) et
> [les fichiers d'environnement](../ENVIRONMENT-FILES.md).

## Objectif

Formaliser l'exploitation des secrets, des environnements et du rollback pour le pipeline Fil Finance:

- frontend Cloudflare Pages;
- backend serveur personnel Traefik;
- Supabase Auth;
- GitLab CI/CD;
- fichiers `.env*` locaux et serveur.

## Scope livre

- separation des variables publiques frontend et secrets serveur;
- liste des variables GitLab necessaires au deploiement SSH;
- rappel des variables Cloudflare Pages preview/production;
- procedure de rollback frontend Pages;
- procedure de rollback backend serveur;
- checklist incident minimaliste.

## Hors scope

- creation effective des secrets dans les dashboards;
- rotation automatisee des secrets;
- gestion d'environnements staging complets;
- Cloudflare Workers/Durable Objects.

## Sources de configuration

### Cloudflare Pages

Variables publiques, visibles dans le bundle Expo Web:

```text
NODE_VERSION=24.13.1
EXPO_PUBLIC_FILFINANCE_WS_URL=wss://api.filfinance.fr/ws/news
EXPO_PUBLIC_FILFINANCE_EDGE_WS_URL=wss://live.filfinance.fr/ws/live
EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL=https://media.filfinance.fr
EXPO_PUBLIC_SUPABASE_URL=https://project-ref.supabase.co
EXPO_PUBLIC_SUPABASE_PUBLISHABLE_KEY=sb_publishable_xxx
EXPO_PUBLIC_SUPABASE_ANON_KEY=eyJhbGciOi-placeholder
```

Depuis `EPIC-034`, `live.filfinance.fr` ne sert plus le data plane audio HLS. La base HLS
publique production est Bunny `https://media.filfinance.fr`.

Regles:

- depuis `EPIC-028`, configurer ces cles dans GitLab pour le build production `main`;
- garder Cloudflare Pages dashboard sans secret serveur;
- utiliser des valeurs preview/dev differentes seulement quand `EPIC-029` ouvrira l'environnement `dev`;
- ne jamais mettre de secret serveur dans une variable `EXPO_PUBLIC_*`;
- laisser le deploy command Cloudflare Pages vide si un projet Git integration historique existe encore.

### GitLab CI/CD

Variables serveur requises:

```text
SSH_PRIVATE_KEY
USER
HOST
PORT
DEPLOY_PATH
```

Valeur cible de `DEPLOY_PATH`:

```text
/home/traefik/filfinance-docker
```

Regles:

- `SSH_PRIVATE_KEY` contient la cle privee de deploiement encodee en base64;
- la cle publique correspondante est autorisee sur le serveur pour l'utilisateur de deploiement;
- les variables GitLab ne doivent pas contenir les variables publiques Expo, sauf si un futur job GitLab publie directement Pages.

### Serveur personnel

Fichier runtime attendu:

```text
/home/traefik/filfinance-docker/.env.server
```

Regles:

- creer ce fichier depuis `.env.server.example`;
- ne jamais le committer;
- le proteger avec `chmod 600`;
- y stocker uniquement la config backend serveur.

### Developpement local

Fichiers locaux:

```text
filfinance-service/.env
filfinance-client/.env
```

Regles:

- `filfinance-service/.env` sert uniquement au backend local;
- `filfinance-client/.env` sert uniquement au client Expo local;
- `.env.server` remplace `filfinance-service/.env` sur le serveur Docker.

## Rollback frontend Cloudflare Pages

Procedure simple:

1. ouvrir le projet Cloudflare Pages `fil-finance`;
2. aller dans `Deployments`;
3. identifier le dernier deploiement stable;
4. lancer `Rollback` ou `Redeploy` sur ce deploiement stable;
5. executer le smoke public:

```bash
bash scripts/smoke_deploy.sh
```

Point operationnel: `Retry` relance le meme commit. Pour valider un correctif, pousser un nouveau commit sur `main`.

## Rollback backend serveur

Procedure serveur:

```bash
cd /home/traefik/filfinance-docker
git log --oneline -10
git checkout <commit-stable>
docker compose --env-file .env.server -f compose.yaml -f compose.prod.yaml up -d --build --force-recreate
docker compose --env-file .env.server -f compose.yaml -f compose.prod.yaml ps
```

Validation:

```bash
curl -fsS https://api.filfinance.fr/health
FILFINANCE_CHECK_EDGE=1 bash scripts/smoke_deploy.sh full
```

Retour au flux normal:

```bash
git checkout main
git pull origin main
```

Si le rollback reste en place plus longtemps qu'un diagnostic court, pousser ensuite un revert ou un correctif sur `main` pour realigner GitLab et le serveur.

## Checklist incident

Pour tout incident deploy non trivial, noter:

- date et heure;
- commit concerne;
- symptome utilisateur;
- surface touchee: Pages, API, WebSocket, auth, SQLite, Supabase;
- action immediate: rollback, revert ou correctif;
- cause racine probable;
- action preventive.

## Criteres d'acceptation

- les secrets serveur restent hors Git;
- les variables publiques frontend sont distinguees des secrets serveur;
- le rollback Pages est faisable sans commit correctif;
- le rollback backend est faisable depuis le serveur;
- le smoke `scripts/smoke_deploy.sh` est la validation commune apres rollback.
