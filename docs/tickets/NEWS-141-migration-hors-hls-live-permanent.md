# NEWS-141 - Migration hors HLS live permanent

## Objectif

Retirer totalement le modele `/audio/live/playlist.m3u8` du runtime applicatif.

Le live audio Fil Finance doit devenir evenementiel: un `audio_ready` declenche un clip HLS finalise par news. Le serveur ne doit plus exposer de radio HLS permanente, meme en compatibilite nominale.

## Priorite

Priorite 4.

Ce ticket clot la migration produit apres le contrat, l'origin clip, la publication pub/sub et le client global.

## Scope

- Supprimer les routes publiques `/audio/live/*`.
- Supprimer le metadata endpoint `/api/audio/live`.
- Supprimer les flags/env legacy qui reactivaient la radio permanente.
- Supprimer le demarrage automatique d'un player HLS live permanent.
- Adapter `.env.server.example` et les docs d'exploitation.
- Clarifier le statut du replay du jour.
- Documenter que le live audio n'est plus une radio HLS permanente.

## Avancement implementation

- Le client ne construit plus l'URL `/audio/live/playlist.m3u8`.
- Le client ne charge plus `/api/audio/live` et le module frontend dedie a ce metadata endpoint est supprime.
- Le type frontend `HlsLiveMetadata` est supprime: le client ne garde plus de contrat compile pour le live HLS permanent.
- Le backend ne route plus `/audio/live/*` et ne porte plus de flag `NEWS_HLS_LEGACY_LIVE_ROUTES_ENABLED`.
- Les compteurs runtime legacy sont retires; les compteurs conserves couvrent seulement clips et replay.
- `useHlsAudioPlayer` ne contient plus de boucle idle/polling live; il lit uniquement des clips HLS recus par evenement ou le replay du jour demande explicitement.
- L'ecran de connexion affiche le modele "clips via audio_ready" au lieu de presenter `HLS live` comme chemin nominal.
- `.env.server.example` documente uniquement les segments replay et clip.
- Le replay du jour reste une action separee et explicite, basee sur `/audio/replay/today/playlist.m3u8`.

## Hors scope

- Migration R2.
- LL-HLS.

## Approche technique

La migration doit separer clairement:

- live temps reel: clips HLS finalises par news, pilotes par `audio_ready`;
- replay: fonctionnalite distincte, conservee seulement si elle garde une valeur produit claire;
- historique EPIC-019: documente comme palier transitoire, sans surface runtime active.

## Criteres d'acceptation

- Un idle de plusieurs heures ne genere aucune requete playlist HLS.
- L'ancien request storm ne peut plus se produire dans le chemin nominal.
- Le client nominal ne lance plus `/audio/live/playlist.m3u8`: OK, plus de builder URL ni de start live.
- L'ancien endpoint public radio est absent du router backend.
- Les docs d'environnement de production indiquent le modele clip evenementiel: OK dans `.env.server.example`.
- Le statut du replay du jour est explicite: OK, action separee du live temps reel.

## Tests attendus

- Smoke idle 30 minutes sans requete HLS.
- Test de non-regression lecture clip.
- Test que le client ne demarre pas l'ancien live permanent et que le backend ne route plus la radio.
- Verification docs `.env.server.example` et runbooks.

## Dependances

- [NEWS-138](./NEWS-138-origin-hls-clip-progressif-intro-corps.md)
- [NEWS-139](./NEWS-139-publication-backend-pubsub-audio.md)
- [NEWS-140](./NEWS-140-client-audio-global-queue-player-hls-clip.md)
- [EPIC-021](../epics/EPIC-021-audio-live-evenementiel-clips-hls-progressifs.md)
