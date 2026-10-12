# NEWS-172 - Bugfix resume audio du jour et jour Europe/Paris

## Constat

Le resume audio du jour pouvait lire des news de la veille autour du changement de jour. Le symptome est visible quand le client considere deja la nouvelle journee, alors que l'origin HLS conserve encore le dossier replay du jour precedent.

## Cause

- L'origin HLS calculait son `local_day` avec la timezone locale du process. En production Docker, cette timezone peut etre UTC. Entre minuit Europe/Paris et minuit UTC, `/audio/replay/today/playlist.m3u8` pouvait donc rester attache a J-1 du point de vue produit.
- Le replay classait aussi les segments par jour de generation TTS. Une news J-1 rattrapee apres minuit pouvait donc etre publiee dans le replay de J.

## Correctif

- Le jour HLS du replay est maintenant calcule en Europe/Paris a partir de l'horloge UTC, sans dependre de `TZ`.
- Une publication replay est acceptee seulement si le timestamp metier de la news appartient au jour produit courant.
- Le calcul d'offset Europe/Paris est centralise dans `shared::time` et reutilise par le formatage horaire des news.
- Les tests verrouillent le passage a J+1 a `22:30Z` en ete et `23:30Z` en hiver, ainsi que le skip d'une news hors jour courant avant appel a `ffmpeg`.

## Validation attendue

- Apres minuit Europe/Paris, le resume audio du jour cible le dossier HLS du nouveau jour meme si le conteneur est en UTC.
- Une news de la veille traitee apres minuit Europe/Paris ne doit pas apparaitre dans le resume audio du jour.
- Les segments de J-1 restent exclus de la playlist `/audio/replay/today/playlist.m3u8`.
- Le formatage horaire des narrations conserve le meme comportement Europe/Paris.
