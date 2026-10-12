# Pilote Bunny DEV

Ce runbook est le contrat opératoire courant pour `dev`. Il ne modifie ni DNS, ni Pages, ni
Worker de production Cloudflare. La sous-zone déléguée est `dev.filfinance.fr`.

## Topologie

| Surface | Hostname | Bunny |
| --- | --- | --- |
| Landing | `dev.filfinance.fr` | Storage Zone `BUNNY_STORAGE_ZONE_DEV_LANDING` + Pull Zone |
| PWA | `app.dev.filfinance.fr` | Storage Zone `BUNNY_STORAGE_ZONE_DEV_PWA` + Pull Zone, fallback `/index.html` avec HTTP 200 |
| API / Better Auth | `api.dev.filfinance.fr` | Pull Zone vers `https://origin.dev.filfinance.fr`, cache désactivé |
| HLS | `media.dev.filfinance.fr` | Pull Zone vers `https://origin.dev.filfinance.fr`, cache selon headers Rust, query string dans la clé, CORS HLS, POST refusé |
| Live | `live.dev.filfinance.fr` | Pull Zone vers `https://origin.dev.filfinance.fr`, WebSockets activés, cache désactivé |
| Origin privé | `origin.dev.filfinance.fr` | Traefik, certificat dédié, filtré aux plages Bunny |

Les cinq premiers hôtes ont un certificat Bunny. `origin.dev.filfinance.fr` a un certificat
Traefik DNS-01 Bunny et n'est jamais une URL client. `FILFINANCE_API_HOST=api.dev.filfinance.fr` reste
l'hôte public canonique de Better Auth; `FILFINANCE_ORIGIN_HOST=origin.dev.filfinance.fr` ajoute
seulement la route d'origine Traefik.

## Provisionnement hors dépôt

1. Exporter les enregistrements DEV existants et les règles pare-feu avant toute modification.
   Précharger les deux Storage Zones, déployer le backend avec
   `FILFINANCE_DEV_BOOTSTRAP_RELEASE=true`, puis déléguer les NS Bunny depuis la zone parent
   Cloudflare. Le bootstrap ignore uniquement les sondes publiques tant que les nouveaux hosts
   ne résolvent pas; les contrôles locaux Docker et Better Auth restent obligatoires.
2. Créer les deux Storage Zones et les cinq Pull Zones ci-dessus. Activer le fallback SPA PWA,
   les certificats, Bunny Shield Basic sur API/media/live et un budget d'alerte de 5 USD/mois.
3. Ajouter les deux règles Shield: limitation des routes Better Auth et limitation du handshake
   `/ws/live`. Vérifier explicitement que API conserve les méthodes applicatives, HLS bloque POST
   et que `live` accepte WebSocket.
4. Configurer Traefik avec le resolver DNS-01 `bunny_dns` et une clé Bunny DNS distincte, montée
   en fichier secret. Restreindre ensuite le pare-feu de l'origin aux plages Bunny publiées sur
   TCP 80/443, conserver un accès opérateur séparé et vérifier le certificat de l'origin après
   la délégation DNS.

Ces opérations exigent un compte Bunny et l'accès DNS/pare-feu; aucun identifiant n'est stocké
dans ce dépôt.

## Profil serveur DEV

Le fichier `.env.server.dev` déployé sur l'origin doit contenir au minimum :

```dotenv
FILFINANCE_API_HOST=api.dev.filfinance.fr
FILFINANCE_ORIGIN_HOST=origin.dev.filfinance.fr
FILFINANCE_TRAEFIK_CERT_RESOLVER=bunny_dns
BETTER_AUTH_URL=https://api.dev.filfinance.fr
BETTER_AUTH_JWT_AUDIENCE=https://api.dev.filfinance.fr
BETTER_AUTH_TRUSTED_ORIGINS=filfinance://,https://app.dev.filfinance.fr
APP_WEB_URL=https://app.dev.filfinance.fr
NEWS_AUTH_ISSUER=https://api.dev.filfinance.fr
NEWS_AUTH_AUDIENCE=https://api.dev.filfinance.fr
NEWS_EDGE_PUBLISH_ENABLED=false
NEWS_EDGE_TICKET_ENABLED=true
NEWS_EDGE_TICKET_SECRET=<secret-protégé-au-moins-32-caractères>
NEWS_LIVE_FANOUT_ENABLED=true
NEWS_LIVE_REDIS_URL=redis://filfinance-redis:6379/0
```

Redis est privé dans Docker, avec AOF, healthcheck et volume dédié. Le backend vérifie le ticket
HMAC court de `/api/edge/live-ticket` sur `GET /ws/live?ticket=…`, conserve le snapshot et les
historiques bornés dans Redis, puis relaie les messages Pub/Sub entre replicas. Une panne Redis
est journalisée; elle ne bloque pas l'ingestion ni la diffusion locale.

## CI et secrets GitLab

Le job `release:dev-lock` appelle `scripts/check_bunny_dev_release_preflight.sh` avant tout
déploiement. Configurer ces variables GitLab **protégées et masquées** :

- `BUNNY_API_KEY_DEV`, `BUNNY_STORAGE_PASSWORD_DEV_LANDING`,
  `BUNNY_STORAGE_PASSWORD_DEV_PWA`.

Configurer ces identifiants **protégés, non masqués** :

- `BUNNY_DNS_ZONE_ID_DEV=857837`;
- `BUNNY_STORAGE_ZONE_DEV_LANDING=1802401`, `BUNNY_PULL_ZONE_ID_DEV_LANDING=6451217`;
- `BUNNY_STORAGE_ZONE_DEV_PWA=1802402`, `BUNNY_PULL_ZONE_ID_DEV_PWA=6451219`;
- `BUNNY_PULL_ZONE_ID_DEV_API=6451222`, `BUNNY_PULL_ZONE_ID_DEV_MEDIA=6033259`,
  `BUNNY_PULL_ZONE_ID_DEV_LIVE=6451226`;
- `FILFINANCE_DEV_BUNNY_DEPLOY_ENABLED=true` après revue du provisioning.

`FILFINANCE_DEV_EXECUTION_ENABLED=true` et `FILFINANCE_BETTER_AUTH_DEPLOY_ENABLED=true` sont
aussi requis sur la branche `dev` protégée. La clé DNS-01 Traefik est différente de
`BUNNY_API_KEY_DEV` et reste uniquement sur le serveur.

`scripts/deploy_bunny_storage.mjs` n'écrit aucun secret et ne les inclut pas aux artefacts.

## Validation et rollback

`smoke:dev` vérifie landing, PWA et route profonde, Better Auth/API, HLS avec ticket, puis
WebSocket `live` avec réception de `hello`. Faire les paliers HLS 50, 100, 200, 500 puis 1 000
clients seulement si 500 est propre, avec `FILFINANCE_LOAD_TEST_SKIP_WS=true`; exécuter séparément
le fan-out WebSocket sur `live.dev.filfinance.fr` avec deux replicas et au plus 500 sockets.

La bascule est acceptée après résolution Bunny, certificats valides, trafic DEV absent de
Cloudflare et succès des smokes/charges. Le rollback restaure la délégation DNS et la précédente
configuration DEV; il ne réactive pas le proxy HLS Worker.
