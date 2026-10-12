# NEWS-149 - Migration auth tests profil admin

## Objectif

Migrer les vues restantes vers la nouvelle structure `expo-router` et feature-first.

## Scope

- Migrer le login et les guards auth.
- Migrer les tests moderator/admin.
- Migrer le profil utilisateur.
- Migrer la page admin.
- Conserver les comportements et libelles actuels.

## Criteres d'acceptation

- La route `tests` reste protegee moderator/admin.
- La route `admin` reste protegee admin.
- Le run de test conserve sa trace actuelle.
- Le profil conserve role, email et preference theme.

## Avancement

- Le chargement profil, le role applicatif et la preference theme sont isoles dans `profile/hooks/useUserProfileRuntime`.
- La persistance du theme garde le comportement existant: application locale immediate, puis sauvegarde profil si token disponible.
- Les guards `tests` et `admin` continuent de consommer le role visible depuis le runtime racine.
- La trace de test et les etats d'UI associes sont isoles dans `test-runs/hooks/useTestRunRuntime`.
