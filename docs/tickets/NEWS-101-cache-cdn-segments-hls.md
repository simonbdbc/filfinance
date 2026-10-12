# NEWS-101 - Cache CDN des segments HLS

## Objectif

Permettre a Cloudflare CDN d'absorber la charge des auditeurs HLS sans renvoyer chaque requete segment vers le backend Rust.

## Decision

Le backend conserve deux strategies cache separees:

- playlists live: fraiches, TTL tres court;
- segments live: immuables, TTL long.

Le cache est pilote par les headers origin. Une regle Cloudflare pourra ensuite renforcer ces TTLs si besoin, mais le backend doit deja exposer un contrat HTTP correct.

Etat actuel 2026-05-10: comme `sbcenter.fr` n'est pas encore gere par Cloudflare DNS,
le premier volet CDN passe par le Worker HLS proxy `/hls/audio/...`. La configuration
de cache sur domaine API proxifie reste une amelioration future.

## Headers live

Playlist:

```http
Content-Type: application/vnd.apple.mpegurl
Cache-Control: public, max-age=1, must-revalidate
Cloudflare-CDN-Cache-Control: max-age=1
X-Content-Type-Options: nosniff
```

Segment:

```http
Content-Type: video/MP2T
Cache-Control: public, max-age=86400, immutable
Cloudflare-CDN-Cache-Control: max-age=86400
X-Content-Type-Options: nosniff
```

## Pourquoi ce choix

Les playlists changent quand de nouveaux segments apparaissent. Elles doivent donc rester tres courtes en cache.

Les segments portent une sequence monotone dans l'URL:

```text
/audio/live/segments/{sequence}.ts
```

Une fois publie, un segment ne doit jamais changer de contenu. Il peut donc etre cache longtemps par Cloudflare et par le navigateur.

`Cloudflare-CDN-Cache-Control` est utilise pour donner une consigne edge explicite a Cloudflare sans changer la logique produit. `Cache-Control` reste present pour les navigateurs et les autres caches standards.

## Configuration Cloudflare future sur domaine API proxifie

Pour un futur domaine API/HLS proxie par Cloudflare:

1. Verifier que le DNS du sous-domaine API est proxie par Cloudflare.
2. Ne pas creer de bypass cache global qui matche `/audio/live/*`.
3. Ajouter une Cache Rule si les headers origin ne suffisent pas:
   - expression: path starts with `/audio/live/segments/`
   - eligibility: eligible for cache
   - edge TTL: 1 day
4. Ajouter une Cache Rule courte pour les playlists si necessaire:
   - expression: path equals `/audio/live/playlist.m3u8`
   - eligibility: eligible for cache
   - edge TTL: 1 second

## Validation

Validation automatique:

```bash
cd filfinance-service
cargo test
```

Resultat local:

```text
93 passed
```

Validation live apres activation HLS:

```bash
curl -I http://127.0.0.1:8088/audio/live/playlist.m3u8
curl -I http://127.0.0.1:8088/audio/live/segments/1.ts
curl -I https://filfinance-api.sbcenter.fr/audio/live/playlist.m3u8
curl -I https://filfinance-api.sbcenter.fr/audio/live/segments/1.ts
```

Verifier:

- `Cache-Control` correspond au type de ressource;
- `Cloudflare-CDN-Cache-Control` est present cote origin local;
- `Content-Type` est correct;
- `X-Content-Type-Options: nosniff` est present;
- cote URL publique Cloudflare, `Cloudflare-CDN-Cache-Control` peut ne pas etre renvoye au navigateur;
- apres plusieurs requetes segment, `CF-Cache-Status` doit tendre vers `HIT`;
- les segments chauds ne saturent pas Rust origin.

## Limites

- Cette etape ne prouve pas encore le ratio de cache hit CDN sous charge.
- Le load test 1000 auditeurs reste porte par `NEWS-104`.
- Le replay du jour reutilise la meme strategie segment immutable depuis `NEWS-102`.

## References

- Cloudflare CDN-Cache-Control: https://developers.cloudflare.com/cache/concepts/cdn-cache-control/
- Cloudflare Cache Rules: https://developers.cloudflare.com/cache/how-to/cache-rules/
- Cloudflare Origin Cache Control: https://developers.cloudflare.com/cache/concepts/cache-control/
