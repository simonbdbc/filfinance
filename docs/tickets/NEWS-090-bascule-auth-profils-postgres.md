# NEWS-090 - Bascule auth HTTP/WebSocket vers profils Postgres

## Objectif

Brancher les routes HTTP, `/api/me` et les handshakes WebSocket sur le store profil Supabase Postgres.

## Statut

Implemente cote repo le 2 mai 2026.

Les routes HTTP et les handshakes WebSocket utilisent maintenant le `UserProfileStore` configure. En mode `supabase_postgres`, les roles et preferences viennent de `public.user_profiles`.

## Scope

- `GET /api/me`
- `PATCH /api/me`
- resolution `CurrentUser` apres verification JWT
- middleware HTTP listener/moderator
- handshake `/ws/news`
- handshake `/ws/audio`
- handshake `/ws/test-runs`
- logs d'erreur distincts pour auth invalide, profil indisponible et role insuffisant

## Hors scope

- changement du contrat client `CurrentUserProfileResponse`
- migration des news vers Postgres
- creation d'une UI admin roles
- backfill historique des profils

## Approche technique

Conserver la sequence de securite actuelle:

1. verifier le JWT Supabase;
2. extraire l'identite (`sub`, `email`);
3. creer ou relire `public.user_profiles`;
4. construire `CurrentUser` depuis le profil;
5. appliquer le controle listener/moderator.

La source de droits devient `public.user_profiles.role` en cible hybride. Les claims JWT `role`, `app_metadata` et `user_metadata` ne doivent pas etre utilises comme source d'autorisation applicative.

## Criteres d'acceptation

- un token Supabase valide cree un profil Postgres `listener` si absent
- `GET /api/me` renvoie le profil Postgres courant
- `PATCH /api/me` persiste `preferred_tts_voice` et `theme` dans Postgres
- un `listener` accede aux flux news/audio
- un `listener` est refuse sur runtime et `/ws/test-runs`
- un `moderator` ou `admin` accede aux runtime et `/ws/test-runs`
- une panne Postgres profil produit une erreur exploitable, differente d'un JWT invalide

## Tests attendus

- tests HTTP `GET /api/me` et `PATCH /api/me`
- tests de role moderator sur route runtime
- tests unitaires de construction `CurrentUser`
- validation WebSocket manuelle listener/moderator
- smoke script mis a jour pour profils Postgres
- `cargo fmt --check`: OK
- `cargo check`: OK
- `cargo test`: OK

## Dependances

- [NEWS-089 - Repository backend hybride des profils](./NEWS-089-repository-backend-hybride-profils.md)
- [AUTH-RUNBOOK](../AUTH-RUNBOOK.md)
