# NEWS-191 - Qualification complète des news du jour

## Objectif

À chaque lancement d'app et à chaque nouvelle ingestion, vérifier que toutes les
news du jour sont qualifiées, puis qualifier uniquement les manquantes.

## Scope

- `GET /api/internal/n8n/daily-summary/news` renvoie les news du jour avec leur
  état de qualification existant.
- n8n/OpenRouter qualifie uniquement les news non qualifiées.
- Le callback `phase=qualifications` upsert les qualifications de façon
  idempotente.
- Le backend refuse de produire le résumé IA tant qu'il reste des news du jour
  non qualifiées.
- Les news importantes sont triées par `published_at ASC`, puis date
  d'ingestion, puis `news_event_id`.

## Critères D'Acceptation

- Une news déjà qualifiée n'est pas retraitée inutilement.
- Une news non qualifiée force `summary_required=false` et
  `status=qualification_incomplete` jusqu'à complétion.
- Telegram reste basé sur `newImportantNews` pour éviter les doublons.
- Le résumé final voit toutes les news importantes du jour, anciennes et
  nouvelles.
