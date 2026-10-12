# NEWS-096 - Client WebSocket edge metadata

## Objectif

Permettre au client Expo Web de consommer le WebSocket metadata Cloudflare Durable Objects sans supprimer le fallback backend `/ws/news`.

La bascule doit rester progressive:

- sans `EXPO_PUBLIC_FILFINANCE_EDGE_WS_URL`, le client garde le comportement actuel;
- avec `EXPO_PUBLIC_FILFINANCE_EDGE_WS_URL`, le client tente l'edge;
- si l'edge ferme ou echoue, le client retombe sur le WebSocket backend configure par `EXPO_PUBLIC_FILFINANCE_WS_URL`.

## Implementation

Fichiers principaux:

- `filfinance-client/src/shared/api/urls.ts`
  - ajout de `defaultEdgeLiveWsUrl()`.
- `filfinance-client/src/shared/api/contracts.ts`
  - ajout de `IncomingLiveMetadataMessage`, union du contrat backend actuel et du contrat edge.
- `filfinance-client/src/features/live-feed/useNewsSocket.ts`
  - support du payload edge `payload.item`;
  - consommation des `translated_news_item` du snapshot de reconnexion edge;
  - recuperation du ticket court edge si l'URL edge est configuree;
  - fallback automatique vers le backend;
  - exposition de l'URL metadata active et du mode fallback.
- `filfinance-client/src/app/FilFinanceApp.tsx`
  - separation entre URL backend API/audio/tests et URL metadata live.
- `filfinance-client/src/features/settings/ConnectionSettingsCard.tsx`
  - affichage du mode metadata actif: `backend`, `edge` ou `fallback backend`.

## Variables

Cloudflare Pages et `filfinance-client/.env`:

```env
EXPO_PUBLIC_FILFINANCE_WS_URL=wss://filfinance-api.sbcenter.fr/ws/news
EXPO_PUBLIC_FILFINANCE_EDGE_WS_URL=
```

Quand le Worker edge sera pret a valider en live:

```env
EXPO_PUBLIC_FILFINANCE_EDGE_WS_URL=wss://filfinance-live-edge.your-cloudflare-account.workers.dev/ws/live
```

`EXPO_PUBLIC_FILFINANCE_WS_URL` reste obligatoire car il porte encore:

- les API HTTP derivees;
- `/ws/audio`;
- `/ws/test-runs`;
- le fallback metadata.

## Compatibilite contrat

Backend actuel:

```json
{ "type": "translated_news_item", "payload": { "id": 42 } }
```

Edge metadata:

```json
{
  "type": "translated_news_item",
  "payload": {
    "event_id": "translated_news_item:00000000000000000042",
    "server_time": "2026-05-03T08:00:00Z",
    "item": { "id": 42 }
  }
}
```

Le client normalise les deux formats vers `TranslatedNewsItem`.

## Historique journalier

Le comportement de reference reste celui de la branche backup `backup/main-prod-before-epic18-2026-05-03`:

- `/api/translations` charge la base de l'historique journalier;
- `translated_news_item` met a jour l'historique en temps reel;
- le snapshot edge doit rejouer les derniers `translated_news_item` apres reconnexion;
- les evenements audio `audio_ready`, `audio_failed` et `audio_finalized` ne modifient jamais l'historique visible.

Cette separation evite de coupler l'historique au pipeline TTS/HLS. Une news peut etre visible dans l'historique meme si elle n'est pas admise dans la queue audio, ou si sa generation audio echoue.

## Validation

Commandes:

```bash
cd filfinance-client
npm run typecheck
npm run build:web
```

Critères d'acceptation:

- sans URL edge, le client reste sur le WebSocket backend;
- avec URL edge, les cards live peuvent arriver depuis Durable Objects;
- avec URL edge, une reconnexion rattrape les derniers `translated_news_item` via le snapshot;
- si l'edge est indisponible, le client reconnecte le fallback backend;
- les erreurs edge sont visibles dans le resume systeme via le hook live.
- `audio_ready` ne declenche aucun refresh de `/api/translations`.

## Suite

- `NEWS-097`: ticket edge court signe cote backend pour proteger `/ws/live`.
- `NEWS-098`: load test metadata 1000 clients.
