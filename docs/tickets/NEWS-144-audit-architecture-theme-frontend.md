# NEWS-144 - Audit architecture et theme actuel

## Objectif

Documenter l'etat actuel du frontend avant la refonte architecturelle `EPIC-022`.

## Scope

- Cartographier les responsabilites de l'ancien `FilFinanceApp`, du shell applicatif, audio, live feed, historique, auth, tests, profil et admin.
- Identifier les cas ou le theme peut ne pas etre applique.
- Lister les tokens et styles actuels a conserver.
- Verrouiller les invariants audio et navigation.

## Constats initiaux

- L'ancien `FilFinanceApp` etait le point de concentration de la navigation, du profil utilisateur, des WebSockets, de l'audio, de l'historique, des tests et des erreurs.
- Le shell applicatif porte le chrome applicatif, le menu, le switch theme et le controle audio global.
- Les features existent, mais elles ne suivent pas encore toutes le schema `components/hooks/services/types/screens/index.ts`.
- Le theme est applique par `ThemeProvider` et tokens inline, mais il n'a pas encore de contrat separe `themes.ts` / `tokens.ts` ni de checks QA dedies.

## Etat apres premiers lots EPIC-022

- L'entree frontend utilise `expo-router/entry` avec routes fines dans `app/`.
- Le shell, les routes et les ecrans feature-first sont en place sans refonte visuelle.
- Le theme dispose maintenant de modules dedies et d'un check `qa:theme`.
- Le profil utilisateur et la preference theme sont sortis du runtime global vers `profile/hooks/useUserProfileRuntime`.
- L'API `/api/me` est rattachee a `profile/services/userProfileApi`, plus au module `auth`.
- `src/application` porte le provider runtime racine qui compose les runtimes live/audio/historique/tests deja extraits par feature.

## Invariants

- Ne pas changer les couleurs et styles actuels.
- Ne pas reintroduire `/ws/audio`.
- Ne pas reintroduire le live HLS permanent `/audio/live`.
- Garder l'audio live global au-dessus des routes.
- Garder l'historique journalier independant du pipeline audio.

## Criteres d'acceptation

- Les dependances et responsabilites actuelles sont documentees.
- Les risques theme sont identifies avant migration.
- La refonte code peut demarrer sans ambiguite sur les invariants produit.
