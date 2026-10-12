# EPIC-030 - Résumé IA journalier n8n/OpenRouter

## Objectif

Sortir OpenRouter du pipeline live et l'utiliser pour un traitement différé:
produire un vrai résumé audio de la journée courante, enrichi par une allowlist
de sources web.

Le live reste volontairement simple:

```text
ingestion -> traduction backend -> Piper -> HLS
```

Le résumé journalier suit un chemin séparé:

```text
scheduler backend -> n8n AI Agent OpenRouter qualification
  -> callback qualifications -> Telegram important
  -> résumé OpenRouter si contexte absent ou hash qualifié modifié
  -> callback texte backend -> Piper -> HLS résumé
```

## Décisions

- Piper est le seul TTS supporté.
- La voix par défaut est `NEWS_TTS_VOICE=fr_FR-upmc-medium-jessica`.
- n8n ne reçoit plus les news live une par une.
- Le backend déclenche n8n toutes les 120 secondes par défaut si une nouvelle
  ingestion, un nouveau jour ou un résumé courant incomplet est détecté.
- n8n ne transporte jamais d'audio.
- n8n envoie Telegram à l'étape de qualification pour les nouvelles news
  importantes renvoyées par le backend.
- Les sources web complémentaires viennent uniquement de
  `FILFINANCE_DAILY_SUMMARY_SOURCE_URLS`.
- Le callback interne est `POST /api/internal/n8n/daily-summary-callback`.
- L'idempotence est portée par `daily_news_qualifications(local_day, news_event_id)`
  et `daily_summaries(local_day, content_hash)`.

## Sécurité

- Aucun secret réel n'est committé.
- Le webhook et le callback utilisent `X-Filfinance-Webhook-Secret`.
- Le credential OpenRouter reste dans l'UI n8n.
- `FILFINANCE_N8N_PROD_ENABLED=true` ajoute n8n au deploy `main`.

## Definition of Done

- Les variables de modes audio live alternatifs sont absentes.
- Le workflow n8n actif est `filfinance_daily_summary_openrouter.json`.
- Le frontend lit `GET /api/daily-summary/today`, puis lit `playlist_url`.
- Le bouton ne retombe pas sur l'ancien replay des clips.
- Le résumé n'est jamais vide: en absence de news importante, il contient le
  titre du jour, le contexte de marché et une phrase claire côté news
  importantes.
- La documentation couvre setup, callback, production et rollback.

## Références

- [README n8n Filfinance](../../n8n/docs/README_N8N_FILFINANCE.md)
- [ENVIRONMENT-FILES](../ENVIRONMENT-FILES.md)
- [PRODUCTION-FINAL-RUNBOOK](../PRODUCTION-FINAL-RUNBOOK.md)
