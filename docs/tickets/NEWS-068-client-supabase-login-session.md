# NEWS-068 - Client Supabase, login, logout et restauration de session

## Statut

Fait le 1 mai 2026.

## Objectif

Brancher le premier slice client de `Supabase Auth` dans `filfinance-client`.

## Scope livre

- ajout des dependances `@supabase/supabase-js`, `@react-native-async-storage/async-storage` et `react-native-url-polyfill`
- ajout de `src/shared/api/supabaseClient.ts`
- ajout de `src/features/auth/AuthProvider.tsx`
- ajout de `src/features/auth/AuthGate.tsx`
- ajout de `src/features/auth/AuthSessionCard.tsx`
- restauration de session au demarrage via Supabase
- ecoute des changements de session via `onAuthStateChange`
- gate de connexion avant montage des flux news, audio et test-runs
- login par email/mot de passe
- creation de compte par email/mot de passe
- message post-inscription si la confirmation email Supabase est activee
- logout depuis l'ecran principal
- variables `.env.example` ajoutees:
  - `EXPO_PUBLIC_SUPABASE_URL`
  - `EXPO_PUBLIC_SUPABASE_PUBLISHABLE_KEY`

## Limite volontaire

Ce ticket protege l'experience cliente. Depuis `NEWS-069` et `NEWS-070`, le backend peut aussi verifier les JWT sur HTTP et WebSocket quand `NEWS_AUTH_ENABLED=true`.

## Validations attendues

- `./node_modules/.bin/tsc --noEmit --pretty false`
- `npm run typecheck -- --pretty false` via un Node recent
- export web Expo smoke

## Validations executees

- `./node_modules/.bin/tsc --noEmit --pretty false`: OK
- `npm run typecheck -- --pretty false` via le runtime Node recent: OK
- export web Expo vers `filfinance-client/dist-web-smoke`: OK
- audit `StyleSheet` sur `filfinance-client/src`: OK
- audit anciens noms produit dans `filfinance`: OK

## Mise a jour creation de compte

Le 1 mai 2026, le formulaire d'auth a ete etendu avec un mode `Creation de compte`:

- creation de compte Supabase par email/mot de passe via `signUp`
- confirmation locale du mot de passe
- message utilisateur quand Supabase demande une confirmation email
- documentation dashboard mise a jour dans [SUPABASE-CONFIG](../SUPABASE-CONFIG.md)

## Prochaine etape

`NEWS-069` doit ajouter la verification JWT Supabase cote Rust et le contexte utilisateur minimal.
