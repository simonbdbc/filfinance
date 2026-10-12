# EPIC-034 - Retrait HLS du CDN Cloudflare standard

## Objectif

Retirer immediatement le data plane audio HLS du CDN Cloudflare standard apres la
mitigation Trust & Safety du 18 juin 2026.

Le Worker Edge reste le canal metadata WebSocket et publication serveur. En revanche,
`/hls/audio/...` ne doit plus livrer playlists ou segments HLS tant qu'une offre media
Cloudflare conforme, ou une autre cible de diffusion, n'a pas ete validee.

## Statut

En attente d'implementation.

## Constat

La campagne de charge HLS a fait transiter des playlists `.m3u8` et segments `.ts`
audio-only via le Worker Cloudflare et le domaine `live.filfinance.fr`. Cloudflare a
applique deux mitigations actives sur la zone:

- `Cache rate limit`;
- `Video stream redirect`.

Le comportement technique reste assimilable a une diffusion media HLS a fort volume:

- segments MPEG-TS servis avec `Content-Type: video/MP2T`;
- cache Worker des segments;
- cache court des playlists;
- tests de charge visant 500 puis 1000 auditeurs.

Ce modele ne doit plus etre le chemin nominal tant qu'il repose sur le CDN Cloudflare
standard.

## Decisions

- `live.filfinance.fr` reste reserve au control plane Edge: `/health`, `/ws/live` et
  `/publish/live`.
- Le proxy HLS Worker `/hls/audio/...` est ferme par defaut par
  `FILFINANCE_HLS_PROXY_ENABLED=false`.
- Toute requete HLS Worker fermee repond `410 Gone` pour rendre l'ancien chemin
  explicitement obsolete.
- Les variables Pages production doivent garder `EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL`
  vide; le client retombe alors sur les URLs HLS directes backend derivees de l'API.
- Les tests de charge HLS refusent toute cible Cloudflare ou `filfinance.fr` sans opt-in
  explicite `FILFINANCE_ALLOW_CLOUDFLARE_MEDIA_LOAD_TEST=true`.
- L'opt-in de charge Cloudflare ne doit etre utilise qu'apres validation d'une offre media
  conforme.

## Options media hors scope de cet EPIC

Le choix long terme est volontairement separe de la coupure immediate:

- Cloudflare Stream Delivery si Fil Finance conserve son HLS origin Rust;
- Cloudflare Stream si l'audio est encapsule dans une video minimale compatible Stream;
- R2 ou un autre stockage/CDN media si l'usage est confirme conforme;
- sous-domaine DNS-only ou CDN non Cloudflare pour un contournement temporaire.

## Tickets de dev

- `NEWS-199`: kill-switch du proxy HLS Worker.
- `NEWS-200`: nettoyage des variables et docs client HLS.
- `NEWS-201`: garde anti-charge Cloudflare dans les scripts HLS.
- `NEWS-202`: validation, runbook Trust & Safety et verification post-coupure.

## Definition of Done

- `FILFINANCE_HLS_PROXY_ENABLED=false` est configure en prod et dev Worker.
- `/hls/audio/...` retourne `410 Gone` quand le switch est absent ou different de
  `"true"`.
- Le comportement HLS existant reste testable uniquement quand le switch vaut `"true"`.
- Les exemples production ne configurent plus la base HLS publique vers
  `live.filfinance.fr`.
- Les scripts de charge bloquent `filfinance.fr`, `workers.dev`,
  `cloudflarestream.com` et `videodelivery.net` sans opt-in explicite.
- Le runbook production indique clairement que `live.filfinance.fr` ne sert plus le data
  plane audio HLS.

## References

- [EPIC-026 - Securite et charge production](./EPIC-026-securite-et-charge-production.md)
- [EPIC-027 - Cache court des playlists HLS Worker](./EPIC-027-cache-playlists-hls-worker.md)
- [NEWS-162 - Plan charge HLS CDN](../tickets/NEWS-162-plan-charge-hls-cdn.md)
- [NEWS-167 - Validation charge combinee Edge metadata et HLS](../tickets/NEWS-167-validation-charge-combinee-edge-hls.md)
