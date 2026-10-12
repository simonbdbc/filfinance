# EPIC-032 - Refonte Calendrier Du Jour n8n + Audio Piper

> Document historique, supersédé par
> [`EPIC-037`](./EPIC-037-orchestrateur-rust-natif-calendrier.md).
> Le parcours canonique est désormais Rust natif ; workflows, endpoints et
> variables n8n actifs sont retirés du dépôt. Le texte ci-dessous conserve le
> processus historique n8n, pas une procédure d'exploitation actuelle ni une
> référence de résultat. La livraison du nouveau SHA et les contrôles PROD
> restants sont suivis exclusivement dans EPIC-037.

## Objectif

Remplacer le workflow lourd de résumé du jour par un système séparé du live news:

- pipeline live inchangé: ingestion news -> traduction backend -> Piper -> HLS;
- calendrier du jour via n8n: ingestion calendriers -> fusion -> qualification -> stockage;
- résumé calendrier via n8n/OpenRouter;
- génération audio Piper côté Rust;
- alertes HLS T-5 et T-1 pour les événements high.

## Architecture

Trois composants coopèrent sans bloquer le hot path live:

1. Scheduler Rust `calendar_day`
   - generation automatique le samedi entre `02:00` et `04:00` Europe/Paris pour lundi-vendredi de la semaine ISO suivante;
   - verification automatique le dimanche entre `02:00` et `04:00` Europe/Paris avec regeneration des jours incomplets;
   - fenetre quotidienne `01:30` -> `03:00` conservee uniquement pour compatibilite et etat admin legacy;
   - callbacks protégés par `X-Filfinance-Webhook-Secret`;
   - tables SQLite `calendar_day_*`.

2. Workflows n8n
   - `filfinance_calendar_day_ingest.json`;
   - `filfinance_calendar_day_summary.json`;
   - normalisation dans des nodes `Code`, sans `Structured Output Parser`.

3. Audio Piper
   - worker `CalendarDayAudioWorker` séparé du live;
   - HLS pour chaque événement high;
   - HLS pour le résumé du calendrier;
   - publication `calendar_alert_ready` pour les alertes.

## Contrats

Endpoints internes:

- `GET /api/internal/n8n/calendar-day/sources`
- `GET /api/internal/n8n/calendar-day/items`
- `POST /api/internal/n8n/calendar-day/callback`

Endpoints produit/admin:

- `GET /api/calendar-day/today`
- `GET /api/calendar-day/today/summary`
- `GET /api/admin/calendar-day/today`
- `POST /api/admin/calendar-day/today`
- `GET /ws/admin/calendar-day`

Control plane live public:

- `calendar_day_summary` dans `hello`, `heartbeat` et `source_status`
- statut compact uniquement: `local_day`, `status`, `audio_available`, `updated_at`
- aucune URL HLS, texte de résumé, titre ou erreur détaillée dans ce payload public

## Sécurité

- Aucun secret réel dans Git.
- n8n reçoit et renvoie le header `X-Filfinance-Webhook-Secret`.
- Les artifacts admin sont redacted avant stockage.
- n8n/OpenRouter restent hors ingestion live.

## Definition Of Done

- Le live news fonctionne sans n8n.
- Le scheduler calendrier est désactivable par `FILFINANCE_CALENDAR_DAY_ENABLED=false`.
- Les deux workflows n8n sont importables.
- Le bouton produit lit seulement un résumé HLS prêt.
- L’admin calendrier suit runs, étapes, artifacts, événements, résumé et alertes.
- Les alertes `calendar_alert_ready` passent après le clip courant, sans interruption.
- Le statut public du résumé du jour est visible via le flux live sans polling HTTP.
