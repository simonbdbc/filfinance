# NEWS-136 - Fix request storm HLS live a vide

## Objectif

Empecher le client ou le player HLS de multiplier les requetes sur `/audio/live/playlist.m3u8` quand le live audio est active mais qu'aucun segment n'est encore disponible.

Symptome remonte le 3 mai 2026:

```text
https://filfinance-api.sbcenter.fr/audio/live/playlist.m3u8
```

Plus de `32000` requetes peuvent etre emises en quelques secondes quand il n'y a rien a ecouter.

## Priorite

Priorite 0.

Une playlist HLS vide peut etre techniquement valide, mais un polling non borne de ce volume n'est pas acceptable en production. Ce ticket doit passer avant le tuning `NEWS-131`.

Note du 4 mai 2026: ce ticket est une mitigation du modele HLS live permanent. Le modele cible est maintenant `EPIC-021`: `audio_ready` temps reel + clips HLS finalises par news, sans player HLS actif en idle.

## Analyse initiale

Ce qui est normal:

- un player HLS live recharge regulierement la playlist;
- une playlist vide peut servir de point d'attente avant le premier segment.

Ce qui n'est pas normal:

- des dizaines de milliers de requetes en quelques secondes;
- une boucle de retry sans backoff;
- un client qui demarre un player HLS quand l'origin annonce explicitement qu'aucun audio n'est disponible.

## Hypotheses a verifier

- `hls.js` en `lowLatencyMode` poll trop agressivement une playlist live vide.
- Le client demarre HLS live sans verifier la disponibilite d'un segment.
- La playlist vide ne fournit pas assez d'information pour que le player calcule un intervalle raisonnable.
- Le cache `max-age=1` ne suffit pas si chaque navigateur bypass ou si Cloudflare ne cache pas l'URL en l'etat.
- Une erreur manifeste entraine un cycle stop/start cote client.

## Scope

- Mesurer les requetes playlist cote navigateur et cote origin.
- Ajouter un garde frontend avant demarrage live HLS:
  - ne pas initialiser `hls.js` si aucun segment live n'est disponible;
  - afficher un etat `en attente de la prochaine news audio`;
  - reessayer avec backoff borne.
- Ajouter une metadata backend ou etendre `/api/runtime/hls` pour exposer:
  - `live_available`;
  - nombre de segments live;
  - derniere sequence;
  - URL playlist.
- Ajuster la playlist vide si necessaire pour ralentir le refresh.
- Desactiver `lowLatencyMode` quand la playlist est vide si c'est la source du storm.
- Ajouter un test ou smoke qui verifie que le mode idle ne lance pas de polling HLS intensif.

## Implementation

- Ajout de `/api/audio/live`, protege listener, avec `available`, `segment_count`, bornes de sequence, URL playlist et `retry_after_ms`.
- Ajout des champs `live_available`, `live_segment_count`, `first_live_sequence`, `latest_live_sequence` et `live_playlist_url` dans `/api/runtime/hls`.
- Le client web lit la metadata live avant de lancer HLS; si aucun segment n'existe, il reste en etat `waiting` et reteste avec un intervalle borne entre 3s et 15s.
- `hls.js` et l'element `<audio>` ne sont plus instancies pendant l'attente d'un premier segment live.
- Audit de reprise: le garde frontend seul ne couvre pas les bundles anciens, les onglets deja ouverts ni les lecteurs qui appellent directement `/audio/live/playlist.m3u8`.
- L'origin HLS renvoie maintenant `204 No Content` avec `Retry-After: 5` et cache court quand la fenetre live est vide ou stale; il ne sert plus de manifest live vide interpretable comme une diffusion active.
- La disponibilite live tient compte d'une watermark de fraicheur par segment publie: les segments anciens restent disponibles pour le replay du jour, mais ne maintiennent plus le live actif indefiniment.
- Le lecteur web desactive `lowLatencyMode`, borne les retries manifest/playlist et repasse en polling metadata lorsqu'une erreur fatale HLS survient apres le demarrage.
- Le smoke runtime accepte un live idle en `204` et continue de verifier que le replay du jour expose une playlist HLS valide.

## Critere d'acceptation

- Activer le live audio sans segment disponible ne provoque pas de request storm.
- Le client indique un etat idle comprehensible.
- Le player HLS demarre automatiquement ou manuellement quand un segment devient disponible.
- La cadence de polling idle est bornee et documentee.
- `/audio/live/playlist.m3u8` retourne `204` quand aucun segment live frais n'est disponible.
- Le replay du jour n'est pas degrade.
- `npm run typecheck` et `npm run build:web` passent pour la partie client.
- `cargo test` passe si le backend expose une nouvelle metadata HLS.

## Hors scope

- LL-HLS complet.
- Migration audio vers R2.
- Tuning segment `2s`, couvert par `NEWS-131`.
