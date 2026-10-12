# EPIC-019 - Audio HLS live et replay du jour

## Objectif

Remplacer le streaming audio binaire historique par un premier modele broadcast audio scalable:

- live: Rust origin HLS + Cloudflare CDN
- replay: un seul replay HLS du jour, de minuit a maintenant
- reset replay a minuit
- segments cacheables
- aucun historique de replay
- aucun clip replay par news

## Valeur livree

L'audio devient un data plane diffusable par CDN, au lieu d'un fan-out WebSocket porte par le backend Rust.

Le backend a d'abord produit une seule narration TTS globale, consommee comme une radio Fil Finance.

Ce modele "radio HLS permanente" est maintenant considere comme un palier transitoire abandonne. Il a permis de livrer l'origin HLS, les headers CDN, le replay du jour et les premiers smokes runtime, mais il n'est plus expose par le runtime cible.

## Statut

Cadrage ajoute le 2 mai 2026.

`NEWS-099` livre le contrat audio broadcast HLS le 3 mai 2026.

`NEWS-100` livre le premier origin HLS live Rust le 3 mai 2026.

`NEWS-101` durcit les headers HTTP/CDN HLS le 3 mai 2026.

`NEWS-102` livre le replay HLS unique du jour le 3 mai 2026.

`NEWS-103` branche le client web sur le player HLS live/replay du jour le 3 mai 2026.

`NEWS-125` package le runtime Docker Piper necessaire a la validation live TTS/HLS le 3 mai 2026.

`NEWS-127` et `NEWS-131` cadrent l'observabilite puis le tuning HLS/CDN.

Decision du 4 mai 2026: le live HLS global permanent est supersede par [EPIC-021 - Audio live evenementiel par clips HLS progressifs](./EPIC-021-audio-live-evenementiel-clips-hls-progressifs.md).

`NEWS-136` reste une mitigation court terme contre le request storm HLS a vide, mais la correction produit durable consiste a ne plus maintenir de player HLS live pendant l'idle.

`NEWS-141` supprime ensuite les routes runtime `/audio/live/*`, le metadata endpoint `/api/audio/live`, les flags legacy et les compteurs associes. Le replay du jour reste separe.

## Architecture cible

Architecture initiale de ce palier:

```text
Backend Rust
  -> TTS unique
  -> Rust origin HLS
  -> Cloudflare CDN
  -> clients Pages
```

Replay du jour:

```text
Backend Rust
  -> segments HLS du jour
  -> /audio/replay/today/playlist.m3u8
  -> reset a minuit
```

Architecture cible apres supersession par `EPIC-021`:

```text
Backend Rust
  -> clip HLS finalise par news
  -> audio_ready via control plane
  -> Cloudflare CDN pour les segments
  -> clients Pages sans player HLS en idle
```

## Decisions

- Le live audio est global, non personnalise par utilisateur.
- La preference de voix TTS ne pilote plus le live global.
- Le replay n'est pas decoupe par news.
- Le replay unique du jour remplace les clips replay unitaires.
- Le replay ne garde pas d'historique apres reset.
- Le live demarre en HLS classique avec segments 4-6 secondes.
- LL-HLS reste une optimisation future si la latence produit l'exige.
- Le WAV produit par Piper reste un format interne; le format HLS public cible est AAC-LC en segments MPEG-TS `.ts`.
- Le replay du jour est cale sur minuit `Europe/Paris`.
- L'URL replay conservee est `/audio/replay/today/playlist.m3u8`; la surface `/audio/live/*` est retiree par `NEWS-141`.
- R2 n'est pas necessaire pour le live au depart.
- R2 peut rester une option future pour le replay du jour si le backend ne doit plus servir les segments.
- Le live nominal utilise `/audio/clips/{stream_id}/playlist.m3u8` publie par evenement `audio_ready`.
- Le replay du jour reste a clarifier separement: il peut rester utile, mais il ne doit pas imposer un player live permanent.

## Scope

Dans cet EPIC:

Note du 4 mai 2026: EPIC-019 est supersede par EPIC-021 pour le modele live cible. Le WebSocket audio binaire `/ws/audio` a ete retire, et le live permanent HLS global n'est plus le chemin nominal.

- definir le contrat audio live HLS
- produire les manifests et segments live depuis Rust
- servir les segments avec headers CDN corrects
- produire un replay HLS unique de la journee courante
- reset du replay a minuit
- adapter le client a un player HLS
- retirer le chemin nominal `/ws/audio`
- tester 1000 auditeurs audio

