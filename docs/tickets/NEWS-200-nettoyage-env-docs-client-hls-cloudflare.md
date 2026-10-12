# NEWS-200 - Nettoyage env et docs client HLS Cloudflare

## Objectif

Supprimer les exemples qui font passer la lecture HLS client par `live.filfinance.fr`.

## Scope

- Garder `EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL=` vide dans les exemples production.
- Documenter que la valeur vide fait retomber le client sur l'API backend derivee de
  `EXPO_PUBLIC_FILFINANCE_WS_URL`.
- Mettre a jour les runbooks prod et migration.
- Marquer les anciennes references Worker HLS comme historiques ou interdites a relancer.

## Criteres d'acceptation

- Aucun runbook production ne recommande une base HLS publique sur `live.filfinance.fr`.
- `live.filfinance.fr` est documente comme control plane Edge uniquement.
- Les docs expliquent que le choix media long terme est hors scope de la coupure.

## Dependances

- `EPIC-034`
- `NEWS-199`
