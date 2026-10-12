# NEWS-138 - Origin HLS clip progressif compose intro et corps

## Objectif

Creer un origin HLS par news, avec une playlist mutable en interne jusqu'a `#EXT-X-ENDLIST`.

Le backend produit l'intro puis le corps traduit dans la meme playlist, mais le chemin stable publie `audio_ready` seulement quand la playlist finale est disponible.

## Priorite

Priorite 1.

Ce ticket porte le coeur du nouveau data plane audio de `EPIC-021`.

## Statut

Implementation origin livree le 4 mai 2026.

Ce ticket livre le support HLS clip cote backend: routes publiques, playlists progressives, cache headers, target duration clip et API interne `publish_clip_intro` / `append_clip_body_and_finalize`.

L'orchestration temps reel qui appelle ces methodes et publie `audio_ready` / `audio_finalized` est branchee par `NEWS-139`. Le timeout corps explicite reste a durcir avec l'observabilite de `NEWS-142`.

## Scope

- Generer une intro TTS courte a partir du `narration_prefix` canonique de la news.
- Utiliser `HH:MM. Information importante.` si `news_level` contient le mot actif `active`.
- Utiliser `HH:MM. Nouvelle information.` dans les autres cas.
- Convertir l'heure en timezone `Europe/Paris`.
- Produire un premier segment HLS intro et une playlist initiale interne valide.
- Ecrire segments et playlists de facon atomique.
- Lancer ou poursuivre la traduction OPUS-MT en parallele de l'intro.
- Ajouter les segments TTS du corps traduit dans la meme playlist, sans repeter le `narration_prefix`.
- Inserer `#EXT-X-DISCONTINUITY` si intro et corps viennent de deux encodages separes.
- Finaliser la playlist avec `#EXT-X-ENDLIST`.
- Servir la playlist sous `/audio/clips/{stream_id}/playlist.m3u8`.
- Servir les segments sous `/audio/clips/{stream_id}/segment-000.ts`.

## Hors scope

- Player frontend.
- Publication Durable Objects.
- Streaming direct `Piper -> ffmpeg`.
- Personnalisation de voix par utilisateur.

## Approche technique

Le backend produit un dossier de clip par `stream_id`.

Les segments sont immuables et peuvent etre caches longtemps. La playlist reste mutable tant que `#EXT-X-ENDLIST` n'est pas present et doit donc etre servie avec un cache court ou revalidation obligatoire.

La publication stable `audio_ready` ne doit intervenir qu'apres:

- disponibilite publique de tous les segments du clip;
- disponibilite publique d'une playlist finale qui reference uniquement des segments existants;
- presence de `#EXT-X-ENDLIST`.

## Criteres d'acceptation

- `audio_ready` n'est jamais emis avant disponibilite du clip finalise: branche dans `NEWS-139`.
- La playlist ne reference jamais un segment absent: OK, les segments sont deplaces avant commit en memoire.
- La playlist finale contient `#EXT-X-ENDLIST`: OK.
- `#EXT-X-DISCONTINUITY` est present quand intro et corps ne partagent pas la meme timeline d'encodage: OK.
- Un timeout stoppe proprement un clip dont le corps n'arrive pas: a durcir dans `NEWS-142`.
- Une panne HLS ne bloque pas l'ingestion ni la persistence news: comportement worker branche dans `NEWS-139`.

## Implementation

- Ajout des routes publiques:
  - `/audio/clips/{stream_id}/playlist.m3u8`;
  - `/audio/clips/{stream_id}/segment-000.ts`.
- Ajout d'un index interne de clips HLS progressifs par `stream_id`.
- Ajout d'une playlist `EVENT` mutable jusqu'a finalisation.
- Ajout de `#EXT-X-ENDLIST` a la finalisation.
- Ajout de `#EXT-X-DISCONTINUITY` sur le premier segment du corps.
- Ajout de headers cache distincts:
  - playlist mutable: revalidation obligatoire;
  - playlist finalisee: cache court;
  - segments: cache immutable.
- Ajout de `NEWS_HLS_CLIP_SEGMENT_TARGET_DURATION_SECS`, defaut `2`.

## Tests attendus

- Test playlist initiale interne avec intro seule: OK.
- Test ajout du corps et finalisation `#EXT-X-ENDLIST`: OK.
- Test `#EXT-X-DISCONTINUITY` entre intro et corps: OK.
- Test ecriture atomique: couvert par commit memoire apres deplacement segment, integration `ffmpeg` a couvrir dans `NEWS-139`.
- Test timeout corps: a couvrir dans `NEWS-139`.
- Test headers cache playlist mutable vs segments immuables: OK.

## Note de stabilisation

Le modele stable privilegie la lecture de clips finalises pour eviter les blocages HLS observes quand le client ouvre une playlist mutable avant le corps. L'optimisation future qui rejoue l'intro immediatement puis enchaine le corps finalise est suivie par [`EPIC-033`](../epics/EPIC-033-audio-live-intro-immediate-corps-finalise.md).

## Validation locale

- `cargo test hls`: OK.

## Dependances

- [NEWS-137](./NEWS-137-contrat-audio-ready-clip-hls-progressif.md)
- [EPIC-019](../epics/EPIC-019-audio-hls-live-replay-du-jour.md)
- [EPIC-021](../epics/EPIC-021-audio-live-evenementiel-clips-hls-progressifs.md)