Hors scope:

- clips replay par news
- historique de replay multi-jours
- personnalisation live par voix utilisateur
- stockage R2 obligatoire pour le live
- fan-out audio via Durable Objects

Note post-cadrage: les clips HLS par news deviennent finalement le modele nominal du live dans `EPIC-021`; le hors scope ci-dessus decrit seulement le cadrage initial de `EPIC-019`.

## Tickets de dev

### [`NEWS-099` Contrat audio broadcast HLS](../tickets/NEWS-099-contrat-audio-broadcast-hls.md)

Objectif: figer le contrat live et replay avant implementation.

Statut: fait.

Scope:

- URL live playlist
- URL replay today playlist
- formats audio acceptes
- duree cible des segments
- timezone de reset
- etats client possibles
- headers HTTP/CDN des playlists et segments
- retrait du WebSocket audio binaire historique

Criteres d'acceptation:

- le contrat distingue clairement live et replay
- le replay est unique pour la journee courante
- aucun endpoint clip par news n'est cree
- le format public HLS est separe du WAV interne TTS
- les metadata edge `audio_state` et `now_playing` sont identifiees

### [`NEWS-100` Rust origin HLS live](../tickets/NEWS-100-origin-rust-hls-live.md)

Objectif: produire un premier flux HLS live depuis le backend.

Statut: code livre, validation live Piper/HLS a faire.

Scope:

- orchestration TTS unique
- generation segments audio via `ffmpeg`
- manifest HLS live
- fenetre glissante
- nettoyage best-effort des anciens dossiers jour
- logs de production audio
- variables `NEWS_HLS_*`
- routes `/audio/live/*`

Criteres d'acceptation:

- le flux live est lisible par un player HLS web
- le pipeline news/audio reste non bloquant pour l'ingestion
- une panne HLS ne casse pas la persistence news
- le WebSocket audio binaire historique n'est plus expose pendant la cible EPIC-021

### [`NEWS-101` Cache CDN des segments HLS](../tickets/NEWS-101-cache-cdn-segments-hls.md)

Objectif: configurer les headers pour que Cloudflare CDN absorbe la charge clients.

Statut: code livre, validation live Cloudflare a faire.

Scope:

- cache long pour segments immuables
- cache court ou no-store pour manifests live
- headers `Content-Type` corrects
- strategie de purge implicite via noms de segments versionnes
- header `Cloudflare-CDN-Cache-Control`
- tests de contrat des headers HLS

Criteres d'acceptation:

- les segments peuvent etre caches sans risque de contenu stale
- les manifests live restent frais
- le backend Rust ne sert pas chaque requete client quand le CDN est chaud
- les headers origin sont documentes pour les futures Cache Rules Cloudflare

### [`NEWS-102` Replay HLS unique du jour](../tickets/NEWS-102-replay-hls-unique-du-jour.md)

Objectif: produire un replay HLS qui couvre uniquement la journee courante.

Statut: code livre, validation live Piper/HLS a faire.

Scope:

- playlist `/audio/replay/today/playlist.m3u8`
- segments du jour
- endpoint metadata `GET /api/replay/today`
- reset a minuit
- purge de la veille
- recharge des segments du jour apres redemarrage
- refus des segments replay hors jour courant

Criteres d'acceptation:

- le replay couvre de minuit a maintenant
- aucun historique de replay n'est conserve
- les segments sont cacheables
- le replay reste lisible pendant que la journee continue
- aucun endpoint clip par news n'est cree

### [`NEWS-103` Client player HLS](../tickets/NEWS-103-client-player-hls.md)

Objectif: remplacer la lecture `/ws/audio` par un player HLS live/replay.

Statut: code livre, validation live navigateur a faire.

Scope:

- player live HLS
- bouton replay du jour
- synchronisation `now_playing` via Durable Objects
- etats `loading`, `playing`, `delayed`, `error`
- fallback navigateur si HLS natif ou librairie non disponible
- absence de fallback WebSocket audio binaire

Criteres d'acceptation:

- le client n'ouvre pas de WebSocket audio binaire
- le live audio reste synchronise avec les cards via metadata
- le replay du jour ne declenche pas d'historique multi-jours

### `NEWS-104` Load test audio 1000 auditeurs

Objectif: valider la distribution audio avant promotion production.

Scope:

- simulation clients HLS
- mesure requetes origin
- mesure cache hit ratio CDN
- mesure latence live
- mesure interruptions audio

