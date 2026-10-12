# NEWS-192 - Assurance résumé au démarrage et lancement app

## Objectif

Faire de `GET /api/daily-summary/today` un point d'assurance idempotent: s'il
manque le contexte du jour ou les qualifications, le backend planifie un run n8n
sans bloquer la réponse HTTP.

## Scope

- Au démarrage backend, le scheduler vérifie immédiatement le jour courant.
- Au lancement frontend, le hook audio appelle `GET /api/daily-summary/today`.
- Si le résumé courant est `missing`, incomplet ou en erreur, le backend crée ou
  réutilise une ligne `pending` et déclenche n8n en tâche de fond.
- Si un run est déjà `pending` ou `audio_generating`, aucun doublon n'est lancé.

## Critères D'Acceptation

- Un nouveau jour Europe/Paris force un nouveau contexte.
- Un lancement d'app avec résumé absent planifie un run.
- Un lancement d'app avec run déjà en cours ne relance pas n8n.
- Le bouton frontend ne retombe pas sur l'ancien replay HLS des clips live.
