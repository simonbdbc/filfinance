# NEWS-150 - QA architecture theme non-regression frontend

## Objectif

Ajouter les validations de non-regression frontend apres refonte architecturelle.

## Scope

- Verifier le contrat des tokens theme.
- Verifier que les surfaces principales sont themees.
- Valider `npm run typecheck`.
- Valider `npm run build:web`.
- Valider les routes web directes.
- Valider l'absence de retour du legacy audio.
- Verifier que les imports externes aux features passent par les exports publics.
- Verifier les invariants UI qui ont deja regresse: selecteur theme sidebar et scroll vertical web.

## Criteres d'acceptation

- `npm run typecheck` passe.
- `npm run build:web` passe.
- Les routes `/login`, `/live`, `/history`, `/tests`, `/profile` et `/admin` sont accessibles.
- Le theme clair/sombre actuel est coherent sur toutes les routes.
- Aucune reference nominale a `/ws/audio` ou `/audio/live` n'est reintroduite.
- Les racines de features ne contiennent que `index.ts` et les dossiers autorises.
- Aucun import `@features/<feature>/<fichier>` ne contourne l'API publique d'une feature.
- La sidebar conserve un selecteur theme clair/sombre en bas du menu.
- Le shell web conserve un conteneur scrollable vertical et la sidebar ne bloque pas le scroll interne.

## Avancement

- `npm run qa:frontend` valide le contrat theme, l'absence de legacy audio dans `src`, le contrat architecture feature-first, et le typecheck.
- `npm run build:web` exporte correctement le client web avec `expo-router`.
- `npm run qa:architecture` verifie automatiquement qu'aucun fichier metier ne reste a la racine d'une feature en dehors de `index.ts`.
- `npm run qa:architecture` verifie automatiquement que les routes et modules applicatifs importent les features via `@features/<feature>` plutot que par fichiers internes.
- `npm run qa:architecture` valide aussi `src/application`, bloque le retour de l'alias `@app` et interdit l'ancien module `@features/app-shell`.
- `npm run qa:shell` verifie automatiquement la presence du selecteur theme sidebar et les invariants de scroll vertical web.
- L'export web sert `/login`, `/live`, `/history`, `/tests`, `/profile` et `/admin` en 200 via fallback SPA.
- Validation navigateur connectee: `/live`, `/history`, `/profile`, `/tests` et `/admin` rendent sans erreur console apres chargement du profil admin.
- Validation navigateur connectee: le theme bascule clair/sombre depuis la sidebar puis revient a l'etat initial sans erreur console.
- Validation navigateur connectee: l'audio global reste active lors d'une navigation SPA `/live` -> `/history` -> `/profile`, puis se desactive proprement.
- Validation navigateur connectee: l'historique journalier long scrolle verticalement et affiche les items sans erreur console.
