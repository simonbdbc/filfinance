# EPIC-028 - Migration production `main` vers `filfinance.fr`

> **Archive historique.** Les chemins `filfinance-client`, Supabase et
> `wrangler@latest` ci-dessous ne decrivent plus la release active. Utiliser
> [le runbook GitLab actuel](../GITLAB-RUNBOOK.md) et
> [le runbook Better Auth](../AUTH-RUNBOOK.md).

## Objectif

Migrer l'environnement de production Fil Finance depuis le domaine Cloudflare Pages
provisoire vers la cible publique finale:

- branche de production: `main`;
- zone DNS: `filfinance.fr` achete chez OVH mais gere par Cloudflare;
- frontend production: `https://filfinance.fr`;
- API production: `https://api.filfinance.fr`;
- live edge production: `https://live.filfinance.fr`;
- media HLS production: `https://media.filfinance.fr`;
- developpement courant: local uniquement, via `.env.local`, `compose.local.yaml` et le lancement Expo/Rust local.

Cet EPIC doit rendre `main` explicitement deployable en production sans utiliser
`https://fil-finance.pages.dev` comme environnement de production nominal.

## Valeur livree

- La production publique a un domaine final lisible et stable.
- Le pipeline `main` devient le chemin de deploiement production complet.
- Les smokes de production ciblent `filfinance.fr` et `api.filfinance.fr`.
- Le developpement local reste possible sans environnement public `dev`.
- La future creation de `fil-finance.pages.dev` comme environnement `dev` est isolee dans `EPIC-029`.

## Statut

Cadrage ajoute le 21 mai 2026.

Demarrage repo le 21 mai 2026:

- `NEWS-176`, `NEWS-177` et `NEWS-178` sont crees;
- `.gitlab-ci.yml` ajoute `edge:check`, `deploy:main:pages`, `deploy:main:edge` et `smoke:main`;
- `deploy:home-srv` est renomme `deploy:main:backend` sans changement fonctionnel serveur;
- les deploys Pages/Edge et le smoke final sont proteges par `FILFINANCE_PROD_DEPLOY_ENABLED=true`;
- `scripts/smoke_deploy.sh` cible les domaines production par defaut et accepte un scope `frontend`.

Priorite: avant ouverture publique durable de `filfinance.fr`.

Dependances:

- `EPIC-020` reste le palier historique GitLab, Pages provisoire et backend serveur.
- `EPIC-026` porte le durcissement securite/charge production.
- `EPIC-027` valide le cache HLS Worker.

## Decisions

- `main` est la branche unique de production.
- `filfinance.fr` remplace `fil-finance.pages.dev` comme frontend production.
- `fil-finance.pages.dev` n'est plus un fallback produit de production apres migration; il sera repris par `EPIC-029` pour la branche `dev`.
- Le developpement avant `EPIC-029` reste local: pas de backend dev public, pas de Pages dev public.
- GitLab reste le pipeline maitre: validations, deploiement frontend, backend, edge, puis smokes.
- Pour permettre plus tard `fil-finance.pages.dev` sur `dev`, la production Pages doit etre isolee dans un projet Pages dedie, par exemple `filfinance-prod`, avec custom domain `filfinance.fr`.
- Supabase reste partage entre local/prod pour ce palier; la separation de projet Supabase n'est pas incluse.

## Scope

Dans cet EPIC:

- ajouter `filfinance.fr` dans les domaines/zones geres par Cloudflare et basculer les nameservers OVH vers Cloudflare;
- s'assurer que les records HTTP/HTTPS de production sont proxifies par Cloudflare;
- creer ou configurer le projet Cloudflare Pages production `filfinance-prod`;
- attacher `filfinance.fr` au projet Pages production;
- migrer les variables Pages production vers les domaines finaux;
- brancher le deploy frontend production depuis GitLab sur `main`;
- maintenir le deploy backend serveur production depuis `main`;
- deployer le Worker edge production depuis `main`;
- mettre a jour `scripts/smoke_deploy.sh` et les runbooks pour les URLs finales;
- documenter rollback frontend, backend et edge.

Hors scope:

- creation de l'environnement dev public sur `fil-finance.pages.dev`;
- creation d'un backend dev public;
- separation Supabase dev/prod;
- migration de SQLite pipeline vers Supabase;
- changement fonctionnel produit hors besoins de configuration production.

## Architecture cible production

```text
main
  -> GitLab CI validate
  -> deploy frontend Pages prod
       project: filfinance-prod
       custom domain: filfinance.fr
  -> deploy backend serveur
       host: serveur personnel Traefik
       domain: api.filfinance.fr
  -> deploy Worker edge
       domain: live.filfinance.fr
  -> serve HLS media via Bunny
       domain: media.filfinance.fr
  -> smoke production complet
```

Variables publiques Pages production:

