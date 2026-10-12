# NEWS-145 - Prototype expo-router sans changement visuel

## Objectif

Introduire un prototype `expo-router` pour FilFinance sans changer le rendu visuel nominal.

## Scope

- Passer l'entree frontend vers `expo-router/entry`.
- Creer `app/_layout.tsx`, `app/(auth)` et `app/(app)`.
- Creer les routes directes `/login`, `/live`, `/history`, `/tests`, `/profile` et `/admin`.
- Garder les route files fines et deleguer aux screens dans `src/features`.
- Conserver `AuthProvider`, `ThemeProvider` et l'audio global au niveau racine approprie.

## Hors scope

- Refonte visuelle.
- Changement des contrats backend.
- Deploy automatique du prototype.

## Criteres d'acceptation

- Le client demarre avec `expo-router`.
- Le rendu reste equivalent au shell actuel.
- Le refresh direct d'une URL web fonctionne.
- Les guards auth et roles restent operationnels.
