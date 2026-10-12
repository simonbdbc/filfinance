# EPIC-027 - Cache court des playlists HLS Worker

## Objectif

Reduire la pression origin et la latence des playlists HLS en ajoutant un cache tres court
cote Worker, sans affaiblir le modele de securite HLS par `hls_ticket`.

Les runs de charge du 11 mai 2026 valident les points critiques suivants:

- segments HLS servis en `200`;
- cache segments Worker majoritairement `HIT`;
- cache playlist Worker majoritairement `PLAYLIST_HIT`;
- zero erreur segment a 500 puis 1000 auditeurs;
- zero erreur playlist a 1000 auditeurs;
- test global `ok=true` apres lissage du demarrage.

Le cache tres court des playlists a ete ajoute pour eviter que chaque poll HLS retourne
a l'origin Rust. Le palier 1000 auditeurs HLS replay est valide avec `ok=true`.

## Constat initial

Etat du Worker HLS avant `EPIC-027`:

- `/hls/audio/...` proxy vers l'origin Rust;
- l'origin Rust valide `hls_ticket`;
- les playlists sont reecrites pour renvoyer les segments vers `/hls/audio/...`;
- les segments `.ts` sont caches par le Worker Cache API;
- les playlists etaient toujours bypass cache.

Resultat de charge avant cache playlist observe a 500 auditeurs:

- `playlist.requests=4064`;
- `playlist.failures=0`;
- `playlist.hls_proxy_cache_status.BYPASS_PLAYLIST=4064`;
- `playlist.p95=16532ms`;
- `playlist.p99=61905ms`;
- `segment.hls_proxy_cache_status.HIT=11844`;
- `segment.hls_proxy_cache_status.MISS=348`.

La charge segment etait absorbee. Le gain necessaire visait donc les playlists.

Resultat de charge apres cache playlist observe a 1000 auditeurs:

- `playlist.requests=40067`;
- `playlist.failures=0`;
- `playlist.hls_proxy_cache_status.PLAYLIST_HIT=38974`;
- `playlist.hls_proxy_cache_status.PLAYLIST_MISS=1093`;
- `playlist.p95=42.61ms`;
- `playlist.p99=90.03ms`;
- `segment.requests=40067`;
- `segment.failures=0`;
- `segment.hls_proxy_cache_status.HIT=40064`;
- `segment.hls_proxy_cache_status.MISS=3`.

La charge playlist et segment est maintenant absorbee par le Worker dans le scenario replay.

## Decision cible

Ajouter un cache Worker tres court pour les playlists HLS:

- TTL cible: `1s` a `2s`;
- cache key: URL complete, query string incluse, donc `hls_ticket` conserve;
- stockage uniquement des reponses playlist `200`;
- aucun cache des `401`, `403`, `404` ou `5xx`;
- aucun cache des requetes `Range`, `HEAD`, methodes non `GET`;
- reecriture playlist appliquee avant stockage;
- header de diagnostic explicite pour distinguer `PLAYLIST_HIT`, `PLAYLIST_MISS`, `BYPASS_*`.

Le ticket HLS reste valide par l'origin Rust sur cache miss. Sur cache hit, le risque
d'autorisation stale est borne par le TTL tres court et par la cache key qui conserve le
ticket complet.

## Hors scope

- Ignorer `hls_ticket` dans la cache key.
- Deplacer la validation HLS dans le Worker.
- Mettre les playlists en cache long.
- Changer la duree des segments HLS.
- Migrer les segments vers R2.
- Configurer le cache Cloudflare sur domaine API proxifie.

## Tickets de dev

- `NEWS-163`: contrat et configuration du cache court playlist Worker.
- `NEWS-164`: implementation du cache playlist dans `filfinance-edge`.
- `NEWS-165`: adaptation load test et validation 500/1000 auditeurs.
- `NEWS-166`: runbook deploiement, rollback et monitoring playlist cache.

## Definition of Done

- le cache playlist est desactive par defaut ou controle par variable Worker;
- `FILFINANCE_HLS_PLAYLIST_CACHE_TTL_SECONDS=1` ou `2` active le cache;
- les playlists `200` sont servies en `PLAYLIST_HIT` apres warmup;
- les erreurs auth et origin ne sont jamais stockees;
- les segments gardent leur comportement actuel;
- le load test 500 reste `ok=true`;
- le palier 1000 replay HLS Worker est mesure, archive et `ok=true`;
- le rollback est documente: TTL a `0` puis redeploiement Worker.

## References

- [NEWS-159 - Systeme securite HLS CDN](../tickets/NEWS-159-systeme-securite-hls-cdn.md)
- [NEWS-162 - Plan charge HLS CDN](../tickets/NEWS-162-plan-charge-hls-cdn.md)
- [EPIC-026 - Securite et charge production](./EPIC-026-securite-et-charge-production.md)
