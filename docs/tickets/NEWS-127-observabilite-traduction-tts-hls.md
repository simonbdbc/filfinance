# NEWS-127 - Observabilite traduction, TTS et HLS

## Objectif

Mesurer le chemin live complet avant d'optimiser: traduction, normalisation, TTS Piper, encodage HLS, disponibilite playlist et cache CDN.

Sans cette observabilite, les optimisations risquent de viser le mauvais goulot.

## Priorite

Priorite 2, juste apres `NEWS-126`.

## Scope

- Exposer ou logger les latences p50/p95 de traduction.
- Exposer le provider et le profil de traduction actifs.
- Mesurer la queue de traduction.
- Mesurer la queue TTS.
- Mesurer la duree de synthese Piper.
- Mesurer la taille WAV produite.
- Mesurer la duree d'encodage HLS par news.
- Mesurer le delai entre `tts_received` et playlist HLS mise a jour.
- Compter les erreurs `ffmpeg`.
- Ajouter les infos utiles aux traces `/ws/test-runs`.

## Mesures CDN/HLS a suivre

- Requetes origin par seconde sur playlists.
- Requetes origin par seconde sur segments `.ts`.
- Cache hit ratio CDN des segments.
- Latence playlist visible.
- Latence audio audible cote client.
- Retard moyen du live.

## Livraison 2026-05-03

- `/api/runtime/translation` expose maintenant `pending_total`, le nombre de samples et les latences `translation_duration_p50_ms` / `translation_duration_p95_ms`.
- `/api/runtime/tts` expose maintenant `pending_total`, la duree de synthese recente et les latences `generation_duration_p50_ms` / `generation_duration_p95_ms`.
- `/api/runtime/hls` expose l'etat origin HLS, la commande `ffmpeg`, les compteurs `published_total`, `failed_total`, `ffmpeg_failed_total`, les dernieres durees de publication/encodage et les p50/p95.
- Les logs de publication HLS portent `publish_duration_ms`, `render_duration_ms` et `segment_count`.
- Les traces `/ws/test-runs` incluent `synthesis_duration_ms` sur les etapes TTS et une etape `hls_published` avec `publish_duration_ms`, `render_duration_ms`, `segment_count` et `tts_to_playlist_ms`.

## Critere d'acceptation

- Un run de test donne une trace temporelle exploitable de bout en bout.
- Les logs indiquent clairement si le goulot est OPUS-MT, Piper, `ffmpeg`, origin HLS, CDN ou client.
- Les mesures permettent de comparer objectivement `beam-size`, workers et duree de segment HLS.

## Hors scope

- Ajouter les pools de workers.
- Changer la duree des segments HLS.
- Ajouter un dashboard externe.
