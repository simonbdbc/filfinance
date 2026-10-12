# EPIC-035 — Pilote Bunny DEV complet

## Statut

Implémenté dans le dépôt; le provisioning réel reste bloqué par les accès Bunny, DNS et pare-feu
opérateur. Production Cloudflare est hors périmètre.

## Cible DEV

- DNS délégué: `dev.filfinance.fr`.
- Landing et PWA: une Storage Zone Bunny et une Pull Zone chacune; la PWA retourne
  `/index.html` avec HTTP 200 pour les routes profondes.
- API: `api.dev.filfinance.fr`, cache désactivé, origine privée
  `https://origin.dev.filfinance.fr`.
- HLS: `media.dev.filfinance.fr`, query strings dans la clé, cache gouverné par les headers Rust,
  CORS HLS et POST interdit.
- WebSocket: `live.dev.filfinance.fr`, WebSockets activés et cache désactivé.
- Bunny Shield Basic protège API/media/live avec des limites dédiées à l'authentification et au
  handshake WebSocket. Budget DEV: 5 USD/mois.

Le backend Rust vérifie le ticket court HMAC avant `GET /ws/live?ticket=…`. Redis, privé dans
Docker avec AOF et healthcheck, porte le snapshot et le Pub/Sub entre replicas; Bunny ne porte
aucun état métier. `NEWS_EDGE_PUBLISH_ENABLED=false` en DEV. Les producteurs de traduction,
audio, calendrier et statuts passent par l'entrée de publication commune, ce qui garde la
compatibilité du Worker/Durable Object Cloudflare de production.

## Livraison et validation

La CI DEV téléverse les assets Bunny avant `index.html`, supprime les objets obsolètes depuis le
manifeste distant, puis purge la Pull Zone. Le préflight vérifie les ressources/hostnames Bunny et
les variables GitLab protégées. Voir [le runbook Bunny DEV](../BUNNY-DEV-RUNBOOK.md) pour le
provisioning, les certificats, Shield, firewall, smokes, paliers HLS 50→1 000, test fan-out et
rollback DNS.
