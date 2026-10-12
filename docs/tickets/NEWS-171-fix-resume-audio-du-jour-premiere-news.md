# NEWS-171 - Bugfix resume audio du jour depuis la premiere news

## Constat

Le resume audio du jour pouvait demarrer plus loin que la premiere news affichee pour la journee courante. Le correctif precedent forcait le lecteur HLS a lire la playlist replay depuis son debut, mais le client ne transmettait pas encore la premiere traduction visible et l'origin HLS ne gardait pas de lien entre les segments replay et la traduction source.

## Cause

- `replayAudioForHistory` ignorait la liste de news passee par les ecrans et lancait toujours le replay journalier generique.
- Les segments replay etaient indexes uniquement par sequence HLS. Le backend ne pouvait donc pas couper la playlist a partir de la premiere news attendue par le client.
- Apres redemarrage, les segments reconstruits depuis le disque ne contiennent pas encore cette metadata applicative; un index partiel ne doit pas faire demarrer le replay apres les anciens segments.

## Correctif

- Le frontend trie l'historique courant avec la meme logique que la timeline audio et transmet l'ID de la premiere traduction via `start_translation_id`.
- L'endpoint replay accepte `start_translation_id` en query string et genere une playlist qui commence au premier segment associe a cette traduction ou a une traduction posterieure.
- Les nouveaux segments replay stockent `translation_id`, `news_event_id` et `stream_id` dans l'index en memoire.
- Si aucune metadata compatible n'est disponible, ou si des segments legacy sans metadata precedent le point de coupe trouve, le backend conserve la playlist complete pour ne pas casser le replay apres redemarrage.

## Validation attendue

- Le bouton de resume audio du jour demarre sur la premiere news du jour qui dispose d'audio.
- Si la premiere news visible n'a pas encore d'audio, le replay demarre sur la premiere news suivante disponible dans le replay.
- Les segments d'une meme news restent consecutifs dans la playlist.
- Un redemarrage en cours de journee ne doit pas couper les segments du matin reconstruits depuis disque.
- Le comportement des clips HLS et des playlists sans `start_translation_id` reste inchange.

## Suite

`NEWS-174` complete ce comportement: une news visible du jour ne doit plus rester
durablement sans segment replay quand elle a ete rattrapee hors du flux live.
