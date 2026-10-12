# NEWS-142 - Observabilite, smokes et validation latence

## Objectif

Ajouter les mesures et smokes necessaires pour valider le nouveau live audio evenementiel par clips HLS finalises.

La validation doit prouver a la fois la faible latence et l'absence de requetes HLS en idle.

## Priorite

Valide le 12 mai 2026 comme gate de cloture de `EPIC-021`.

Les suites d'observabilite plus larges sont rattachees a `EPIC-005` et `EPIC-006`.

## Scope

- Mesurer le delai news recue vers `audio_ready`.
- Mesurer le delai `audio_ready` vers debut audible client.
- Mesurer le delai intro vers corps.
- Compter les requetes playlist et segments par clip.
- Verifier zero requete HLS quand la queue est vide.
- Journaliser les erreurs par etape: intro TTS, traduction, TTS corps, HLS, publication, client.
- Adapter le smoke runtime GitLab au nouveau modele.
- Documenter les seuils de go/no-go.

## Hors scope

- Load test 1000 clients complet si l'environnement Cloudflare cible n'est pas encore pret.
- Optimisation `Piper -> ffmpeg` en streaming.
- LL-HLS.
- Modification du pipeline CI GitLab dans ce topic.

## Avancement implementation

- Le backend logue `audio_ready_pipeline_duration_ms` et `news_received_to_audio_ready_ms` lors de la publication du clip finalise.
- Le backend logue `intro_to_body_gap_ms` lors de la finalisation du corps.
- L'origin HLS logue chaque requete playlist et segment par `stream_id`, ce qui permet de compter les hits par clip.
- Le snapshot `/api/runtime/hls` expose les compteurs HTTP HLS par famille: clip evenementiel et replay du jour.
- Le client logue `audio_ready_to_client_playing_ms` au demarrage effectif du clip.
- Le client logue la fin de clip par `stream_id` et continue la queue.
- Le backend fallback `/ws/news` transporte `audio_ready` quand l'edge n'est pas configure, ce qui rend le run E2E testable sans WebSocket audio dedie.
- Le smoke runtime ne teste plus `/audio/live/playlist.m3u8`: cette radio legacy n'est plus routee.
- Le smoke GitLab reste a durcir dans un topic CI dedie pour generer et verifier un clip complet.

## Protocole validation produit

Ces validations se font apres deploiement, sans modifier le pipeline CI dans ce topic.

### 1. Baseline idle

Objectif: prouver que le live active mais sans news ne garde aucun player HLS ouvert.

1. Activer le live audio global dans le client.
2. Fermer toute lecture replay explicite.
3. Relever `/api/runtime/hls` au debut de l'idle.
4. Attendre 30 minutes sans news audio.
5. Relever a nouveau `/api/runtime/hls`.

Resultat attendu:

- `clip_playlist_requests_total` inchange;
- `clip_segment_requests_total` inchange;
- `replay_playlist_requests_total` inchange si aucun replay n'a ete lance explicitement;
- aucune requete vers `/audio/live/*`, route absente du runtime.

### 2. Clip reel

Objectif: verifier la latence et le cycle de vie d'un clip HLS finalise.

1. Laisser le live audio global active.
2. Declencher une news reelle ou un scenario de test metier qui produit un `TranslatedNewsItem`.
3. Observer les logs backend pour `audio_ready_pipeline_duration_ms` et `news_received_to_audio_ready_ms`.
4. Observer la reception `audio_ready` cote client.
5. Observer le log client `audio_ready_to_client_playing_ms`.
6. Verifier que le player HLS est detruit quand le clip est termine et que la queue est vide.

Resultat attendu:

- la premiere lecture audible demarre sans polling client;
- la playlist du clip est demandee uniquement pendant la lecture du clip;
- les segments du clip sont servis sous `/audio/clips/{stream_id}/...`;
- aucun player HLS ne reste actif apres `ended` si la queue est vide.

### 3. Intro vers corps

Objectif: verifier que la production interne intro puis corps aboutit a un clip finalise stable.

1. Sur le meme clip, verifier que l'intro est produite avant le corps dans les logs backend.
2. Verifier la finalisation backend avec `intro_to_body_gap_ms`.
3. Verifier que la playlist finale contient `#EXT-X-ENDLIST`.

Resultat attendu:

- `audio_ready` n'est emis qu'apres disponibilite de la playlist finale;
- aucune playlist ne reference un segment absent;
- le timeout corps produit `audio_failed` ou une erreur loguee sans bloquer la queue suivante.

### 4. Navigation frontend

Objectif: verifier que le lecteur global n'est plus attache a une page.

1. Demarrer la lecture d'un clip.
2. Changer de page pendant la lecture.
3. Lancer un test vocal manuel si disponible.
4. Revenir sur la page principale.

Resultat attendu:

- le clip live ne se coupe pas au changement de page;
- le test vocal manuel reste utilisable;
- la queue live reprend les clips suivants dans l'ordre.

## Mesures attendues

- `news_received_to_audio_ready_ms`
- `audio_ready_to_client_playing_ms`
- `intro_to_body_gap_ms`
- `clip_playlist_request_count` via logs `hls clip playlist requested`
- `clip_segment_request_count` via logs `hls clip segment requested`
- `clip_playlist_requests_total` via `/api/runtime/hls`
- `clip_segment_requests_total` via `/api/runtime/hls`
- `replay_playlist_requests_total` via `/api/runtime/hls`
- `replay_segment_requests_total` via `/api/runtime/hls`
- `idle_hls_request_count`
- `audio_clip_failed_total`

## Criteres d'acceptation

- Idle 30 minutes: aucune requete HLS clip/replay hors action replay explicite.
- Premiere lecture audible sans polling.
- Plusieurs news successives sont lues dans l'ordre.
- Changement de page sans coupure.
- Timeout corps teste et visible dans les logs.
- Le smoke runtime valide les dependances HLS et le replay explicite sans demarrer de player permanent.

## Tests attendus

- Smoke backend de generation clip.
- Smoke frontend ou navigateur: `audio_ready` declenche la lecture depuis le debut du clip finalise.
- Scenario idle 30 minutes.
- Scenario deux news successives.
- Scenario changement de page pendant lecture.
- Scenario erreur corps apres intro.

## Dependances

- [NEWS-137](./NEWS-137-contrat-audio-ready-clip-hls-progressif.md)
- [NEWS-138](./NEWS-138-origin-hls-clip-progressif-intro-corps.md)
- [NEWS-139](./NEWS-139-publication-backend-pubsub-audio.md)
- [NEWS-140](./NEWS-140-client-audio-global-queue-player-hls-clip.md)
- [NEWS-141](./NEWS-141-migration-hors-hls-live-permanent.md)
- [EPIC-021](../epics/EPIC-021-audio-live-evenementiel-clips-hls-progressifs.md)
