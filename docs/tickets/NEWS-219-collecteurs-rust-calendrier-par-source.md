# NEWS-219 - Collecteurs Rust calendrier par source

## Objectif

Remplacer progressivement les fetches n8n par des adaptateurs Rust explicites,
une source a la fois.

## Scope

- Trait commun d'adaptateur et snapshots traces par source.
- Verification native de couverture, horaires, valeurs, provenance et
  deduplication.
- Limitation de concurrence par host, timeout et erreurs classees.

## Criteres d'acceptation

- Chaque source possede fixtures et tests de contrat natifs avant activation.
- Une source native devient nominale dans le parcours Rust complet ; son
  resultat n'est pas compare a n8n et la validation finale est le test de
  recuperation complet local de `EPIC-037`.
- Aucune protection anti-bot n'est contournee.

## Dependances

- [`NEWS-217`](./NEWS-217-contrats-rust-calendrier-parite.md)
- [`NEWS-218`](./NEWS-218-client-openrouter-rust-shadow-calendrier.md)