Criteres d'acceptation:

- 1000 auditeurs peuvent suivre le live sans saturer Rust origin
- le CDN porte la majorite des requetes segments
- les resultats guident le choix HLS classique ou LL-HLS

### [`NEWS-136` Fix request storm HLS live a vide](../tickets/NEWS-136-fix-hls-live-idle-request-storm.md)

Objectif: mitiguer le request storm observe quand le live HLS permanent est active sans segment frais.

Statut: mitigation transitoire.

Scope:

- ne pas instancier `hls.js` quand aucun segment live n'est disponible
- borner les retries metadata
- retourner `204 No Content` sur playlist live vide ou stale
- documenter l'idle HLS comme probleme produit, pas seulement bug technique

Criteres d'acceptation:

- activer le live sans segment ne provoque pas de request storm
- le client indique un etat idle
- le chemin nominal futur doit etre traite par `EPIC-021`

### [`EPIC-021` Audio live evenementiel par clips HLS progressifs](./EPIC-021-audio-live-evenementiel-clips-hls-progressifs.md)

Objectif: remplacer le modele live permanent par `pub/sub` leger + clips HLS finalises par news.

Statut: cible officielle du live audio depuis le 4 mai 2026.

Scope:

- contrat `audio_ready`
- origin HLS clip progressif intro + corps
- publication backend vers control plane
- player frontend global pilote par queue
- migration hors `/audio/live/playlist.m3u8` nominal
- smokes zero requete HLS en idle

### [`NEWS-127` Observabilite traduction, TTS et HLS](../tickets/NEWS-127-observabilite-traduction-tts-hls.md)

Objectif: mesurer le chemin live complet avant tuning.

Statut: a faire.

Scope:

- latence traduction
- queue TTS
- duree synthese Piper
- duree encodage HLS
- latence playlist visible
- erreurs `ffmpeg`
- mesures CDN/HLS

Criteres d'acceptation:

- un run E2E permet d'identifier le goulot reel
- les mesures guident les profils, pools et segments HLS

### [`NEWS-125` Runtime Docker Piper pour TTS/HLS](../tickets/NEWS-125-runtime-docker-piper-hls.md)

Objectif: rendre l'image Docker backend capable de lancer le provider `piper_local_session`.

Statut: code livre, validation live serveur a faire.

Scope:

- installer Python et le venv Piper dans l'image
- copier le helper `piper_local_session.py`
- pointer `.env.server.example` vers le chemin conteneur
- permettre le bootstrap des assets voix dans le volume `/app/data/models/piper`
- garder le smoke runtime comme garde-fou de deploy

Criteres d'acceptation:

- le service ne produit plus `failed to spawn piper helper`
- `scripts/smoke_backend_runtime.sh` valide Piper quand `NEWS_TTS_ENABLED=true`
- une news de test produit un segment HLS apres activation TTS/HLS

### [`NEWS-131` Tuning HLS/CDN latence live](../tickets/NEWS-131-tuning-hls-cdn-latence.md)

Objectif: comparer le segment actuel `4s` avec un profil `2s` sans saturer l'origin.

Statut: a faire.

Scope:

- `NEWS_HLS_CLIP_SEGMENT_TARGET_DURATION_SECS=2`
- playlist live courte
- cache `.m3u8` tres court
- cache `.ts` immutable
- mesure origin/CDN/client

Criteres d'acceptation:

- latence percue reduite
- charge origin acceptable
- replay du jour coherent

## Definition of Done du palier initial

- le live audio passe par Rust origin HLS + Cloudflare CDN
- le WebSocket audio binaire n'est plus expose
- le replay est un unique HLS du jour, reset a minuit
- aucun clip replay par news n'est implemente
- aucun historique replay n'est conserve
- le load test 1000 auditeurs est documente

Note de supersession: cette Definition of Done reste la trace du cadrage initial `EPIC-019`. Le live audio nominal a partir du 4 mai 2026 est redefini par `EPIC-021`, avec clips HLS finalises par news et absence de player HLS en idle.

## References

- [EPIC-018 - Metadata live edge Durable Objects](./EPIC-018-metadata-live-edge-durable-objects.md)
- [EPIC-021 - Audio live evenementiel par clips HLS progressifs](./EPIC-021-audio-live-evenementiel-clips-hls-progressifs.md)
- [EPIC-020 - GitLab et pipeline Cloudflare](./EPIC-020-gitlab-pipeline-cloudflare.md)
