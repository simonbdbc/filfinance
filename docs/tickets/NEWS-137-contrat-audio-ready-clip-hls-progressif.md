# NEWS-137 - Contrat `audio_ready` et clip HLS progressif

## Objectif

Figer le contrat public du nouveau live audio evenementiel avant implementation.

Le client ne doit plus decouvrir l'audio par polling. Le backend publie un evenement `audio_ready` quand un clip HLS de news possede une playlist finalisee avec `#EXT-X-ENDLIST`.

## Priorite

Priorite 0.

Ce ticket remplace le modele cible de live HLS permanent. Il doit passer avant les changements backend et frontend de `NEWS-138` a `NEWS-141`.

## Statut

Implementation livree le 4 mai 2026.

## Scope

- Definir les messages `audio_ready`, `audio_failed` et `audio_finalized`.
- Documenter le payload minimal de `audio_ready`.
- Definir `stream_id` comme identifiant stable du clip audio.
- Relier `translation_id`, `news_event_id`, timestamp metier, `stream_id`, URL de playlist et watermark audio.
- Garantir que le control plane ne transporte jamais d'audio binaire.
- Garantir la compatibilite avec Cloudflare Durable Objects et le fallback backend.
- Definir le comportement de reconnexion et de deduplication.

## Hors scope

- Generation HLS.
- Player frontend.
- Stockage R2.
- Streaming direct `Piper -> ffmpeg`.

## Contrat `audio_ready`

Payload minimal:

```json
{
  "type": "audio_ready",
  "translation_id": "...",
  "stream_id": "...",
  "playlist_url": "/audio/clips/{stream_id}/playlist.m3u8",
  "published_at": "...",
  "target_duration_secs": 2
}
```

Champs requis:

- `translation_id`: item traduit rattache a la news source.
- `stream_id`: identifiant opaque, stable et dedupliquable du clip.
- `playlist_url`: playlist HLS du clip finalise par news.
- `published_at`: timestamp serveur de publication de l'evenement.
- `target_duration_secs`: duree cible des segments du clip.

Champs enrichis emis par le backend:

- `news_event_id`: identifiant de la news source, utile aux traces et aux smokes produit.
- `timestamp`: timestamp metier de la news, utilise uniquement par la watermark audio. Ce champ ne remplace pas `/api/translations` et ne doit pas rafraichir l'historique journalier.

## Messages associes

- `audio_ready`: playlist HLS finale disponible et lisible depuis le debut.
- `audio_failed`: generation impossible, timeout ou erreur irreversible.
- `audio_finalized`: optionnel, utile pour observabilite et debug; le client ne doit pas en dependre pour demarrer la lecture stable.

## Criteres d'acceptation

- Aucun message du control plane ne contient de chunk audio: OK, le Worker rejette `audio_base64`, `audio_bytes`, `bytes`, `chunk` et les variantes de chunks.
- `audio_ready` suffit au client pour ajouter le clip dans sa queue: OK, le payload expose `translation_id`, `stream_id`, `playlist_url`, `published_at`, `target_duration_secs`, et le backend ajoute `news_event_id`/`timestamp` pour l'ordre audio.
- `stream_id` permet la deduplication cote client et cote edge: OK, champ obligatoire dans `audio_ready` et `audio_finalized`.
- L'ordre audio est pilote par une watermark backend monotone: OK, l'admission TTS rejette une news plus ancienne avant tout transport live.
- Le contrat est representable cote Rust, cote Worker Durable Object et cote TypeScript client: OK.
- Une reconnexion peut rejouer ou rattraper les derniers evenements sans relire un clip deja accepte: contrat pret via `event_id` et `stream_id`, historique/catch-up a livrer dans `NEWS-139`.

Note de stabilisation 2026-06: `audio_ready` annonce maintenant un clip finalise, afin d'eviter qu'un player HLS reste bloque sur une playlist mutable ouverte avant l'arrivee du corps. Le retour a une intro audible immediate est documente separement dans [`EPIC-033`](../epics/EPIC-033-audio-live-intro-immediate-corps-finalise.md).

## Implementation

- Backend Rust:
  - ajout de `EdgeAudioReadyPayload`;
  - ajout de `EdgeAudioFailedPayload`;
  - ajout de `EdgeAudioFinalizedPayload`;
  - ajout des variantes `audio_ready`, `audio_failed` et `audio_finalized` dans `EdgeLiveMessage`;
  - test de serialisation metadata-only.
- Worker Durable Object:
  - ajout des trois types dans `PUBLISHABLE_MESSAGE_TYPES`;
  - validation stricte des champs requis par type;
  - rejet des payloads qui contiennent des formes audio binaires.
- Client TypeScript:
  - ajout des types `EdgeAudioReady`, `EdgeAudioFailed` et `EdgeAudioFinalized`;
  - ajout des messages dans `EdgeLiveWsMessage`;
  - `timestamp` et `news_event_id` restent optionnels pour compatibilite avec les anciens evenements edge.

## Tests attendus

- Tests de contrat Rust/TypeScript pour les nouveaux types: OK.
- Test de rejet d'un payload audio binaire dans le control plane: OK.
- Test de deduplication par `stream_id`: type pret, logique de queue a livrer dans `NEWS-140`.
- Test de tri ou rejet par watermark audio: OK cote backend, validation navigateur a completer dans `NEWS-140`.

## Validation locale

- `cargo test edge_live`: OK.
- `npm run check` dans `filfinance-edge`: OK.
- `npm run typecheck` dans `filfinance-client`: OK.

## Dependances

- [EPIC-018](../epics/EPIC-018-metadata-live-edge-durable-objects.md)
- [EPIC-021](../epics/EPIC-021-audio-live-evenementiel-clips-hls-progressifs.md)
