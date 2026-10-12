# NEWS-148 - Migration live audio global historique

## Objectif

Migrer les domaines live, audio et historique dans la nouvelle architecture sans changer le comportement.

## Scope

- Garder l'audio live au-dessus des routes applicatives.
- Conserver la queue audio pilotee par `audio_ready`.
- Conserver l'ordre audio par timestamp metier et watermark.
- Separer la vue live de la vue historique.
- Garder l'historique journalier independant de l'audio.

## Criteres d'acceptation

- Changer de route ne coupe pas l'audio live.
- Les clips HLS sont lus dans l'ordre.
- Aucune requete HLS n'est emise quand la queue est vide.
- `/ws/audio` et `/audio/live` restent absents du chemin nominal.

## Avancement

- L'historique journalier est isole dans `history/hooks/useHistoryRuntime`.
- Les refs utilisees pour resoudre les clips audio restent stables et fournies au player HLS depuis le runtime historique.
- Le canal live metadata est isole dans `live-feed/hooks/useLiveMetadataRuntime`.
- La queue live, l'item courant, les actions de nettoyage et les callbacks edge audio sont isoles dans `audio/hooks/useAudioRuntime`.
- Le runtime racine conserve l'audio global au-dessus des routes et ne relance pas de player HLS permanent.
