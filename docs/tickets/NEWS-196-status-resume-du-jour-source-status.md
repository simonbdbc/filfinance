# NEWS-196 - Statut public live du résumé calendrier du jour

## Objectif

Transmettre le statut compact du résumé calendrier du jour dans le control plane
live public déjà utilisé pour les badges sources.

Le bouton `Résumé audio du jour` doit connaître la disponibilité audio du résumé
sans lancer de polling récurrent sur `GET /api/calendar-day/today/summary`.

## Contrat public

Le tableau `sources` reste inchangé. Le payload `hello`, `heartbeat` et
`source_status` ajoute un champ global:

```json
{
  "calendar_day_summary": {
    "local_day": "2026-06-17",
    "status": "audio_ready",
    "audio_available": true,
    "updated_at": "2026-06-17T08:00:00Z"
  }
}
```

`audio_available=true` seulement si le statut est `audio_ready` et qu'une playlist
HLS exploitable existe. Le payload public ne contient ni URL HLS, ni titre, ni
texte de résumé, ni erreur détaillée.

## Scope

- Backend Rust: construire le statut compact depuis le store `calendar_day`.
- WebSocket backend: exposer `calendar_day_summary` dans `hello` et `heartbeat`.
- Edge/fallback: exposer le même champ dans `source_status`, le snapshot edge et
  les heartbeats edge.
- Client Expo: consommer ce champ comme source de vérité de disponibilité du
  bouton `Résumé audio du jour` et garder le fetch HTTP uniquement au clic de
  lecture.

## Hors scope

- Migration DB.
- Nouvelle route HTTP.
- Changement du format des badges sources.
- Exposition publique des détails du résumé ou des playlists.

## Critères d'acceptation

- Le feed admin conserve uniquement les badges sources existants: aucun badge ni
  pseudo-source n'est créé pour le résumé.
- Le bouton `Résumé audio du jour` devient prêt quand `audio_available=true`.
- Le chargement simple de l'app ne déclenche plus de boucle
  `/api/calendar-day/today/summary`.
- Le clic de lecture valide toujours la playlist via HTTP avant de lancer HLS.
- Les anciens payloads sans `calendar_day_summary` restent tolérés côté client et
  Worker edge.

## Validation attendue

- `cargo test` côté `filfinance-service`.
- `npm run check` côté `filfinance-edge`.
- `npm run typecheck`, `npm run qa:audio-legacy` et `npm run qa:frontend` côté
  `filfinance-client`.
