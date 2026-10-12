# NEWS-176 - Configuration production `filfinance.fr`

> **Archive historique — ne pas executer.** Cette procedure decrit encore un
> frontend Expo unique et Supabase. La cible actuelle separe la landing
> `filfinance.fr` de la PWA `app.filfinance.fr` et utilise Better Auth. Appliquer
> [le runbook GitLab](../GITLAB-RUNBOOK.md),
> [le runbook Better Auth](../AUTH-RUNBOOK.md) et
> [le runbook de production](../PRODUCTION-FINAL-RUNBOOK.md).

## Objectif

Preparer la configuration production finale:

- zone DNS: `filfinance.fr` reste achete chez OVH mais devient gere dans Cloudflare;
- frontend: `https://filfinance.fr`;
- API: `https://api.filfinance.fr`;
- edge live metadata: `https://live.filfinance.fr`;
- media HLS: `https://media.filfinance.fr`;
- branche source: `main`;
- developpement: local uniquement jusqu'a `EPIC-029`.

## Statut

Demarre cote repo le 21 mai 2026.

La configuration dashboard Cloudflare, DNS, Supabase et serveur reste a appliquer
dans les environnements cibles avant d'activer le deploy production complet.

## Configuration Cloudflare Pages production

Pre-requis zone DNS Cloudflare:

1. ajouter `filfinance.fr` comme site/zone dans Cloudflare;
2. importer ou recreer les records DNS utiles depuis OVH;
3. remplacer les nameservers OVH par les deux nameservers assignes par Cloudflare;
4. attendre que la zone Cloudflare passe en statut actif;
5. verifier que les records web sont proxifies Cloudflare quand ils portent du trafic HTTP/HTTPS;
6. laisser les records non HTTP, par exemple MX/TXT, en DNS-only.

Le but est que `filfinance.fr`, `api.filfinance.fr` et `live.filfinance.fr`
soient servis via Cloudflare, pas directement via la zone DNS OVH.

Creer ou utiliser un projet Pages production dedie, recommande:

```text
filfinance-prod
```

Le projet `fil-finance` est reserve au futur environnement dev de `EPIC-029`.

Configuration attendue:

- mode: Direct Upload par GitLab CI;
- custom domain: `filfinance.fr`;
- production branch logique: `main`;
- build effectue dans GitLab, pas dans Cloudflare Git integration.

Variables publiques a fournir au build GitLab:

```text
EXPO_PUBLIC_FILFINANCE_WS_URL=wss://api.filfinance.fr/ws/news
EXPO_PUBLIC_FILFINANCE_EDGE_WS_URL=wss://live.filfinance.fr/ws/live
EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL=https://media.filfinance.fr
EXPO_PUBLIC_SUPABASE_URL=https://project-ref.supabase.co
EXPO_PUBLIC_SUPABASE_PUBLISHABLE_KEY=sb_publishable_xxx
```

`EXPO_PUBLIC_SUPABASE_ANON_KEY` reste accepte par compatibilite, mais la convention
du projet est `EXPO_PUBLIC_SUPABASE_PUBLISHABLE_KEY`.

`live.filfinance.fr` est control plane Edge uniquement depuis `EPIC-034`; il ne doit plus
etre configure comme base HLS. La base HLS production est Bunny
`https://media.filfinance.fr`.

## Configuration API production

Cible:

```text
api.filfinance.fr
```

Actions attendues:

- creer l'enregistrement DNS dans Cloudflare vers le serveur personnel;
- activer le proxy Cloudflare sur cet enregistrement si le trafic API doit passer par Cloudflare;
- ajouter ou adapter la route Traefik vers `filfinance-service@docker`;
- verifier le certificat TLS;
- verifier `GET https://api.filfinance.fr/health`;
- mettre a jour `.env.server` avec `NEWS_SERVER_ENV=production` et `NEWS_AUTH_ENABLED=true`.

## Configuration media HLS production

Cible:

```text
media.filfinance.fr
```

Actions attendues:

- creer la Pull Zone Bunny `filfinance-prod-hls`;
- brancher l'origin `https://api.filfinance.fr`;
- ajouter `media.filfinance.fr` comme custom hostname Bunny;
- creer le CNAME Cloudflare en DNS-only vers le hostname Bunny;
- appliquer la regle Cloudflare origin limitee a
  `(http.host eq "api.filfinance.fr" and starts_with(http.request.uri.path, "/audio/"))`;
- verifier le certificat Bunny;
- verifier `401` sans ticket et `200` avec ticket sur `/audio/replay/today/playlist.m3u8`.

## Configuration edge production

Cible:

```text
live.filfinance.fr
```

Actions attendues:

- deployer le Worker `filfinance-live-edge`;
- attacher le domaine `live.filfinance.fr`;
- verifier que le domaine Worker est gere dans la zone Cloudflare `filfinance.fr`;
- verifier `GET https://live.filfinance.fr/health`;
- definir les secrets Worker:

```bash
npx wrangler secret put FILFINANCE_EDGE_PUBLISH_SECRET
npx wrangler secret put FILFINANCE_EDGE_TICKET_SECRET
```

Ces secrets doivent correspondre a `.env.server`:

```text
NEWS_EDGE_PUBLISH_SECRET
NEWS_EDGE_TICKET_SECRET
```

## Supabase

URLs autorisees minimales:

- `https://filfinance.fr`;
- `https://filfinance.fr/**` si necessaire pour les redirects;
- `http://localhost:8052` pour le developpement local.

`https://fil-finance.pages.dev` sera ajoute ou conserve uniquement quand `EPIC-029`
ouvrira l'environnement dev public.

## Criteres d'acceptation

- `filfinance.fr` est attache au projet Pages production dedie.
- la zone `filfinance.fr` est active dans Cloudflare apres changement des nameservers OVH.
- les records HTTP/HTTPS utiles sont proxifies par Cloudflare.
- `api.filfinance.fr` repond sur `/health`.
- `live.filfinance.fr` repond sur `/health`.
- `media.filfinance.fr` sert le HLS Bunny avec ticket valide.
- Supabase Auth accepte `filfinance.fr`.
- `fil-finance.pages.dev` n'est pas utilise comme fallback produit de production.

## Dependances

- [EPIC-028](../epics/EPIC-028-migration-production-filfinance-fr.md)
- [PRODUCTION-FINAL-RUNBOOK](../PRODUCTION-FINAL-RUNBOOK.md)
