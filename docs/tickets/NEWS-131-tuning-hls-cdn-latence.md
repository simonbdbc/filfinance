# NEWS-131 - Tuning HLS/CDN latence live

## Objectif

Reduire la latence audio live sans augmenter inutilement la charge origin.

Le premier objectif est de comparer le segment HLS actuel `4s` avec un profil `2s`, puis de verifier que Cloudflare CDN absorbe bien les segments.

## Priorite

Priorite 6.

Ce ticket vient apres la remise en place OPUS-MT, l'observabilite et les gains qualite TTS.

Note du 4 mai 2026: le tuning du live HLS permanent est supersede par `EPIC-021`. Les mesures utiles doivent etre reprises sur les clips HLS finalises par news, en particulier via `NEWS-142`.

## Scope

- Tester `NEWS_HLS_LIVE_SEGMENT_TARGET_DURATION_SECS=2`.
- Comparer avec le profil actuel `4s`.
- Garder une playlist live courte.
- Conserver les segments `.ts` en cache long immutable.
- Garder les playlists `.m3u8` en cache tres court.
- Mesurer les requetes origin et le cache hit ratio.
- Mesurer la latence audio audible cote client.

## Configuration de test

```env
NEWS_HLS_LIVE_SEGMENT_TARGET_DURATION_SECS=2
NEWS_HLS_LIVE_WINDOW_SEGMENTS=6
```

## Critere d'acceptation

- Le player HLS demarre correctement en live et replay du jour.
- Le passage a `2s` reduit la latence percue.
- La charge origin reste acceptable.
- Les segments restent cacheables par Cloudflare.
- Le replay du jour reste coherent.

## Hors scope

- LL-HLS complet.
- R2 obligatoire.
- Durable Objects pour audio.
