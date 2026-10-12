# NEWS-220 - Bascule calendrier Rust vers SQLite, audio et alertes

## Objectif

Activer le parcours calendrier Rust nominal apres validation des sources, en
reutilisant la persistence et la diffusion audio existantes.

## Scope

- Feature flag de bascule par environnement.
- Ecriture `calendar_day_*`, Piper/HLS et `calendar_alert_ready` existants.
- Idempotence, rollback vers la release precedente et surfaces
  HTTP/WebSocket/HLS/admin.

## Criteres d'acceptation

- Un run ou redemarrage ne produit aucun doublon d'item, alerte ou audio.
- Le chemin Rust seul satisfait ces assertions dans le test de recuperation
  complet local unique de `EPIC-037`.
- Le live news reste independant en cas d'erreur calendrier.

## Dependances

- [`NEWS-219`](./NEWS-219-collecteurs-rust-calendrier-par-source.md)
