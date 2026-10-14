# Fil Finance Edge

Worker Cloudflare de metadata live pour `EPIC-018`.

## Endpoints

- `GET /health`: verification publique du Worker.
- `GET /ws/live?ticket=...`: WebSocket de metadata live, porte par le Durable Object `filfinance-live`.
- `POST /publish/live`: endpoint serveur de publication metadata. Il exige une signature HMAC avec `FILFINANCE_EDGE_PUBLISH_SECRET`.
- `GET /hls/audio/...`: proxy HLS vers l'origin Rust. Le Worker transmet `hls_ticket` a l'origin, ne valide pas le ticket lui-meme, met en cache les segments `.ts` et peut mettre les playlists en cache tres court.

## Variables

- `LIVE_OBJECT_NAME`: nom du Durable Object, par defaut `filfinance-live`.
- `FILFINANCE_HLS_PROXY_ENABLED`: active explicitement le proxy HLS si la valeur vaut
  `true`. Toute autre valeur ferme `/hls/audio/...` avec `410 Gone`.
- `FILFINANCE_HLS_ORIGIN_BASE_URL`: base HTTPS de l'origin HLS Rust, par defaut `https://api.filfinance.fr`.
- `FILFINANCE_HLS_PLAYLIST_CACHE_TTL_SECONDS`: TTL tres court des playlists HLS. `0` ou vide desactive; `1` ou `2` active; le code limite a `5`.

## Configuration locale

Les configurations et commandes de publication/cloud/secrets sont conservées dans GitLab privé. Les tests utilisent uniquement des fixtures et fonctions locales ; aucune connexion au Worker distant.

## Publication backend

Le backend signe chaque body brut avec une horloge courte:

```text
X-FilFinance-Timestamp: 2026-05-09T09:12:00.000Z
X-FilFinance-Signature: sha256=<base64url-hmac>
```

La signature est:

```text
HMAC_SHA256(secret, timestamp + "." + raw_body)
```

Le Worker refuse une signature absente, invalide ou hors fenetre de validite.

## Proxy HLS

Le proxy HLS est ferme par defaut depuis `EPIC-034`, apres mitigation Cloudflare Trust &
Safety sur la livraison media. `live.filfinance.fr` reste le control plane Edge
metadata. Ne reactiver `FILFINANCE_HLS_PROXY_ENABLED=true` qu'apres validation d'une
cible media conforme.

Le proxy HLS expose les chemins suivants:

```text
/hls/audio/replay/today/playlist.m3u8
/hls/audio/replay/today/segments/{local_day}/{segment_name}
/hls/audio/clips/{stream_id}/playlist.m3u8
/hls/audio/clips/{stream_id}/{segment_name}
```

Le backend Rust reste responsable de la securite HLS: le client demande
`GET /api/hls-ticket`, puis ajoute `hls_ticket` a la playlist. Le Worker transmet
la query string a l'origin.

Les playlists renvoyees par l'origin sont reecrites par le Worker: les lignes de segments
`/audio/...` deviennent `/hls/audio/...`, afin que le player repasse par le proxy pour
les segments.

Le header `X-FilFinance-HLS-Proxy-Cache` permet de verifier le comportement:

- `HIT`: segment servi depuis le cache Worker;
- `MISS`: segment recupere depuis l'origin puis stocke;
- `PLAYLIST_HIT`: playlist servie depuis le cache Worker;
- `PLAYLIST_MISS`: playlist recuperee depuis l'origin, reecrite puis stockee;
- `BYPASS_PLAYLIST_DISABLED`: cache playlist desactive;
- `BYPASS_ORIGIN_ERROR`: reponse origin non `200`, non stockee;
- `BYPASS_RANGE` ou `BYPASS_METHOD`: requete non cacheable.

## Validation locale

```bash
npm run check
```

