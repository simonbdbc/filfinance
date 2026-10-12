# NEWS-185 - Persistence du résumé IA journalier

## Objectif

Stocker les runs de résumé journalier de manière idempotente dans SQLite.

## Scope

- Table `daily_summaries`.
- Clé fonctionnelle `local_day, content_hash`.
- Statuts: `missing`, `pending`, `summarized`, `audio_generating`,
  `audio_ready`, `error`.
- Stockage du texte résumé, narration, provider, modèle, latence, erreur et
  playlist HLS finale.

## Critères d'acceptation

- Rejouer la même journée avec le même contenu ne relance pas n8n si un run est
  déjà `pending`, `audio_generating` ou `audio_ready`.
- Si les news du jour changent, le `content_hash` crée une nouvelle version.
- `news_translations` reste le contrat stable du live.
