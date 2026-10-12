# NEWS-164 - Implementation cache playlists HLS Worker

## Objectif

Implementer le cache tres court des playlists dans `filfinance-edge`, en conservant le
comportement actuel des segments.

## Scope

- Lire `FILFINANCE_HLS_PLAYLIST_CACHE_TTL_SECONDS`.
- Mettre en cache les playlists `200` apres reecriture `/audio/...` vers `/hls/audio/...`.
- Conserver la query string dans la cache key Worker.
- Ne jamais stocker les erreurs origin.
- Ajouter les headers de diagnostic playlist.
- Ajouter les tests unitaires Worker.

## Implementation cible

Dans `filfinance-edge/src/hls-proxy.js`:

1. Detecter les playlists avec `isHlsPlaylistPath`.
2. Si TTL absent ou `0`, conserver le comportement actuel `BYPASS_PLAYLIST_DISABLED`.
3. Si TTL actif:
   - chercher `caches.default.match(cacheKey)`;
   - retourner `PLAYLIST_HIT` si present;
   - sinon fetch origin;
   - reecrire le body playlist;
   - si origin `200`, stocker une copie avec `Cache-Control: public, max-age=<ttl>`;
   - retourner `PLAYLIST_MISS`.
4. Conserver le cache segment `.ts` existant.

Pseudo-contrat:

```text
cacheKey = Request(request.url, { method: "GET" })
```

La query `hls_ticket` reste donc dans la cle.

## Tests attendus

- TTL absent: playlist bypass.
- TTL `0`: playlist bypass.
- TTL `2`: playlist cacheable.
- TTL trop haut: clamp a `5`.
- Reponse `200`: stockable.
- Reponse `401`: non stockable.
- Playlist reecrite avant stockage.
- Segment `.ts`: comportement existant inchange.

## Criteres d'acceptation

- `npm run check` passe dans `filfinance-edge`.
- Une premiere requete playlist renvoie `PLAYLIST_MISS`.
- Une deuxieme requete meme URL dans le TTL renvoie `PLAYLIST_HIT`.
- Une requete sans ticket ne peut pas reutiliser une playlist valide.
- Les segments gardent `HIT` / `MISS`.

## Dependances

- `EPIC-027`
- `NEWS-163`
