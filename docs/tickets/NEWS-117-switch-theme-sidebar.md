# NEWS-117 - Switch theme clair/sombre dans la sidebar

## Statut

Implementation livree le 3 mai 2026.

## Objectif

Ajouter un controle de theme clair/sombre en bas de la sidebar mobile-style introduite par `NEWS-114`.

## Scope

- Ajouter un switch theme en bas de `AppShell` / sidebar.
- Afficher l'etat courant:
  - clair;
  - sombre.
- Appliquer le theme immediatement a l'interface.
- Garder le switch accessible au clavier et aux lecteurs d'ecran.
- Conserver le comportement sidebar:
  - ouverture par bouton icone haut droite;
  - fermeture par overlay;
  - fermeture apres navigation;
  - switch utilisable sans fermer la sidebar.

## Persistance

Premier lot recommande:

- persistance locale pour eviter de bloquer l'UX si le profil n'est pas charge;
- synchronisation avec `user_profiles.preferences.theme` livree ensuite par `NEWS-119`.

Cette approche evite que le theme depenne du chargement reseau au premier rendu.

## Hors scope

- Gestion d'un theme `system` automatique.
- Personnalisation avancee de palette par utilisateur.

## Criteres d'acceptation

- Le switch est visible en bas de la sidebar.
- Le theme bascule sans reload.
- Le choix est conserve au refresh local.
- Le switch ne reintroduit pas de scroll horizontal.
- `npm run typecheck` passe.
- `npm run build:web` passe.

## Implementation

- `ThemeProvider` charge la preference locale `filfinance.theme` via AsyncStorage.
- `ThemeProvider` persiste les bascules `light` / `dark` apres chargement initial.
- `AppShell` affiche un switch en bas de sidebar.
- Le switch applique le theme immediatement sans fermer la sidebar.
- Le footer de sidebar reste separe de la navigation pour eviter les changements involontaires de page.
- `NEWS-119` raccorde ensuite ce switch a `PATCH /api/me` et `preferences.theme`.

## Validation locale

- `npm run typecheck`
- `npm run build:web`
- `npm run web -- --port 19007 --non-interactive`, serveur Expo stable jusqu'a l'arret manuel.
