# NEWS-186 - Pipeline production main n8n résumé

## Objectif

Intégrer n8n au pipeline de déploiement production `main` sans le rendre
obligatoire.

## Scope

- `docs:check` valide docs, compose n8n, workflow et sample de résumé.
- `deploy:main:backend` inclut `compose.n8n.yaml` seulement si
  `FILFINANCE_N8N_PROD_ENABLED=true`.
- `scripts/smoke_backend_runtime.sh` teste n8n seulement quand le flag est actif.
- Le live Piper reste le défaut de production.

## Critères d'acceptation

- `FILFINANCE_N8N_PROD_ENABLED=false` déploie `compose.yaml + compose.prod.yaml`.
- `FILFINANCE_N8N_PROD_ENABLED=true` déploie aussi `compose.n8n.yaml`.
- Le smoke vérifie `n8n`, `n8n-postgres`, `OPENROUTER_MODEL` et le callback
  daily summary.
- Les commandes compose restent idempotentes avec `up -d --build --force-recreate`.
