# NEWS-194 - Observabilité et tests résumé journalier

## Objectif

Rendre visibles les décisions du résumé journalier et couvrir les règles de
non-régression live.

## Scope

- Ajouter ou conserver des logs structurés sur:
  - nombre de news du jour;
  - nombre de news non qualifiées;
  - nombre de news importantes;
  - nouvelles news importantes envoyées à Telegram;
  - hash courant;
  - décision: reuse, pending, rebuild, error.
- Tester:
  - hash change quand les qualifications changent;
  - callback résumé sans contexte rejeté;
  - news importantes triées ASC;
  - `GET /api/daily-summary/today` planifie un run idempotent;
  - live sans n8n configuré.
- Valider le workflow JSON n8n et ses code nodes.

## Critères D'Acceptation

- `cargo test` passe.
- Le workflow n8n est un JSON valide et les nodes Code sont syntaxiquement
  valides.
- La documentation explique pourquoi OpenRouter est réservé au résumé différé.
- Le runbook indique comment revenir à un état sûr sans toucher au pipeline
  live.
