# NEWS-159 - Systeme securite HLS CDN

## Objectif

Proteger l'acces audio HLS sans perdre le benefice du cache Cloudflare CDN.

Les URLs HLS ne doivent plus etre exposees comme ressources publiques nues dans le mode production durci.

## Scope

- Definir un ticket HLS court emis apres authentification App.
- Proteger les playlists clips et replay.
- Proteger les segments sans casser le cache CDN des segments immuables.
- Differencier:
  - playlist clip mutable;
  - playlist clip finalisee;
  - playlist replay du jour;
  - segments immuables.
- Documenter les Cache Rules Cloudflare cible.
- Adapter le client audio pour utiliser une URL HLS avec ticket.

## Hors scope

- LL-HLS complet.
- Stockage R2 obligatoire.
- Changement du format audio.
- Transport audio binaire.

## Modele cible

```text
Frontend authentifie
  -> Backend Rust: demande ticket HLS court
  -> HLS Worker/CDN: charge playlist avec ticket
  -> Backend Rust: valide le ticket HLS
  -> HLS Worker/CDN: cache les segments immutable
  -> Rust origin: sert seulement les misses
```

## Options d'implementation

### Option A - Ticket en query

```text
/audio/clips/{stream_id}/playlist.m3u8?hls_ticket=...
/audio/clips/{stream_id}/segment-001.ts?hls_ticket=...
```

Simple a brancher, mais le token apparait dans les URLs et les logs.

### Option B - Worker HLS avec ticket origin

```text
/hls/audio/clips/{stream_id}/playlist.m3u8?hls_ticket=...
/hls/audio/clips/{stream_id}/segment-001.ts?hls_ticket=...
```

Le Worker proxy vers l'origin Rust. L'origin conserve la validation du ticket, ce qui evite
de dupliquer la logique de securite HLS dans le Worker. Le Worker cache uniquement les
segments `.ts`.

## Implementation retenue

NEWS-159 implemente d'abord l'option A cote Rust et frontend:

- `GET /api/hls-ticket` est une route App securisee par bearer Supabase `listener`.
- Si `NEWS_HLS_ACCESS_CONTROL_ENABLED=false`, la route repond `enabled=false` et le client conserve les URLs nues.
- Si `NEWS_HLS_ACCESS_CONTROL_ENABLED=true`, le backend emet un JWT HMAC HS256 avec scope `hls_audio`.
- Le ticket est partage par bucket temporel, sans identite utilisateur, afin de ne pas creer un cache CDN par utilisateur.
- Les playlists `/audio/clips/*/playlist.m3u8` et `/audio/replay/today/playlist.m3u8` refusent les requetes sans `hls_ticket` valide.
- Les playlists repondent avec les URLs de segments deja enrichies en `hls_ticket`.
- Les segments `/audio/clips/*/*.ts` et `/audio/replay/today/segments/*/*.ts` refusent aussi les requetes sans ticket valide.

Le volet CDN actuel ajoute ensuite l'option B comme point d'entree public HLS:

- le Worker Edge expose `/hls/audio/...`;
- il strippe le prefixe `/hls` et proxy vers `/audio/...` sur l'origin Rust;
- il transmet la query `hls_ticket` sans l'interpreter;
- il ne met pas les playlists en cache applicatif;
- il met en cache les segments `.ts` via le Cache API Worker;
- le header `X-FilFinance-HLS-Proxy-Cache` indique `HIT`, `MISS`, `BYPASS_PLAYLIST`, `BYPASS_RANGE` ou `BYPASS_METHOD`.

Variables:

```env
NEWS_HLS_ACCESS_CONTROL_ENABLED=true
NEWS_HLS_ACCESS_TICKET_SECRET=replace-with-at-least-32-random-characters
NEWS_HLS_ACCESS_TICKET_TTL_SECS=3600
```

La valeur par defaut reste `NEWS_HLS_ACCESS_CONTROL_ENABLED=false` pour ne pas casser les environnements locaux. En production durcie, elle doit etre activee.

Variable client:

```env
EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL=https://media.filfinance.fr
```

Depuis `EPIC-034`, le Worker `live.filfinance.fr` ne doit plus etre utilise comme base
HLS. En production, la cible media conforme est Bunny
`https://media.filfinance.fr`. Si cette variable est vide pendant un rollback, le client
conserve les URLs HLS directes sur le backend Rust.

## Cache cible

| Ressource | Cache navigateur | Cache CDN |
| --- | --- | --- |
| Playlist clip mutable | `no-cache` | tres court |
| Playlist clip finalisee | court a moyen | court a moyen |
| Playlist replay | tres court | tres court |
| Segment clip/replay | long immutable | long immutable |

Avec le Worker HLS proxy, la cache key conserve la query string: le ticket etant partage
par bucket, la multiplication du cache se fait par fenetre de temps, pas par utilisateur.
Ne pas configurer une cache key qui ignore `hls_ticket` tant que la validation est faite
par l'origin Rust, sinon un segment deja cache pourrait etre servi a une URL non autorisee.

Le cache Cloudflare direct sur le domaine API/HLS proxie reste une amelioration future si
`filfinance-api.sbcenter.fr` migre vers Cloudflare DNS proxifie.

## Criteres d'acceptation

- Une playlist HLS sans ticket valide est refusee.
- Un segment HLS sans ticket valide est refuse.
- Les segments restent cacheables par Cloudflare apres validation.
- Le cache key CDN ne cree pas une copie par utilisateur pour les segments immuables.
- Les URLs nues HLS ne sont plus utilisees par le client en production durcie.
- Le player HLS continue de lire les clips sur `audio_ready`.

## Tests attendus

- Tests backend ticket HLS valide, expire, invalide.
- Tests Worker HLS proxy mapping, allowlist et cacheability.
- Tests headers cache playlist/segments.
- Smoke navigateur lecture clip.
- Load test HLS CDN avec verification `CF-Cache-Status`.

## Dependances

- `EPIC-026`
- `EPIC-021`
- Worker Edge deploye
- `EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL` configure cote frontend pour utiliser le proxy
