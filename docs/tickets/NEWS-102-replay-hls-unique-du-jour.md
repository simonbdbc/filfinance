# NEWS-102 - Replay HLS unique du jour

## Objectif

Produire un replay HLS unique qui couvre uniquement la journee courante, de minuit a maintenant, sans clips par news et sans historique multi-jours.

## Implementation

Le replay reutilise les segments live produits par `NEWS-100`.

Endpoints publics HLS:

```http
GET /audio/replay/today/playlist.m3u8
GET /audio/replay/today/segments/{local_day}/{sequence}.ts
```

Endpoint metadata protege listener:

```http
GET /api/replay/today
```

Exemple de metadata:

```json
{
  "enabled": true,
  "available": true,
  "local_day": "2026-05-03",
  "playlist_url": "/audio/replay/today/playlist.m3u8",
  "segment_count": 12,
  "first_sequence": 1,
  "latest_sequence": 12,
  "updated_at": "2026-05-03T10:00:00Z"
}
```

## Comportement

- La playlist replay est une playlist HLS `EVENT`.
- Elle expose tous les segments connus du jour courant.
- Elle ne contient pas `#EXT-X-ENDLIST`, car la journee continue.
- Une playlist vide reste valide si aucun segment TTS n'a encore ete produit.
- Les segments replay sont les memes fichiers immuables que les segments live.
- Le replay refuse les URLs dont `local_day` n'est pas le jour courant.
- A minuit local, les segments de la veille sortent de l'index et leur dossier est nettoye en best-effort.
- Au redemarrage, l'origin recharge les segments `.ts` deja presents dans le dossier du jour courant.

## Reset a minuit

Le reset est base sur le jour local du process.

En production, le serveur doit rester configure sur la timezone `Europe/Paris` pour respecter le contrat produit:

```text
replay du jour = minuit Europe/Paris -> maintenant
```

## Cache

Le replay reutilise les headers de `NEWS-101`:

Playlist replay:

```http
Content-Type: application/vnd.apple.mpegurl
Cache-Control: public, max-age=1, must-revalidate
Cloudflare-CDN-Cache-Control: max-age=1
X-Content-Type-Options: nosniff
```

Segments replay:

```http
Content-Type: video/MP2T
Cache-Control: public, max-age=86400, immutable
Cloudflare-CDN-Cache-Control: max-age=86400
X-Content-Type-Options: nosniff
```

## Validation

Validation automatique:

```bash
cd filfinance-service
cargo test
```

Resultat local:

```text
96 passed
```

Validation live apres activation HLS:

```bash
# Ajouter `-H "Authorization: Bearer $SUPABASE_ACCESS_TOKEN"` si l'auth backend est activee.
curl -fsS http://127.0.0.1:8088/api/replay/today
curl -fsS http://127.0.0.1:8088/audio/replay/today/playlist.m3u8
curl -I http://127.0.0.1:8088/audio/replay/today/segments/$(date +%F)/1.ts
```

Verifier:

- la playlist contient `#EXT-X-PLAYLIST-TYPE:EVENT`;
- les URLs de segments sont sous `/audio/replay/today/segments/{local_day}/`;
- aucun endpoint clip par news n'existe;
- le replay ne sert pas un `local_day` ancien;
- apres redemarrage, les segments du jour encore presents sur disque sont recharges.

## Limites

- La duree exacte des segments recharges apres redemarrage est reconstruite avec la duree cible HLS.
- Le load test replay sous charge reste couvert par `NEWS-104`.
- Le client Web utilise le replay HLS du jour depuis `NEWS-103`.
