# EPIC-021 - Audio live evenementiel par clips HLS progressifs

## Objectif

Remplacer le modele nominal de live audio HLS permanent par un modele evenementiel:

- control plane temps reel leger via Durable Objects ou fallback backend
- data plane audio via clips HLS finalises par news
- publication `audio_ready` quand la playlist du clip est finalisee avec `#EXT-X-ENDLIST`
- production interne possible en deux temps intro + corps, mais lecture client uniquement sur clip finalise
- aucun player HLS actif quand la file audio est vide

## Valeur livree

Le live audio devient un enchainement de clips metier, declenches par evenement temps reel, au lieu d'une radio HLS permanente.

Cette cible corrige les deux limites du modele `EPIC-019`:

- un live idle ne doit plus rafraichir une playlist vide pendant des heures;
- le client ne doit plus decouvrir les prochains audios par polling, car ce polling ajoute une latence produit non acceptable.

## Statut

Valide le 12 mai 2026.

`EPIC-021` devient la cible audio live officielle.

`EPIC-019` reste documente comme etape transitoire: son origin HLS, ses headers CDN et son replay du jour peuvent etre reutilises, mais le modele "live HLS global permanent" est abandonne pour le chemin nominal.

`NEWS-137` est implemente le 4 mai 2026: le contrat `audio_ready` / `audio_failed` / `audio_finalized` existe cote Rust, Worker Durable Object et client TypeScript, avec tests metadata-only.

Decision de cloture: le modele clips HLS finalises par news, declenches par `audio_ready`, est retenu comme comportement produit valide. Les travaux suivants portent sur la robustesse du pipeline audio via `EPIC-024`, pas sur le modele live audio lui-meme.

Note de stabilisation: le modele stable actuel privilegie la fiabilite en lisant des clips HLS finalises plutot qu'un live-edge fragile sur playlist mutable. L'optimisation future "intro immediate puis corps finalise enchaine" est suivie dans [`EPIC-033`](./EPIC-033-audio-live-intro-immediate-corps-finalise.md).

## Architecture cible

```text
NewsEvent
  -> intro TTS courte
  -> premier segment HLS du clip
  -> traduction OPUS-MT
  -> TTS du corps FR
  -> segments HLS corps
  -> playlist complete
  -> #EXT-X-ENDLIST
  -> audio_ready via control plane
  -> client queue audio globale
  -> player HLS clip depuis le debut
```

## Decisions

- Le live audio nominal n'expose plus de flux permanent `/audio/live/*`.
- Chaque news audio possede un `stream_id` et une URL de playlist finie: `/audio/clips/{stream_id}/playlist.m3u8`.
- L'intro vocale cible reprend le `narration_prefix` canonique: `HH:MM. Information importante.` pour une news active, sinon `HH:MM. Nouvelle information.`
- Le corps audio du clip lit uniquement le texte traduit de la news, car le `narration_prefix` a deja ete lu dans l'intro.
- L'heure de l'intro est convertie en heure locale `Europe/Paris`.
- La duree cible des segments clip est `2s` par defaut.
- Le control plane transporte uniquement des metadata: `audio_ready`, `audio_failed` et eventuellement `audio_finalized`.
- Le control plane ne transporte jamais d'audio binaire.
- `audio_ready` annonce un clip HLS finalise; l'optimisation intro immediate est sortie dans `EPIC-033`.
- Le player HLS est cree seulement pendant la lecture d'un clip et detruit quand la queue est vide.
- La file audio est globale cote frontend et survit aux changements de page.
- L'ordre audio est pilote par watermark backend, pas par l'ordre d'arrivee reseau.
- Le streaming direct `Piper -> ffmpeg` reste hors scope v1; la premiere optimisation demarre apres disponibilite du TTS intro.

## Scope

Dans cet EPIC:

- definir le contrat `audio_ready` et les messages associes
- produire des clips HLS par news, progressifs en interne puis finalises avant publication
- finaliser la playlist avec `#EXT-X-ENDLIST`
- publier l'evenement audio apres finalisation du clip complet
- deplacer le player audio dans un provider frontend global
- supprimer le polling client pour decouvrir l'audio live
- rendre l'idle audio silencieux cote reseau
- ajouter les smokes et mesures de latence propres au nouveau modele

Hors scope:

- streaming direct de la sortie Piper vers `ffmpeg`
- LL-HLS complet
- personnalisation de voix par utilisateur pour le live
- stockage R2 obligatoire
- transport audio binaire via Durable Objects

## Tickets de dev

### [`NEWS-137` Contrat `audio_ready` et clip HLS progressif](../tickets/NEWS-137-contrat-audio-ready-clip-hls-progressif.md)

Objectif: figer le contrat public du nouveau live audio evenementiel.

Scope:

- messages `audio_ready`, `audio_failed` et `audio_finalized`
- payload minimal `audio_ready`
- `stream_id` stable et dedupliquable
- watermark audio backend
- compatibilite Durable Objects et fallback backend

Criteres d'acceptation:

