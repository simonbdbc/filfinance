# EPIC-026 - Securite et charge production

## Objectif

Formaliser le modele production de Fil Finance autour de quatre surfaces clairement separees:

1. securite App: frontend vers backend Rust;
2. securite Worker Edge: Cloudflare Durable Objects et publication serveur;
3. securite HLS: audio cacheable via CDN sans URL publique nue;
4. health minimal public + health detaille admin.

Cet EPIC met aussi a jour le plan de charge: le backend Rust reste l'origin et le producteur, Cloudflare Durable Objects porte le fan-out metadata, et Cloudflare CDN absorbe les lectures HLS.

## Statut

En attente. Les paliers valides restent: HLS seul `1000` et combine Edge + HLS `500`. Le combine `1000` est non valide et laisse `EPIC-026` en attente tant que `EPIC-024`, `EPIC-005` et `EPIC-006` ne sont pas termines.

## Constat

Le fonctionnement actuel est hybride:

- le frontend appelle encore directement plusieurs routes backend;
- le WebSocket backend `/ws/news` sert de fallback live;
- les playlists et segments HLS sont publics pour permettre le cache CDN;
- `/api/sources` est protege par une session, mais pas encore reserve aux admins;
- les badges sources sont encore lies a la lecture directe backend plutot qu'au control plane live.

Ce modele fonctionne pour le MVP, mais il n'est pas assez explicite pour une montee en charge ni pour une posture de securite production.

## Architecture cible

```text
Frontend Web
  -> Backend Rust API
     Securite App: Supabase JWT + role listener/moderator/admin

Frontend Web
  -> Cloudflare Worker / Durable Object
     Securite Worker Edge: ticket live court signe par le backend

Backend Rust
  -> Cloudflare Worker / Durable Object
     Securite Worker Edge: secret serveur ou signature HMAC de publication

Frontend HLS player
  -> Cloudflare CDN / Worker HLS
     Securite HLS: ticket playback court
     Cache: segments immuables, playlists fraiches

Infra
  -> /health
     Reponse publique minimale: {"status":"ok"}
```

## Decisions

- Les routes frontend vers backend utilisent un seul systeme d'autorisation par role.
- En production, le backend refuse de demarrer si l'auth applicative est desactivee.
- `/api/sources` devient une surface admin detaillee, pas une API publique des badges.
- Les badges sources publics sont alimentes par le WebSocket live, via `hello.snapshot.sources` et `heartbeat.sources`.
- Les auditeurs ne pollent pas `/api/sources`.
- Le WebSocket public nominal est le Worker Edge; le backend `/ws/news` reste fallback ou canal de dev.
- Les evenements live metadata ne transportent jamais d'audio binaire.
- Les routes HLS ne doivent plus etre exposees sous forme d'URLs publiques nues quand l'app passe en mode production durci.
- `/health` reste public seulement si sa reponse est minimale.
- Les details d'exploitation vont dans `/api/admin/health`, reserve admin.

## Systeme 1 - Securite App frontend vers backend

Toutes les routes appelees directement par le frontend passent par la meme regle:

- JWT Supabase obligatoire;
- verification signature, expiration, issuer et audience cote Rust;
- role Fil Finance lu depuis `public.user_profiles.role`;
- autorisation par niveau `listener`, `moderator`, `admin`.

Surfaces concernees:

- `/api/news`
- `/api/translations`
- `/api/calendar-day/today`
- `/api/replay/today`
- `/api/hls-ticket`
- `/api/me`
- `/api/edge/live-ticket`
- `/api/sources`
- `/api/runtime/*`
- `/api/admin/*`
- `/ws/news`
- `/ws/test-runs`
- `/ws/admin/logs`

## Systeme 2 - Securite Worker Edge

Le Worker Edge porte deux contrats separes:

- client live: `/ws/live?ticket=...`;
- publication serveur: `/publish/live`.

Le ticket live est court, signe par le backend et contient seulement le minimum necessaire: sujet utilisateur, role/scope, expiration.

La publication serveur doit rester impossible depuis le frontend. La v1 peut conserver un secret serveur fort; la cible durcie utilise une signature HMAC du body avec timestamp pour limiter le rejeu.

## Systeme 3 - Securite HLS

Le HLS doit rester compatible CDN tout en evitant les URLs publiques nues.

Modele cible:

- le backend ou un Worker emet un ticket HLS court apres authentification;
- la playlist HLS est chargee avec ce ticket;
- les segments sont verifies par le Worker ou l'origin avant service;
- Cloudflare met en cache les segments immuables;
- les playlists mutables restent en cache tres court;
- les playlists finalisees peuvent avoir un TTL plus long.

Implementation actuelle: le backend expose `GET /api/hls-ticket` et valide `hls_ticket` sur les routes `/audio/*` quand `NEWS_HLS_ACCESS_CONTROL_ENABLED=true`. Le ticket est commun a un bucket temporel et ne porte pas l'identite utilisateur, afin de garder un cache CDN par fenetre de temps plutot que par utilisateur.

Surfaces concernees:

- `/audio/clips/{stream_id}/playlist.m3u8`
- `/audio/clips/{stream_id}/{segment_name}`
- `/audio/replay/today/playlist.m3u8`
- `/audio/replay/today/segments/{local_day}/{segment_name}`

