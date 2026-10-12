# NEWS-140 - Client audio global, queue metier et player HLS clip

## Objectif

Rendre le live audio frontend persistant dans toute l'application et pilote par evenements `audio_ready`.

Le player HLS ne doit exister que pendant la lecture d'un clip. Quand la queue est vide, aucun player HLS ne tourne et aucune requete playlist ne doit etre emise.

## Priorite

Priorite 3.

Ce ticket corrige les regressions UX observees avec le live HLS permanent: coupure au changement de page, player actif a vide et impossibilite d'utiliser correctement les tests vocaux pendant le live.

## Scope

- Deplacer l'etat live audio proche de la racine frontend.
- Recevoir les evenements `audio_ready` depuis le canal metadata live edge ou fallback backend.
- Ajouter les clips dans une queue metier ordonnee.
- Dedupliquer par `stream_id`.
- Rejeter les clips plus anciens que la watermark audio acceptee.
- Creer un player HLS uniquement quand un clip doit etre lu.
- Detruire le player apres `ended` si la queue est vide.
- Laisser les tests vocaux operateur fonctionner meme si le live est active.
- Afficher un etat idle sans lancer HLS.

## Avancement implementation

- L'etat audio reste porte par `FilFinanceApp`, au-dessus des pages internes.
- Le shell applicatif affiche maintenant un controle audio global visible sur Live, Tests, Profil et Admin.
- Le canal metadata dispatch maintenant `audio_ready`, `audio_failed`, `audio_finalized` et le snapshot `audio_events`.
- Activer le live ne lance plus `/audio/live/playlist.m3u8`: le client attend des clips `audio_ready`.
- Les clips HLS sont queues, dedupliques par `stream_id`, lus un par un, puis le player est detruit quand la queue est vide.
- L'admission HLS utilise maintenant le timestamp metier quand il est present dans `audio_ready`; `published_at` reste seulement un fallback de compatibilite.
- `audio_failed` retire un clip courant ou en attente et laisse les suivants continuer.
- Le reset de test vocal ne coupe plus durablement le live HLS evenementiel; il vide seulement le clip courant/la queue.

## Hors scope

- Generation backend des clips.
- Modification du moteur TTS.
- Personnalisation utilisateur de la voix live.

## Approche technique

Le frontend traite `audio_ready` comme un evenement metier, pas comme un ordre de maintenir un flux live ouvert.

```text
audio_ready
  -> dedup stream_id
  -> admission watermark
  -> queue audio globale
  -> player HLS clip
  -> ended
  -> clip suivant ou destruction player
```

Le provider audio doit rester monte quand l'utilisateur change de page interne.

## Criteres d'acceptation

- Changer de page ne coupe pas le live audio: OK, l'etat audio reste au niveau `FilFinanceApp` et le controle global reste visible dans `AppShell`.
- Un idle audio ne genere aucune requete HLS: OK sur le chemin nominal `audio_ready`.
- Le frontend ne poll plus `/api/audio/live` pour decouvrir un clip: OK pour l'activation live.
- Le frontend peut recevoir `audio_ready` depuis l'edge ou depuis le fallback backend `/ws/news`.
- Plusieurs news successives sont lues dans l'ordre: OK par tri `timestamp` metier puis `translation_id`, avec fallback `published_at` pour les anciens evenements.
- Un clip en erreur ne bloque pas les clips suivants: OK.
- Les tests vocaux manuels restent disponibles pendant que le live est active: OK cote reset global et controle shell; validation navigateur a faire.

## Tests attendus

- Test de queue ordonnee et dedupliquee.
- Test admission watermark.
- Test destruction du player quand queue vide.
- Test changement de page sans coupure.
- Test coexistence live audio et test vocal operateur.
- Test erreur clip puis passage au clip suivant.

## Dependances

- [NEWS-137](./NEWS-137-contrat-audio-ready-clip-hls-progressif.md)
- [NEWS-139](./NEWS-139-publication-backend-pubsub-audio.md)
- [EPIC-013](../epics/EPIC-013-refonte-frontend-feature-based-nativewind.md)
- [EPIC-021](../epics/EPIC-021-audio-live-evenementiel-clips-hls-progressifs.md)
