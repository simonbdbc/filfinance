# NEWS-173 - Bugfix disponibilite du resume audio HLS du jour

## Constat

Le feed pouvait afficher l'action "Lancer le resume audio du jour" des qu'une news
texte existait dans l'historique du jour. Dans certains cas, le lancement echouait
cote client avec:

```text
Impossible de lancer le replay HLS du jour depuis le resume de la journee:
Aucun segment replay HLS disponible pour la journee courante
```

Le symptome est coherent avec une news visible dans l'historique, car l'historique
texte et le replay audio HLS ne sont pas produits par le meme index.

## Cause

- Le bouton du feed utilisait `currentDayHistory.length > 0` comme proxy de
  disponibilite audio.
- Le replay HLS du jour depend de segments TTS deja publies dans l'index HLS
  courant. Apres un redemarrage, un retard TTS ou une news sans segment audio
  encore finalise, l'historique peut donc contenir des news alors que
  `/api/replay/today` annonce `available=false`.
- La metadata HLS etait seulement chargee au moment du clic, ce qui transformait
  un etat normal "audio pas encore pret" en erreur utilisateur.

## Correctif

- Le runtime audio rafraichit maintenant regulierement la metadata
  `/api/replay/today` quand une session web authentifiee est active.
- Le runtime applicatif expose explicitement:
  - `isDailyAudioReplayAvailable`;
  - `isDailyAudioReplayAvailabilityKnown`.
- Le feed desactive l'action de replay tant que la disponibilite HLS du jour
  n'est pas connue ou tant que le replay n'a aucun segment courant.
- Le libelle passe a "Verification du resume audio..." pendant le chargement,
  puis "Resume audio en preparation" quand l'historique existe sans segment HLS.
- Le message d'erreur restant, en cas de course au clic, explique que le resume
  audio n'a pas encore de segment genere pour les news visibles.

## Validation attendue

- Si une news du jour existe mais que `/api/replay/today.available=false`, le
  bouton ne lance plus le player et n'ajoute plus d'erreur rouge dans le feed.
- Si `/api/replay/today.available=true`, le bouton garde le comportement existant
  et lance le replay HLS depuis la premiere news visible compatible.
- Un changement d'URL backend, de session ou de jour force une nouvelle metadata
  avant de considerer le replay disponible.
