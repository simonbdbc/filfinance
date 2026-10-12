# NEWS-166 - Runbook cache playlists HLS Worker

## Objectif

Documenter l'exploitation du cache tres court des playlists HLS: activation, verification,
rollback et monitoring.

## Scope

- Variables Worker a configurer.
- Commandes de deploiement.
- Smoke manuel playlist.
- Interpretation des headers.
- Rollback simple.
- Points de surveillance apres activation.

## Activation cible

Dans `filfinance-edge/wrangler.jsonc` ou via variable d'environnement Worker:

```json
{
  "vars": {
    "FILFINANCE_HLS_PLAYLIST_CACHE_TTL_SECONDS": "2"
  }
}
```

Puis:

```bash
cd /home/traefik/filfinance-docker/filfinance-edge
npx wrangler deploy
```

## Smoke manuel

Runbook historique: depuis `EPIC-034`, ne pas executer ce smoke sur Cloudflare CDN
standard. Le Worker HLS doit rester ferme sauf cible media conforme validee.

1. Recuperer un `hls_ticket` via `/api/hls-ticket`.
2. Appeler deux fois la meme playlist:

```bash
curl -I "https://media-dns-only.example.test/hls/audio/replay/today/playlist.m3u8?hls_ticket=$HLS_TICKET"
curl -I "https://media-dns-only.example.test/hls/audio/replay/today/playlist.m3u8?hls_ticket=$HLS_TICKET"
```

Attendu:

```text
X-FilFinance-HLS-Proxy-Cache: PLAYLIST_MISS
X-FilFinance-HLS-Proxy-Cache: PLAYLIST_HIT
```

Selon les limitations `HEAD`, le smoke de reference peut utiliser `curl -sS -D - ... -o /dev/null`
si le Worker ne cache que les `GET`.

## Rollback

Mettre:

```env
FILFINANCE_HLS_PLAYLIST_CACHE_TTL_SECONDS=0
```

Puis redeployer le Worker.

Le comportement doit revenir a:

```text
X-FilFinance-HLS-Proxy-Cache: BYPASS_PLAYLIST_DISABLED
```

## Monitoring

Surveiller apres activation:

- erreurs playlist `401/403/5xx`;
- ratio `PLAYLIST_HIT` / `PLAYLIST_MISS`;
- latence p95/p99 playlist;
- erreurs segment;
- ratio segment `HIT` / `MISS`;
- CPU/RAM backend;
- logs Traefik sur `/audio/replay/today/playlist.m3u8`.

## Criteres d'acceptation

- Activation et rollback documentes.
- Smoke manuel reproductible.
- Le load test 500 passe avec TTL actif.
- Les resultats 500 et 1000 sont archives dans le compte rendu projet.

## Dependances

- `EPIC-027`
- `NEWS-164`
- `NEWS-165`
