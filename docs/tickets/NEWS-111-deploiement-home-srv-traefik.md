# NEWS-111 - Deploiement serveur perso Traefik

> **Archive historique — ne pas executer.** Les domaines, chemins, services et
> variables de ce premier deploiement sont supersedes. Utiliser
> [le runbook GitLab actuel](../GITLAB-RUNBOOK.md) et
> [les fichiers d'environnement actuels](../ENVIRONMENT-FILES.md).

## Objectif

Integrer le backend Fil Finance au serveur personnel existant `home-srv`, avec le meme modele de deploiement GitLab CI/CD que les stacks `traefik-docker`, `media-docker` et `nextcloud-docker`.

Le frontend reste heberge sur Cloudflare Pages.

## Statut

Prepare cote repo Fil Finance le 2 mai 2026.

Route Traefik API appliquee dans `home-srv/traefik-docker` via `NEWS-112`.

## Audit `home-srv`

Le dossier `/Users/simon/Projets/PlateformeDev/home-srv` contient plusieurs repos Git separes:

- `traefik-docker`
- `media-docker`
- `nextcloud-docker`

Le dossier parent n'est pas un monorepo Git.

### Traefik

Traefik tourne dans `traefik-docker`:

- image `traefik:v2.10.5`
- provider Docker actif avec `exposedByDefault=false`
- provider file actif sur `/etc/traefik/conf`
- reseau Docker externe commun: `traefik_network`
- entrypoints:
  - `web` sur `80`, redirige vers HTTPS
  - `websecure` sur `443`
  - `postgres` sur `15432`
- certificats Let's Encrypt via resolver `lets_encrypt`

Les routes publiques sont principalement gerees dans:

```text
traefik-docker/volumes/config/conf/*.yaml
```

Les services Docker exposent seulement leur port interne via labels, puis les fichiers dynamiques Traefik referencent les services en `@docker`.

### CI/CD existant

Chaque repo serveur possede une `.gitlab-ci.yml`.

Le pattern de deploiement est:

1. GitLab prepare une cle SSH depuis `SSH_PRIVATE_KEY`;
2. GitLab se connecte au serveur via `$USER@$HOST -p$PORT`;
3. GitLab va dans `$DEPLOY_PATH`;
4. GitLab execute:

```bash
git checkout main
git pull origin main
docker compose up -d
```

Fil Finance reprend ce pattern, avec une commande adaptee:

```bash
docker compose --env-file .env.server -f compose.yaml -f compose.prod.yaml up -d --build --force-recreate
```

## Scope livre cote Fil Finance

- `compose.yaml` + `compose.prod.yaml` a la racine;
- image Docker service Rust reutilisant `filfinance-service/Dockerfile`;
- `curl` ajoute dans l'image service pour le healthcheck Docker;
- `.env.server.example` pour les variables serveur;
- job GitLab `deploy:home-srv`;
- `filfinance-client/public/_redirects` pour le fallback SPA Cloudflare/Pages-compatible;
- exclusion `volumes/` dans `.gitignore`.

## Architecture de deploiement proposee

```text
GitLab filfinance/main
  -> pipeline validate
  -> Cloudflare Pages Git integration build/deploy filfinance-client
  -> deploy:home-srv SSH
  -> serveur perso
  -> /chemin/de/deploiement/filfinance
  -> docker compose --env-file .env.server -f compose.yaml -f compose.prod.yaml up -d --build --force-recreate
  -> filfinance_service sur traefik_network
  -> Traefik route HTTPS API/WebSocket
```

Services Docker:

- `filfinance_service`: sert l'API Rust, les WebSockets et l'historique.

Volumes persistants:

- `./volumes/service/data:/app/data`

Ce volume contient SQLite `news.db` et doit rester hors Git.

## Domaines proposes

Proposition:

- app Web Cloudflare Pages provisoire: `https://fil-finance.pages.dev`
- backend API/WebSocket Traefik: `filfinance-api.sbcenter.fr`

Le client Cloudflare Pages doit donc etre build avec:

```env
EXPO_PUBLIC_FILFINANCE_WS_URL=wss://filfinance-api.sbcenter.fr/ws/news
EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL=
EXPO_PUBLIC_SUPABASE_URL=https://project-ref.supabase.co
EXPO_PUBLIC_SUPABASE_PUBLISHABLE_KEY=sb_publishable_xxx
```

Note `EPIC-034`: ne pas configurer le Worker Cloudflare comme base HLS. La valeur vide
laisse le client utiliser l'origin backend.

Le client derive ensuite automatiquement:

- `wss://filfinance-api.sbcenter.fr/ws/audio`
- `wss://filfinance-api.sbcenter.fr/ws/test-runs`
- `https://filfinance-api.sbcenter.fr/api/...`

## Configuration Traefik a ajouter

Fichier propose dans `home-srv/traefik-docker`:

```text
volumes/config/conf/filfinance.yaml
```

Contenu propose pour l'API uniquement:

```yaml
http:
  routers:
    filfinance-api:
      rule: "Host(`filfinance-api.sbcenter.fr`)"
      entryPoints:
        - websecure
      tls:
        certResolver: lets_encrypt
        domains:
          - main: "filfinance-api.sbcenter.fr"
      middlewares:
        - default-headers
      service: filfinance-service@docker
```

Point important: ne pas ajouter `traefik-auth` sur `filfinance-api`, sinon le client Web ne pourra pas ouvrir les WebSockets ni appeler les endpoints HTTP. L'autorisation applicative doit rester portee par Supabase JWT.

## Variables GitLab requises

Variables deja utilisees par les autres repos serveur:

- `SSH_PRIVATE_KEY`: cle privee encodee en base64;
- `USER`: utilisateur SSH serveur;
- `HOST`: host ou IP du serveur;
- `PORT`: port SSH;
- `DEPLOY_PATH`: dossier du repo `filfinance` sur le serveur.

Exemple de `DEPLOY_PATH`:

```text
/home/traefik/filfinance-docker
```

## Variables serveur requises

Sur le serveur, creer:

```text
.env.server
```

a partir de:

```text
.env.server.example
```

Secrets a renseigner au minimum:

- `NEWS_SUPABASE_PROJECT_URL`
- `NEWS_FINANCIAL_JUICE_EMAIL`
- `NEWS_FINANCIAL_JUICE_PASSWORD`

Les variables `EXPO_PUBLIC_*` doivent etre configurees dans Cloudflare Pages, pas dans `.env.server`.

## Limites du premier deploiement serveur

Le premier deploiement est volontairement pragmatique:

- traduction en `heuristic_fr`;
- TTS/Piper desactive;
- source macro externe retiree du scope;
- SQLite local persiste dans `./volumes/service/data`.

Raison: l'image Docker actuelle ne contient pas encore le runtime Python, les modeles `OPUS-MT` ni les voix Piper. Cela sera traite dans un ticket dedie pour eviter de bloquer le deploiement backend initial.
Mise a jour du 3 mai 2026: le runtime Python/Piper est maintenant package par l'image Docker via `NEWS-125`; les assets voix Piper restent dans le volume serveur `/app/data/models/piper` et doivent etre bootstrappes avant d'activer `NEWS_TTS_ENABLED=true`.

## Validations attendues

Avant push:

```bash
FILFINANCE_ENV_FILE=.env.server.example docker compose --env-file .env.server.example -f compose.yaml -f compose.prod.yaml config
docker compose --env-file .env.server -f compose.yaml -f compose.prod.yaml build
```

Apres deploiement:

```bash
docker compose -f compose.yaml -f compose.prod.yaml ps
curl -fsS https://filfinance-api.sbcenter.fr/health
```

Smoke Web:

- `https://fil-finance.pages.dev` charge l'app depuis Cloudflare Pages;
- l'ecran auth Supabase apparait;
- le client tente `wss://filfinance-api.sbcenter.fr/ws/news`;
- `/api/translations` repond avec JWT valide.

## Risques et points de vigilance

- Les repos `media-docker` et `nextcloud-docker` avaient deja des modifications locales non liees a Fil Finance; ne pas les nettoyer depuis ce ticket.
- Le repo `traefik-docker` etait propre lors de l'audit.
- Le domaine API doit pointer vers le serveur avant validation Let's Encrypt.
- Le domaine frontend doit rester configure comme custom domain Cloudflare Pages.
- Si `NEWS_AUTH_ENABLED=true`, Supabase doit etre configure avant lancement, sinon le service refusera de demarrer.

## Suite

Tickets a prevoir:

- [`NEWS-112`](./NEWS-112-route-traefik-api-filfinance.md): appliquer la route Traefik API `filfinance.yaml` dans `traefik-docker`;
- [`NEWS-113`](./NEWS-113-bootstrap-serveur-premier-deploy.md): bootstrap repo sur le serveur, `.env.server`, premier deploy GitLab;
- `NEWS-115`: image Docker avec runtime TTS/Piper et assets montes;
- `NEWS-116`: smoke tests serveur automatises post-deploiement.
