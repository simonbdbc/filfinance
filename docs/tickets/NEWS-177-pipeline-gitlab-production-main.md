# NEWS-177 - Pipeline GitLab production `main`

> **Archive historique.** Ce ticket conserve l'ancien pipeline frontend unique
> et Supabase. La source d'exploitation actuelle est
> [le runbook GitLab](../GITLAB-RUNBOOK.md).

## Objectif

Brancher le pipeline GitLab cible de `EPIC-028` pour que `main` devienne le
chemin de production complet:

- validation docs/Rust/client/edge;
- deploiement Pages production;
- deploiement backend serveur;
- deploiement Worker edge;
- smoke production.

## Statut

Demarre cote repo le 21 mai 2026.

Le deploiement Pages/Edge production est protege par le flag GitLab:

```text
FILFINANCE_PROD_DEPLOY_ENABLED=true
```

Sans ce flag, le pipeline conserve le comportement principal actuel: validation
et deploiement backend `main`.

## Jobs ajoutes ou renommes

Validation:

- `edge:check`: execute `npm ci` puis `npm run check` dans `filfinance-edge`.

Deploy:

- `deploy:main:pages`: upload Direct Upload de `filfinance-client/dist` vers Pages;
- `deploy:main:backend`: ancien `deploy:home-srv`, meme deploiement SSH serveur;
- `deploy:main:edge`: deploy Wrangler du Worker `filfinance-edge`.

Smoke:

- `smoke:main`: execute `scripts/smoke_deploy.sh full` sur les domaines finaux.

## Variables GitLab requises

Activation:

```text
FILFINANCE_PROD_DEPLOY_ENABLED=true
```

Cloudflare Pages:

```text
CLOUDFLARE_ACCOUNT_ID
CLOUDFLARE_API_TOKEN_PAGES_PROD
PAGES_PROJECT_PROD=filfinance-prod
```

Cloudflare Workers:

```text
CLOUDFLARE_API_TOKEN_WORKERS_PROD
```

Variables publiques Expo pour le build production:

```text
EXPO_PUBLIC_SUPABASE_URL
EXPO_PUBLIC_SUPABASE_PUBLISHABLE_KEY
EXPO_PUBLIC_FILFINANCE_WS_URL
EXPO_PUBLIC_FILFINANCE_EDGE_WS_URL
EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL
```

Les trois variables `EXPO_PUBLIC_FILFINANCE_*` ont des defaults production dans
le job `client:web-build` quand le flag `FILFINANCE_PROD_DEPLOY_ENABLED=true`
est actif sur `main`.

Serveur backend:

```text
SSH_PRIVATE_KEY
USER
HOST
PORT
DEPLOY_PATH
```

## Commandes CI principales

Pages Direct Upload:

```bash
npx --yes wrangler@latest pages deploy filfinance-client/dist \
  --project-name="${PAGES_PROJECT_PROD:-filfinance-prod}" \
  --branch=main
```

Worker:

```bash
cd filfinance-edge
npx --yes wrangler@latest deploy
```

Smoke:

```bash
FILFINANCE_CHECK_EDGE=1 bash scripts/smoke_deploy.sh full
```

## Criteres d'acceptation

- `main` lance `edge:check`.
- `deploy:main:backend` garde le deploiement serveur actuel.
- Quand `FILFINANCE_PROD_DEPLOY_ENABLED=true`, `main` lance aussi Pages, Edge et smoke final.
- Les secrets Cloudflare sont lus depuis les variables GitLab protegees.
- Le build production echoue si la configuration Supabase publique est absente.

## References

- [Cloudflare Pages Direct Upload](https://developers.cloudflare.com/pages/get-started/direct-upload/)
- [Cloudflare Pages Direct Upload avec CI](https://developers.cloudflare.com/pages/how-to/use-direct-upload-with-continuous-integration/)
- [Cloudflare Workers GitLab CI/CD](https://developers.cloudflare.com/workers/ci-cd/external-cicd/gitlab-cicd/)
