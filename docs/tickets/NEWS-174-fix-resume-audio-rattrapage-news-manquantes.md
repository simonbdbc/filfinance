# NEWS-174 - Bugfix rattrapage des news manquantes dans le resume audio du jour

## Constat

Le resume audio du jour pouvait ignorer une news visible dans l'historique quand une
news plus recente avait deja produit un segment HLS. Exemple observe:

- `00:31` visible dans le feed mais non lue;
- `02:08` visible et lue.

Le player demarrait donc bien le replay HLS, mais le replay ne contenait pas de
segment audio pour toutes les traductions du jour.

## Cause

- Le worker TTS consomme seulement le bus des traductions diffusees en live.
- Les traductions rattrapees ou rejouees depuis la base peuvent etre persistees
  pour l'historique sans etre rediffusees, pour eviter les doublons Telegram/WS.
- Le watermark live audio rejette volontairement une traduction plus ancienne si
  une traduction plus recente est deja admise. Cette regle est correcte pour le
  live, mais trop stricte pour le replay du jour.
- Le replay HLS ordonnait les segments par sequence de generation. Une news plus
  ancienne generee en retard ne pouvait donc pas retrouver sa place chronologique.

## Correctif

- Ajout d'un rattrapage `replay-only` dans le worker TTS:
  - il scanne les traductions reelles du jour produit courant;
  - il ignore les traductions deja presentes dans l'index replay;
  - il genere uniquement les segments replay HLS, sans publier `audio_ready`,
    sans rediffuser la news et sans passer par le watermark live.
- Les segments replay conservent maintenant le timestamp metier de la news.
- La playlist replay trie les segments metadata par timestamp metier avant de
  couper sur `start_translation_id`.
- Les segments persistants ecrivent un sidecar JSON adjacent au `.ts`, pour
  reconstruire `translation_id`, `news_event_id`, `stream_id` et `timestamp`
  apres redemarrage.
- Si l'origin detecte des segments legacy sans metadata dans le jour courant, le
  worker reconstruit l'index replay du jour depuis les traductions en base afin
  d'eviter les doublons ou les trous durables.

## Validation attendue

- Une news du jour visible dans l'historique finit par avoir un segment replay,
  meme si elle a ete rattrapee apres une news plus recente.
- Le resume audio lit les news du jour dans l'ordre metier du feed, pas dans
  l'ordre de generation TTS.
- Le live audio garde son watermark monotone et ne rejoue pas les anciennes news
  sur le flux temps reel.
- Un redemarrage conserve la correspondance segment -> traduction pour les
  nouveaux segments HLS.