- le control plane ne contient aucun chunk audio
- `audio_ready` suffit a demarrer une lecture du clip finalise cote client
- un client reconnecte peut dedupliquer les evenements recus
- le contrat documente la relation entre `translation_id`, `stream_id` et playlist HLS

### [`NEWS-138` Origin HLS clip progressif compose intro et corps](../tickets/NEWS-138-origin-hls-clip-progressif-intro-corps.md)

Objectif: produire une playlist HLS par news, mutable en interne jusqu'a `#EXT-X-ENDLIST`.

Statut: origin backend implemente le 4 mai 2026; orchestration `audio_ready` et timeout corps a brancher dans `NEWS-139`.

Scope:

- intro TTS locale courte
- premier segment intro publie atomiquement
- playlist initiale interne valide
- ajout du corps apres traduction et TTS
- `#EXT-X-DISCONTINUITY` si intro et corps viennent d'encodages separes
- headers cache differencies entre playlist mutable et segments immuables

Criteres d'acceptation:

- `audio_ready` n'est jamais emis avant disponibilite de la playlist finalisee
- la playlist ne reference jamais un segment absent
- la playlist finale contient `#EXT-X-ENDLIST`
- un timeout stoppe proprement un clip dont le corps n'arrive pas

### [`NEWS-139` Publication backend vers pub/sub audio](../tickets/NEWS-139-publication-backend-pubsub-audio.md)

Objectif: publier les evenements audio depuis le backend vers le control plane temps reel.

Scope:

- publication `audio_ready`
- publication `audio_failed`
- historique court pour reconnexion
- catch-up par watermark si necessaire
- logs correles par `translation_id` et `stream_id`

Criteres d'acceptation:

- le client n'a pas besoin de poller pour decouvrir un nouveau clip
- une panne edge ne bloque ni ingestion ni persistence
- les evenements audio restent ordonnes et dedupliquables
- le worker TTS applique une watermark `timestamp metier + translation_id` avant HLS et avant publication `audio_ready`
- le WebSocket audio binaire `/ws/audio` est retire du modele cible et ne sert plus de fallback

### [`NEWS-140` Client audio global, queue metier et player HLS clip](../tickets/NEWS-140-client-audio-global-queue-player-hls-clip.md)

Objectif: rendre le live audio persistant dans l'app et pilote par evenements `audio_ready`.

Scope:

- provider audio place proche de la racine frontend
- reception des evenements audio par le canal metadata live
- queue metier ordonnee
- creation du player HLS uniquement pendant la lecture
- destruction du player quand la queue est vide
- coexistence avec les tests vocaux operateur

Criteres d'acceptation:

- changer de page ne coupe pas le live audio
- idle audio ne genere aucune requete HLS
- plusieurs news successives sont lues dans l'ordre
- un clip en erreur ne bloque pas les clips suivants

### [`NEWS-141` Migration hors HLS live permanent](../tickets/NEWS-141-migration-hors-hls-live-permanent.md)

Objectif: retirer le modele `/audio/live/playlist.m3u8` du runtime.

Scope:

- supprimer les anciens endpoints `/audio/live/*` et `/api/audio/live`
- supprimer le demarrage automatique d'un player live permanent
- adapter les docs d'environnement
- clarifier le statut du replay du jour

Criteres d'acceptation:

- un idle de plusieurs heures ne genere aucune requete playlist
- l'ancien request storm ne peut plus se produire dans le chemin nominal
- les docs indiquent que le live audio est evenementiel, pas une radio HLS permanente

### [`NEWS-142` Observabilite, smokes et validation latence](../tickets/NEWS-142-observabilite-smokes-validation-latence.md)

Objectif: mesurer et valider le nouveau chemin audio evenementiel.

Scope:

- mesure news recue vers `audio_ready` finalise
- mesure `audio_ready` vers debut audible client
- mesure intro vers corps
- comptage requetes playlist/segments par clip
- verification zero requete HLS en idle
- smoke runtime adapte

Criteres d'acceptation:

- idle 30 minutes: aucune requete HLS
- premiere lecture audible sans polling, depuis un clip finalise
- changement de page sans coupure
- timeout corps teste
- latence et erreurs exposees dans les logs/smokes

## Definition of Done

- le client nominal recoit `audio_ready` via control plane temps reel
- les clips HLS sont finis par news et finalises avec `#EXT-X-ENDLIST`
- le player HLS n'existe pas quand la queue est vide
- le live audio survit aux changements de page
- l'ancien HLS live permanent n'est plus expose par le runtime
- la validation runtime prouve l'absence de requetes HLS en idle

## References

- [EPIC-018 - Metadata live edge Durable Objects](./EPIC-018-metadata-live-edge-durable-objects.md)
- [EPIC-019 - Audio HLS live et replay du jour](./EPIC-019-audio-hls-live-replay-du-jour.md)
- [NEWS-136 - Fix request storm HLS live a vide](../tickets/NEWS-136-fix-hls-live-idle-request-storm.md)
