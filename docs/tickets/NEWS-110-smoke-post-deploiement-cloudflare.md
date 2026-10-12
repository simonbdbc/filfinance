# NEWS-110 - Smoke post-deploiement Pages et backend

> **Archive historique — ne pas utiliser comme gate de release.** Ce smoke
> frontend unique/Supabase est supersede par la suite Better Auth a trois roles
> et les deux frontends. Utiliser [le runbook GitLab actuel](../GITLAB-RUNBOOK.md).

## Statut

Livre cote repo le 2 mai 2026.

Le script est disponible mais n'est pas encore bloquant en CI, car Cloudflare Pages reste deployee par Git integration hors GitLab.

Note `EPIC-028` du 21 mai 2026: le script a ete bascule vers les domaines
production finaux par defaut. L'historique ci-dessous reste utile pour le palier
provisoire; la suite production est portee par [NEWS-178](./NEWS-178-smokes-rollback-production-finale.md).

Smoke public execute avec succes le 2 mai 2026 contre:

- `https://fil-finance.pages.dev`;
- `https://filfinance-api.sbcenter.fr/health`.

Les checks authentifies restent optionnels et demandent `FILFINANCE_AUTH_TOKEN`.

## Objectif

Verifier rapidement que le deploiement public Fil Finance repond apres un push sur `main`:

- frontend Cloudflare Pages;
- fallback SPA;
- backend Traefik;
- endpoints authentifies critiques si un token Supabase est fourni.

## Scope livre

- script racine `scripts/smoke_deploy.sh`;
- URLs configurables par variables d'environnement;
- verification du HTML Pages et des assets Expo;
- verification d'une route profonde pour le fallback SPA;
- verification `GET /health` sur l'API;
- verification optionnelle de `GET /api/me`;
- verification optionnelle de `GET /api/translations`.

## Hors scope

- WebSocket automatise depuis CI;
- smoke complet par role `listener/moderator/admin`;
- blocage automatique du deploiement Cloudflare Pages;
- tests navigateur visuels.

## Utilisation

Smoke public minimal:

```bash
bash scripts/smoke_deploy.sh
```

Smoke avec endpoints authentifies:

```bash
FILFINANCE_AUTH_TOKEN="<supabase-access-token>" bash scripts/smoke_deploy.sh
```

URLs custom si besoin:

```bash
FILFINANCE_FRONTEND_URL="https://fil-finance.pages.dev" \
FILFINANCE_API_URL="https://filfinance-api.sbcenter.fr" \
bash scripts/smoke_deploy.sh
```

## Variables

- `FILFINANCE_SMOKE_SCOPE`: `full` ou `frontend`, defaut `full`.
- `FILFINANCE_FRONTEND_URL`: URL Pages, defaut production `https://filfinance.fr`.
- `FILFINANCE_API_URL`: URL API, defaut production `https://api.filfinance.fr`.
- `FILFINANCE_EDGE_URL`: URL Worker edge, defaut production `https://live.filfinance.fr`.
- `FILFINANCE_CHECK_EDGE`: `1` pour verifier `/health` sur le Worker edge, defaut `0`.
- `FILFINANCE_AUTH_TOKEN`: access token Supabase optionnel pour tester les endpoints proteges.

## Criteres d'acceptation

- `https://fil-finance.pages.dev` retourne un HTML contenant les assets Expo tant que le domaine custom n'est pas actif.
- `https://fil-finance.pages.dev/profile` retourne aussi l'app via le fallback SPA tant que le domaine custom n'est pas actif.
- `https://filfinance-api.sbcenter.fr/health` repond avec succes.
- Avec `FILFINANCE_AUTH_TOKEN`, `/api/me` repond avec succes.
- Avec `FILFINANCE_AUTH_TOKEN`, `/api/translations` repond avec succes.

## Suite

Quand le deploiement Cloudflare Pages passera sous controle GitLab direct, ce script pourra devenir un job `deploy:smoke` bloquant apres publication.
