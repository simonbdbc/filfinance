# NEWS-119 - Synchronisation theme avec user_profiles

## Statut

Implementation livree le 3 mai 2026.

## Objectif

Synchroniser le switch clair/sombre de la sidebar avec le profil utilisateur applicatif, sans perdre le fallback local utile au premier rendu.

## Scope

- Ajouter `theme` aux preferences de profil:
  - SQLite fallback;
  - Supabase Postgres cible.
- Exposer `preferences.theme` dans `GET /api/me`.
- Accepter `theme` dans `PATCH /api/me`.
- Valider strictement les valeurs `light` et `dark`.
- Appliquer le theme charge depuis le profil quand il existe.
- Persister le theme quand l'utilisateur bascule le switch de sidebar.

## Hors scope

- Theme `system`.
- Palette personnalisable par utilisateur.
- Lecture directe de `public.user_profiles` par le client Expo.

## Implementation

- Backend:
  - `PersistedUserProfile` porte maintenant `theme`.
  - `UserPreferences` expose `theme`.
  - `UpdateUserPreferencesRequest` accepte `theme` en patch tri-etat.
  - `NewsStorage` ajoute la colonne SQLite `user_profiles.theme` de facon idempotente.
  - `UserProfileStore` SQLite/Postgres persiste `preferred_tts_voice` et `theme` sans ecraser les champs absents.
  - Le backfill SQLite -> Supabase Postgres preserve aussi `theme`.
- SQL:
  - ajout de `docs/sql/migrations/0008_add_user_theme_preference.sql`;
  - ajout de `docs/sql/supabase/migrations/0002_add_user_theme_preference.sql`;
  - schema consolide SQLite et Supabase mis a jour.
- Client:
  - `CurrentUserProfileResponse.preferences.theme` est type.
  - le switch sidebar applique immediatement le theme localement;
  - le switch appelle `PATCH /api/me` avec `theme`;
  - le profil charge via `GET /api/me` restaure le theme distant s'il existe;
  - la page Profil affiche la preference theme effective cote profil.

## Criteres d'acceptation

- `GET /api/me` renvoie `preferences.theme`.
- `PATCH /api/me` accepte `theme="light"` et `theme="dark"`.
- `PATCH /api/me` refuse un theme inconnu.
- La bascule sidebar reste immediate meme si le backend est lent.
- Le fallback local reste disponible avant chargement du profil.
- `npm run typecheck` passe.
- `npm run build:web` passe.
- `cargo fmt --check` passe.
- Tests Rust profils/preferences passent.

## Validation locale

- `npm run typecheck`: OK.
- `npm run build:web`: OK.
- `cargo fmt --check`: OK.
- `cargo check`: OK.
- `cargo test user_profile`: OK.
- `cargo test profile_store`: OK.
- `cargo test me_profile_persists_user_preferences`: OK.