## Systeme 4 - Health

Deux niveaux sont retenus:

- `/health`: public, minimal, ne dit que `{"status":"ok"}`;
- `/api/admin/health`: protege admin, details DB, TTS, HLS, sources, collector, Edge et stockage.

`/health` ne doit jamais exposer version, configuration, statut des sources, chemins serveur, secrets ou dependances internes.

Implementation actuelle: `/api/admin/health` est une route admin directe du backend Rust. Elle
retourne les diagnostics sous forme de statuts, compteurs et booleens de configuration, sans exposer
les chemins locaux, URLs privees, secrets, tokens, mots de passe ou erreurs brutes.

## Plan de charge cible

### Control plane metadata

Canal nominal:

```text
Backend Rust -> POST /publish/live -> Cloudflare Durable Object -> /ws/live clients
```

Objectifs:

- 1000 clients WebSocket metadata connectes au Worker Edge;
- aucune socket metadata utilisateur finale maintenue par Rust;
- 0 livraison manquante sur messages live representatifs;
- p50/p95/p99 documentes;
- reconnexion avec snapshot court.

Messages diffuses:

- `translated_news_item`
- `audio_ready`
- `audio_failed`
- `audio_finalized`
- `system_notice`
- `heartbeat`
- `source_status` ou `sources` dans `hello`/`heartbeat`

Implementation actuelle: `filfinance-edge/scripts/load-test-live.js` accepte
`FILFINANCE_EDGE_LOAD_TEST_TIERS=100,500,1000` et produit un rapport JSON par palier avec
connexions, livraisons, latences, snapshot `hello` et verification `heartbeat` des sources.

### Badges sources

Les badges publics lisent seulement le resume live:

```json
{
  "source": "financial_juice_browser",
  "enabled": true,
  "status": "active",
  "updated_at": "2026-05-09T09:12:00Z"
}
```

Regles:

- `listener`: voit uniquement les badges publics verts/rouges via WebSocket live;
- `moderator`: voit les badges via WebSocket live et peut acceder aux pages admin autorisees;
- `admin`: voit les badges via WebSocket live et charge `/api/sources` pour les details et switches;
- `/api/sources`: route backend directe reservee admin, avec etat complet et actions.

### Data plane audio HLS

Canal nominal:

```text
Client HLS -> Cloudflare CDN / Worker HLS -> Rust origin HLS
```

Objectifs:

- 1000 auditeurs HLS sans saturation Rust origin;
- les segments chauds sont majoritairement servis par Cloudflare;
- les playlists restent fraiches sans request storm;
- aucun player HLS actif quand la queue audio est vide;
- lecture clip sur `audio_ready`, sans polling de decouverte.

Mesures:

- requetes `GET /api/hls-ticket`;
- requetes playlist;
- requetes segments;
- latence p50/p95/p99;
- erreurs HTTP;
- `CF-Cache-Status`;
- requetes arrivees a Traefik/Rust origin;
- CPU/RAM backend;
- interruptions audio.

Implementation actuelle: `scripts/load_backend_ws_hls.mjs` couvre les paliers HLS/CDN
`50,100,200,500`, mesure les tickets HLS, playlists, segments, bytes, latences et
`CF-Cache-Status`, avec support du replay du jour et d'un clip optionnel via
`FILFINANCE_LOAD_TEST_HLS_CLIP_PLAYLIST_URL`.

## Tickets de dev

- `NEWS-156`: systeme securite App frontend vers backend.
- `NEWS-157`: source status public via heartbeat/snapshot et `/api/sources` admin-only.
- `NEWS-158`: systeme securite Worker Edge.
- `NEWS-159`: systeme securite HLS avec tickets playback et CDN.
- `NEWS-160`: health minimal public et health admin detaille.
- `NEWS-161`: plan de charge Cloudflare Durable Objects metadata.
- `NEWS-162`: plan de charge HLS + CDN.

## Definition of Done

- les quatre systemes de securite sont documentes et implementes;
- aucune route metier frontend vers backend ne fonctionne sans JWT valide en production;
- `/api/sources` detaille est admin-only;
- les badges sources publics ne dependent plus d'un appel direct a `/api/sources`;
- le Worker Edge est le chemin nominal metadata pour les clients finaux;
- le HLS est protege par ticket sans casser le cache CDN des segments;
- `/health` reste minimal;
- `/api/admin/health` donne les diagnostics detailles aux admins;
- les resultats de charge metadata et HLS sont archives.

## References

- [EPIC-014 - Authentification Supabase](./EPIC-014-authentification-supabase.md)
- [EPIC-018 - Metadata live edge Durable Objects](./EPIC-018-metadata-live-edge-durable-objects.md)
- [EPIC-021 - Audio live evenementiel par clips HLS progressifs](./EPIC-021-audio-live-evenementiel-clips-hls-progressifs.md)
- [EPIC-023 - Admin runtime operations](./EPIC-023-admin-runtime-operations.md)
- [EPIC-027 - Cache court des playlists HLS Worker](./EPIC-027-cache-playlists-hls-worker.md)
