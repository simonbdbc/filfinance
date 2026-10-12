# NEWS-139 - Publication backend vers pub/sub audio

## Objectif

Brancher le backend Rust sur le control plane temps reel pour publier les evenements audio du nouveau modele par clips HLS.

Le client doit recevoir `audio_ready` sans polling et pouvoir rattraper les derniers evenements apres reconnexion.

## Priorite

Priorite 2.

Ce ticket depend du contrat `NEWS-137` et du comportement origin `NEWS-138`.

## Scope

- Publier `audio_ready` apres validation de la playlist finale du clip.
- Publier `audio_failed` quand la generation du clip echoue.
- Publier eventuellement `audio_finalized` pour observabilite.
- Conserver un historique court d'evenements audio pour reconnexion.
- Exposer un catch-up par watermark si le canal edge ne suffit pas.
- Correler les logs par `translation_id`, `stream_id` et event id.
- Appliquer une watermark audio monotone avant de publier ou diffuser une news en live audio.
- Garder les erreurs edge non bloquantes pour ingestion, traduction et persistence.

## Hors scope

- Generation HLS du clip.
- UI frontend.
- Load test 1000 clients.
- Stockage long terme de tous les evenements audio.

## Approche technique

Le backend reste source de verite metier. Il publie une seule copie de chaque evenement audio vers le control plane:

```text
Backend Rust
  -> audio_ready
  -> POST /publish/live ou fallback backend WS
  -> Durable Object
  -> clients connectes
```

Le control plane transporte seulement des metadata. Le data plane audio reste la playlist HLS du clip.

## Criteres d'acceptation

- Le client n'a pas besoin de poller `/api/audio/live` pour decouvrir un clip.
- Une reconnexion peut rattraper les derniers `audio_ready` non acceptes.
- Les evenements sont dedupliquables par `stream_id`.
- Une news plus ancienne que la derniere news admise audio est rejetee avant toute publication HLS ou `audio_ready`.
- Une panne edge est loguee mais ne bloque pas le pipeline news.
- Les logs permettent de comprendre ou un clip a echoue: intro, traduction, TTS corps, HLS ou publication.

## Tests attendus

- Test publication `audio_ready` apres finalisation origin.
- Test publication `audio_failed` sur erreur HLS/TTS.
- Test historique court de reconnexion.
- Test watermark: un evenement plus ancien que la reference audio est ignore ou non republie: OK cote runtime TTS.
- Test panne edge non bloquante.

## Avancement implementation

- Le contrat `audio_ready` / `audio_failed` / `audio_finalized` est transportable par le publisher edge existant sans audio binaire.
- Le Durable Object conserve un historique court borne des evenements audio dans le snapshot de reconnexion.
- Le contrat client expose `snapshot.audio_events` pour que la future queue audio puisse rattraper les clips non acceptes apres reconnexion.
- Le worker TTS applique une watermark monotone `timestamp metier + translation_id` avant toute generation live; une news plus ancienne n'entre pas dans `audio_ready`.
- `audio_ready` embarque maintenant `news_event_id` et `timestamp` pour la correlation et l'admission client, sans devenir une source de refresh historique.
- Le worker TTS prepare l'intro, finalise le corps HLS, puis emet `audio_ready` quand le clip finalise est lisible, meme si le publisher edge est desactive.
- Le fallback backend `/ws/news` transporte maintenant les memes evenements `audio_ready`, `audio_failed` et `audio_finalized` que l'edge. Cela permet de tester le live clip HLS quand `NEWS_EDGE_PUBLISH_ENABLED=false`.
- `audio_finalized` ou `audio_failed` reste publie sans dependance a un transport audio legacy; le client stable demarre sur `audio_ready`, pas sur une playlist mutable.
- La v1 garde la TTS du corps apres traduction; le streaming direct `Piper -> ffmpeg` reste hors scope.
- L'optimisation "intro immediate puis corps finalise" est sortie du contrat stable et suivie par [`EPIC-033`](../epics/EPIC-033-audio-live-intro-immediate-corps-finalise.md).

## Dependances

- [NEWS-137](./NEWS-137-contrat-audio-ready-clip-hls-progressif.md)
- [NEWS-138](./NEWS-138-origin-hls-clip-progressif-intro-corps.md)
- [EPIC-018](../epics/EPIC-018-metadata-live-edge-durable-objects.md)
- [EPIC-021](../epics/EPIC-021-audio-live-evenementiel-clips-hls-progressifs.md)
