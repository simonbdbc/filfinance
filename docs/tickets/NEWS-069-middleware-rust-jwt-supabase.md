# NEWS-069 - Middleware Rust de verification JWT Supabase

## Objectif

Ajouter au backend `filfinance-service` une verification serveur des access tokens `Supabase Auth` et un contexte utilisateur injectable dans les handlers HTTP.

## Statut

Implemente le 1 mai 2026.

## Scope livre

- ajout de la dependance `jsonwebtoken` avec provider crypto `rust_crypto`
- ajout de `AuthConfig` dans la configuration Rust
- lecture des variables serveur `NEWS_AUTH_ENABLED`, `NEWS_SUPABASE_PROJECT_URL`, `NEWS_SUPABASE_JWKS_URL`, `NEWS_SUPABASE_JWT_ISSUER`, `NEWS_SUPABASE_JWT_AUDIENCE` et `NEWS_SUPABASE_MODERATOR_ROLES`
- creation du module `auth.rs`
- extraction du header `Authorization: Bearer <jwt>`
- verification locale du JWT via JWKS Supabase
- controle de l'algorithme asymetrique, du `kid`, de la signature, de `exp`, `sub`, `aud` et `iss`
- cache JWKS local borne a 10 minutes
- injection d'un `CurrentUser` dans les extensions `axum`
- resolution du role applicatif depuis la table locale `user_profiles` dans l'etat `NEWS-071`
- creation automatique d'un profil local `listener` pour une session Supabase valide inconnue dans l'etat `NEWS-071`
- ajout des erreurs HTTP `401 Unauthorized` et `403 Forbidden`
- ajout de `authorization` dans les headers CORS autorises

## Routes protegees

`/health` reste public pour les healthchecks.

Routes HTTP listener:

- `GET /api/sources`
- `GET /api/news`
- `GET /api/news/{id}`
- `GET /api/translations`
- `GET /api/calendar-day/today`

Routes HTTP moderator:

- `GET /api/runtime/translation`
- `GET /api/runtime/tts`

Les roles moderator acceptes par defaut sont:

- `moderator`
- `admin`

Ils peuvent etre remplaces par:

```env
NEWS_SUPABASE_MODERATOR_ROLES=moderator,admin
```

## Hors scope

Les WebSockets restent hors scope de ce ticket:

- `/ws/news`
- `/ws/audio`
- `/ws/test-runs`

Leur handshake auth explicite est livre separement dans `NEWS-070`.

Depuis `NEWS-070`, le client propage aussi le token sur les appels HTTP et WebSocket.

## Regle Supabase

Le middleware utilise le JWKS Supabase:

```text
https://project-ref.supabase.co/auth/v1/.well-known/jwks.json
```

Il attend donc des access tokens signes avec des cles asymetriques publiees dans ce JWKS.

Un projet Supabase encore configure avec l'ancien secret JWT partage `HS256` ne sera pas verifie par ce middleware JWKS. Dans ce cas, il faut migrer vers les signing keys asymetriques Supabase ou garder `NEWS_AUTH_ENABLED=false` tant que la configuration n'est pas prete.

## Contrat de role

Supabase Auth est uniquement la source de verite de l'identite (`sub`, `email`) et de la validite de session.

Dans l'etat livre par `NEWS-071`, le role applicatif Fil Finance est lu dans SQLite:

```sql
SELECT role FROM user_profiles WHERE supabase_user_id = '<supabase-sub>';
```

Dans la cible `EPIC-017`, la meme responsabilite passe a Supabase Postgres:

```sql
SELECT role FROM public.user_profiles WHERE supabase_user_id = '<supabase-sub>';
```

Regles:

- un utilisateur Supabase valide absent de `user_profiles` cree un profil `listener`
- les synchronisations suivantes mettent a jour l'email, mais ne modifient pas le role
- `moderator` et `admin` donnent acces aux surfaces runtime/test-run par defaut
- les claims Supabase `role`, `app_metadata` et `user_metadata` ne sont pas sources de droits applicatifs

## Variables serveur

Configuration minimale pour activer le middleware:

```env
NEWS_AUTH_ENABLED=true
NEWS_SUPABASE_PROJECT_URL=https://project-ref.supabase.co
NEWS_SUPABASE_JWKS_URL=https://project-ref.supabase.co/auth/v1/.well-known/jwks.json
NEWS_SUPABASE_JWT_ISSUER=https://project-ref.supabase.co/auth/v1
NEWS_SUPABASE_JWT_AUDIENCE=authenticated
NEWS_SUPABASE_MODERATOR_ROLES=moderator,admin
```

`NEWS_SUPABASE_JWKS_URL` et `NEWS_SUPABASE_JWT_ISSUER` peuvent etre derives depuis `NEWS_SUPABASE_PROJECT_URL`.

## Tests attendus

Validation locale:

```bash
cd filfinance/filfinance-service
cargo check
cargo test
```

Tests unitaires ajoutes:

- parsing du header `Authorization: Bearer`
- rejet d'un header absent
- conversion d'un profil local en `CurrentUser`
- controle des roles moderator configures

## References

- [Configuration Supabase - Fil Finance](../SUPABASE-CONFIG.md)
- [EPIC-014 - Authentification Supabase](../epics/EPIC-014-authentification-supabase.md)
- [NEWS-067 - Cadrage auth Supabase, roles et contrats de session](./NEWS-067-cadrage-auth-supabase-roles-session.md)
- [NEWS-068 - Client Supabase, login, logout et restauration de session](./NEWS-068-client-supabase-login-session.md)
- [NEWS-070 - Authentification des flux WebSocket Supabase](./NEWS-070-auth-websocket-supabase.md)
- [EPIC-017 - Stockage hybride profils Supabase Postgres](../epics/EPIC-017-stockage-hybride-profils-supabase-postgres.md)