```text
NODE_VERSION=24.13.1
EXPO_PUBLIC_FILFINANCE_WS_URL=wss://api.filfinance.fr/ws/news
EXPO_PUBLIC_FILFINANCE_EDGE_WS_URL=wss://live.filfinance.fr/ws/live
EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL=https://media.filfinance.fr
EXPO_PUBLIC_SUPABASE_URL=https://project-ref.supabase.co
EXPO_PUBLIC_SUPABASE_PUBLISHABLE_KEY=sb_publishable_xxx
EXPO_PUBLIC_SUPABASE_ANON_KEY=eyJhbGciOi-placeholder
```

Depuis `EPIC-034`, `live.filfinance.fr` ne doit plus etre utilise comme base HLS. La
cible media production conforme est Bunny via `https://media.filfinance.fr`.

Variables serveur production:

- conserver `.env.server` hors Git sur le serveur;
- verifier `NEWS_SERVER_ENV=production`;
- verifier `NEWS_AUTH_ENABLED=true`;
- verifier `NEWS_EDGE_PUBLISH_URL=https://live.filfinance.fr/publish/live`;
- verifier les secrets Edge, Supabase, FinancialJuice et Telegram.

Pre-requis DNS/Cloudflare:

- `filfinance.fr` est ajoute comme zone Cloudflare;
- les nameservers OVH pointent vers les nameservers assignes par Cloudflare;
- les records HTTP/HTTPS de production sont proxifies;
- les records non HTTP, par exemple MX/TXT, restent en DNS-only si presents.

## Pipeline CI/CD cible

Jobs attendus sur `main`:

- `docs:check`;
- `rust:fmt`;
- `rust:test`;
- `client:typecheck`;
- `client:web-build`;
- `edge:check`;
- `deploy:main:pages`;
- `deploy:main:backend`;
- `deploy:main:edge`;
- `smoke:main`.

Regles:

- aucun deploy production hors branche `main`;
- les deploys production attendent les validations;
- `smoke:main` attend les deploys Pages, backend et edge;
- les variables Cloudflare production sont protegees dans GitLab;
- les variables publiques Expo ne sont jamais placees dans `.env.server`.

Variables GitLab a prevoir:

```text
CLOUDFLARE_ACCOUNT_ID
CLOUDFLARE_API_TOKEN_PAGES_PROD
CLOUDFLARE_API_TOKEN_WORKERS_PROD
PAGES_PROJECT_PROD=filfinance-prod
SSH_PRIVATE_KEY
USER
HOST
PORT
DEPLOY_PATH
```

## Tickets de dev

### [`NEWS-176` Configuration production `filfinance.fr`](../tickets/NEWS-176-configuration-production-filfinance-fr.md)

Objectif: configurer le domaine frontend final et les variables publiques de production.

Criteres d'acceptation:

- `filfinance.fr` sert le build web issu de `main`;
- la zone Cloudflare `filfinance.fr` est active apres bascule OVH;
- les records web de production sont proxifies;
- le certificat TLS Cloudflare est actif;
- les variables Pages pointent vers `api.filfinance.fr` et `live.filfinance.fr`;
- la variable HLS Pages pointe vers `media.filfinance.fr`;
- Supabase Auth accepte `https://filfinance.fr`.

### [`NEWS-177` Pipeline GitLab production `main`](../tickets/NEWS-177-pipeline-gitlab-production-main.md)

Objectif: remplacer le palier provisoire par un chemin production explicite.

Criteres d'acceptation:

- `main` declenche les validations puis les jobs `deploy:main:*`;
- le frontend Pages prod est deploye depuis l'artefact `filfinance-client/dist`;
- le backend serveur reste deploye depuis `main`;
- le Worker edge prod est deploye depuis `main`;
- les tokens Cloudflare sont masques/proteges.

### [`NEWS-178` Smokes et rollback production finale](../tickets/NEWS-178-smokes-rollback-production-finale.md)

Objectif: rendre le go/no-go production reproductible.

Criteres d'acceptation:

- `scripts/smoke_deploy.sh` cible `filfinance.fr` et `api.filfinance.fr` par defaut;
- le smoke public verifie frontend, route profonde, API health et live edge si applicable;
- le smoke authentifie verifie `/api/me` et l'historique;
- les rollbacks Pages, backend et edge sont documentes.

## Definition of Done

- `main` deploie la production complete.
- `https://filfinance.fr` est le frontend public nominal.
- `https://api.filfinance.fr/health` repond.
- `https://live.filfinance.fr` sert le Worker edge attendu.
- `https://media.filfinance.fr/audio/...` sert le HLS via Bunny avec ticket valide.
- `https://fil-finance.pages.dev` n'est plus documente comme production ou fallback produit.
- Le developpement local reste documente et fonctionnel.
- Le rollback production est teste ou executable pas a pas.

## References

- [EPIC-020 - GitLab, Cloudflare Pages et serveur personnel](./EPIC-020-gitlab-pipeline-cloudflare.md)
- [EPIC-026 - Securite et charge production](./EPIC-026-securite-et-charge-production.md)
- [PRODUCTION-FINAL-RUNBOOK](../PRODUCTION-FINAL-RUNBOOK.md)
- [GITLAB-RUNBOOK](../GITLAB-RUNBOOK.md)
