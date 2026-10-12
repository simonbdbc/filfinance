# NEWS-103 - Client player HLS

## Objectif

Brancher le client web sur le transport audio cible de la refonte:

- live audio global via `/audio/live/playlist.m3u8`;
- replay unique du jour via `/audio/replay/today/playlist.m3u8`;
- fallback temporaire `/ws/audio` uniquement si HLS n'est pas disponible cote navigateur.

## Implementation

Le client ajoute `hls.js` pour couvrir les navigateurs sans support HLS natif.

Le hook `useHlsAudioPlayer` gere:

- creation d'un `HTMLAudioElement` invisible;
- lecture HLS native quand le navigateur sait lire `application/vnd.apple.mpegurl`;
- lecture via `hls.js` sinon;
- etats `idle`, `connecting`, `open`, `error`;
- modes `live` et `replay_today`;
- nettoyage complet de l'instance HLS et de l'element audio.

Le hook `useAudioPlayback` devient l'orchestrateur de transport:

- mode nominal: `audioTransportMode = hls`;
- fallback: `audioTransportMode = binary_ws`;
- mode inactif: `audioTransportMode = idle`.

## Live

Quand l'utilisateur active le switch audio:

1. le client stoppe toute lecture precedente;
2. le client tente la playlist live HLS;
3. si HLS demarre, aucun `/ws/audio` n'est ouvert;
4. si HLS echoue, le client tente le fallback binaire historique.

En mode HLS, la file audio par news est desactivee: les news continuent d'entrer dans l'historique, mais elles ne sont plus admises dans la queue `/ws/audio`.

## Replay du jour

Les boutons de replay des cartes d'historique ne reconstruisent plus un replay par IDs.

Ils lancent le replay HLS unique du jour:

1. lecture de `GET /api/replay/today`;
2. refus si le replay est desactive ou vide;
3. lecture de la playlist retournee par `playlist_url`;
4. aucun historique multi-jours et aucun clip par news.

## Synchronisation metadata

Le client accepte maintenant les messages edge:

- `now_playing`;
- `audio_state`;
- snapshot `hello.payload.snapshot`.

Quand le client est en HLS live, `now_playing.translation_id` est resolu dans les items deja connus pour alimenter la card "Item en lecture".

## UI

La carte de connexion expose:

- transport audio courant: `HLS`, `WS binaire`, `idle`;
- mode HLS: `live`, `replay jour`, `idle`;
- moteur: `HLS.js`, `HLS natif`, `AudioWorklet`, `BufferSource`;
- URLs live/replay HLS;
- nombre de segments replay connus apres chargement metadata.

Le selecteur de voix reste visible mais signale que le changement de voix a chaud est reserve au fallback `/ws/audio`. Le flux HLS est un flux global serveur.

## Validation

Validation automatique:

```bash
cd filfinance-client
npm run typecheck
npm run prebuild:web
npm run build:web
```

Validation live:

- ouvrir le client web;
- activer "Lecture audio";
- verifier que le transport affiche `HLS`;
- verifier que l'URL active est `/audio/live/playlist.m3u8`;
- lancer un replay depuis l'historique;
- verifier que le mode affiche `replay jour`;
- verifier qu'aucune nouvelle connexion `/ws/audio` n'est ouverte en mode HLS nominal.

## Limites

- La synchronisation `now_playing` ne peut afficher un item courant que si `translation_id` existe deja dans le cache client.
- Le fallback `/ws/audio` reste disponible pendant la transition, puis devra etre retire ou reserve au debug.
- Le load test 1000 auditeurs reste couvert par `NEWS-104`.
