# NEWS-199 - Kill-switch HLS Worker Cloudflare

## Objectif

Fermer par defaut le proxy HLS du Worker Edge pour que `live.filfinance.fr` ne livre plus
de playlists ou segments audio HLS via le CDN Cloudflare standard.

## Scope

- Ajouter `FILFINANCE_HLS_PROXY_ENABLED`.
- Retourner `410 Gone` sur `/hls/audio/...` quand la variable ne vaut pas exactement
  `"true"`.
- Configurer `false` dans `wrangler.jsonc` pour production et dev.
- Garder `/health`, `/ws/live` et `/publish/live` inchanges.
- Ajouter les tests unitaires Worker.

## Criteres d'acceptation

- Variable absente: HLS proxy ferme.
- `FILFINANCE_HLS_PROXY_ENABLED=false`: HLS proxy ferme.
- `FILFINANCE_HLS_PROXY_ENABLED=true`: comportement proxy existant conserve.
- `npm run check` passe dans `filfinance-edge`.

## Rollback controle

Ne reactiver `FILFINANCE_HLS_PROXY_ENABLED=true` qu'apres validation ecrite d'une cible
media conforme. La reactivation seule ne suffit pas: les scripts de charge restent gardes
par `FILFINANCE_ALLOW_CLOUDFLARE_MEDIA_LOAD_TEST`.

## Dependances

- `EPIC-034`
