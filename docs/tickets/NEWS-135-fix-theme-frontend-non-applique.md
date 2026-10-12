# NEWS-135 - Fix theme frontend parfois non applique

## Objectif

Corriger les cas ou le theme clair/sombre n'est pas applique de maniere coherente cote frontend.

Le symptome remonte: certaines zones du client ne suivent pas toujours le theme actif.

## Priorite

Priorite 1.

Ce ticket est un bug de coherence UI et doit passer avant d'ajouter de nouvelles pages operateur importantes.

## Hypotheses a verifier

- Des composants utilisent encore des classes NativeWind avec couleurs hardcodees.
- Des composants utilisent des styles inline non bases sur `useThemeTokens`.
- Le theme charge depuis `GET /api/me` peut arriver apres le premier rendu et laisser des sous-arbres avec des valeurs initiales.
- Certaines couleurs d'assets SVG ne basculent pas avec le theme.
- Une page nouvellement creee n'est pas enveloppee dans le provider attendu.

## Scope

- Auditer toutes les pages et primitives actuelles:
  - live;
  - settings;
  - profile;
  - tests;
  - admin;
  - historique.
- Rechercher les couleurs hardcodees restantes dans les composants React.
- Verifier le chemin de restauration du theme depuis:
  - stockage local;
  - profil utilisateur distant;
  - switch de sidebar.
- Ajouter ou completer les tests/smokes frontend utiles.
- Documenter les cas corriges dans ce ticket.

## Critere d'acceptation

- Le theme choisi s'applique apres reload.
- Le theme choisi s'applique apres reconnexion utilisateur.
- Le switch theme bascule toute l'interface visible sans zone incoherente.
- Les icones menu/close et boutons restent lisibles dans les deux themes.
- `npm run typecheck` passe.
- `npm run build:web` passe.

## Hors scope

- Ajouter un theme `system`.
- Refaire toute la direction artistique.
- Changer la palette choisie dans `NEWS-118`.
