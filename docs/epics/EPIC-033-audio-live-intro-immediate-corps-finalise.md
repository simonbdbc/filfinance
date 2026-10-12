# EPIC-033 - Audio live intro immediate et corps finalise

## Objectif

Optimiser le live audio evenementiel pour retrouver un demarrage audible quasi immediat sans
reintroduire le bug des playlists HLS mutables.

La cible est de jouer l'intro vocale des qu'elle est disponible, puis d'attendre et d'enchainer
proprement le corps finalise de la news.

## Probleme

Le modele stable privilegie la fiabilite: le client lit un clip HLS finalise avec
`#EXT-X-ENDLIST`, ce qui evite qu'un player reste bloque sur une playlist ouverte avant l'arrivee
du corps.

Ce choix ajoute une petite latence au demarrage audio de chaque news. D'apres les mesures
production auditees, l'ecart typique entre intro prete et corps finalise est d'environ 0,5 a 1
seconde.

## Cible produit

- L'intro demarre des que son audio est pret.
- Le corps de la news s'enchaine une seule fois, sans trou audible notable.
- La file audio continue sur la news suivante si le corps echoue ou expire.
- Le client ne revient pas au live-edge fragile et ne depend pas d'une playlist mutable bloquante.
- L'idle audio reste silencieux cote reseau: aucun polling HLS agressif quand la queue est vide.

## Pistes techniques

- Introduire un etat explicite d'intro, soit via un evenement dedie, soit via un payload
  `audio_ready` enrichi.
- Distinguer cote client le clip intro en cours du corps finalise a venir.
- Attendre `audio_finalized` avec un timeout borne avant d'enchainer le corps.
- Lire le corps depuis le debut d'un manifest finalise, pas depuis un live-edge.
- Publier un `audio_failed` exploitable si le corps TTS ou HLS echoue apres intro.
- Garder le watermark et la queue FIFO comme source d'ordre unique.

## Hors scope

- LL-HLS complet.
- Transport audio binaire via WebSocket ou Durable Object.
- Refonte globale du player audio.
- Personnalisation de voix par utilisateur.

## Tests attendus

- Backend: l'intro peut etre annoncee avant le corps, puis le corps est finalise avec
  `#EXT-X-ENDLIST`.
- Client: l'intro demarre immediatement et le corps s'enchaine exactement une fois.
- Erreur: un timeout ou echec du corps ne bloque pas la file suivante.
- Latence: mesurer `news -> intro audible` et `intro -> corps audible`.
- Non-regression: aucune requete HLS en idle, ordre FIFO conserve.

## Statut

Backlog futur. Ce chantier est une optimisation de `EPIC-021`, pas un prealable au modele stable
actuel.
