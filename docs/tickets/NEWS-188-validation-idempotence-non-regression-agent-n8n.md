# NEWS-188 - Validation idempotence et non-régression résumé IA

## Objectif

Vérifier que le résumé n8n/OpenRouter n'introduit pas de régression dans le live
Filfinance et que les opérations sont rejouables.

## Plan de test local

- `cargo fmt --check`
- `cargo test`
- `docker compose --env-file .env.local.example -f compose.yaml -f compose.local.yaml -f compose.n8n.yaml config --quiet`
- Validation JSON du workflow n8n.
- `sh -n` sur les scripts modifiés.
- Callback daily summary avec bon et mauvais secret.

## Plan de test production simulée

- `docs:check` valide EPIC-030, NEWS-184 à NEWS-188, docs n8n et compose n8n.
- `deploy:main:backend` choisit les fichiers compose selon
  `FILFINANCE_N8N_PROD_ENABLED`.
- `smoke_backend_runtime.sh` ignore n8n quand le flag est absent ou faux.
- `smoke_backend_runtime.sh` vérifie n8n quand le flag est vrai.

## Non-régression attendue

- Filfinance fonctionne avec `FILFINANCE_N8N_DAILY_SUMMARY_WEBHOOK_URL` vide.
- Filfinance continue si n8n ou OpenRouter est indisponible.
- Aucune news live n'est envoyée à n8n.
- Aucun secret réel n'est committé.
