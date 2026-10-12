# EPIC-017 - Stockage hybride profils Supabase Postgres

> **Archive historique — ne pas executer.** Supabase et ce stockage hybride sont
> supersedes par Better Auth + PostgreSQL. Utiliser
> [le runbook Better Auth](../AUTH-RUNBOOK.md).

## Objectif

Basculer vers une architecture hybride:

- `Supabase Auth` pour l'identite et la session.
- `Supabase Postgres` pour les profils, roles et preferences utilisateur.
- `SQLite` local pour le pipeline news/audio.

Cette decision remplace la cible locale de `NEWS-071` pour les profils, sans migrer les tables news vers Postgres.

## Valeur livree

Les roles et preferences deviennent partageables entre environnements et instances backend. Un utilisateur garde son role et ses reglages meme si la base SQLite locale de pipeline est reconstruite, deplacee ou reservee au debug.

Le pipeline news/audio conserve son stockage local, ce qui evite d'ajouter une dependance reseau au chemin critique ingestion -> traduction -> TTS -> WebSocket.

## Statut

Cadrage ajoute le 2 mai 2026.

Priorite 2 au 2 mai 2026, puis go/no-go live au 3 mai 2026.

`NEWS-089`, `NEWS-090` et `NEWS-091` sont implementes cote repo: le backend dispose d'un `UserProfileStore` configurable, peut lire/ecrire les roles/preferences dans `public.user_profiles` Supabase Postgres avec `NEWS_PROFILE_STORE=supabase_postgres`, et fournit une commande de backfill SQLite -> Supabase Postgres. `NEWS-119` ajoute la preference `theme` au meme modele.

`NEWS-091` est valide live sur Supabase Session pooler avec `total_read=1`, `imported=1` et `errors=0`.

Le fallback `NEWS_PROFILE_STORE=sqlite` reste disponible pour les tests et le developpement local. `NEWS-092` est implemente cote repo avec runbook hybride, smoke HTTP et smoke WebSocket; `NEWS-120` ajoute la verification live de la preference `theme` dans le smoke HTTP. Il reste a executer les smokes live avec tokens listener/moderator au moment du go/no-go.

## Decision d'architecture

Source de verite:

- identite: `Supabase Auth`
- role applicatif: `public.user_profiles.role` dans Supabase Postgres
- preferences utilisateur: `public.user_profiles.*` dans Supabase Postgres
- news, traductions, calendrier, replay, debug audio: SQLite local

Le client continue de passer par le backend Rust:

- `GET /api/me`
- `PATCH /api/me`

Aucune `service_role key` ne doit etre exposee au client Expo.

## Scope

Dans cet EPIC:

- ajouter le schema Supabase Postgres `public.user_profiles`
- abstraire le repository de profils dans le backend Rust
- brancher `/api/me`, `auth.rs` et `websocket.rs` sur le store profil Postgres
- conserver SQLite pour `news_events`, `news_translations` et `calendar_day_*`
- fournir une migration/backfill des profils locaux existants vers Supabase Postgres
- mettre a jour runbooks, smoke tests et documentation d'exploitation

Hors scope:

- migration des news vers Supabase Postgres
- acces direct client Expo aux tables Postgres
- gestion admin complete des utilisateurs depuis l'app
- audit trail avance des changements de role

## Tickets de dev

### [`NEWS-088` Schema Supabase Postgres des profils](../tickets/NEWS-088-stockage-hybride-profils-supabase-postgres.md)

Objectif: livrer le schema SQL cible `public.user_profiles` et la documentation d'application dans Supabase.

Scope:

- table `public.user_profiles`
- reference `auth.users(id)`
- role par defaut `listener`
- contrainte de roles `listener`, `moderator`, `admin`
- preference `preferred_tts_voice`
- preference `theme`
- trigger `updated_at`
- RLS activee sans policy publique au premier lot

Criteres d'acceptation:

- le SQL est documente dans `docs/sql/supabase`
- le bootstrap moderator est documente
- la doc explique que SQLite reste le stockage news/audio

### [`NEWS-089` Repository backend hybride des profils](../tickets/NEWS-089-repository-backend-hybride-profils.md)

Objectif: introduire une couche de persistence profils distincte de `NewsStorage`.

Scope:

- config serveur pour la connexion Supabase Postgres privee
- trait ou facade de repository profil
- implementation Postgres cible
- conservation transitoire de l'implementation SQLite pour dev/fallback si necessaire
- erreurs explicites quand le store profil est indisponible

Criteres d'acceptation:

- `NewsStorage` ne porte plus la responsabilite metier long terme des profils
- le pipeline news/audio n'a pas de dependance Postgres
- les tests couvrent l'upsert identite sans ecraser role/preferences

### [`NEWS-090` Bascule auth HTTP/WebSocket vers profils Postgres](../tickets/NEWS-090-bascule-auth-profils-postgres.md)

Objectif: faire lire les roles et preferences depuis Supabase Postgres pendant l'auth runtime.

Scope:

- `GET /api/me`
- `PATCH /api/me`
- middleware HTTP listener/moderator
- handshake `/ws/news`, `/ws/audio`, `/ws/test-runs`
- refus explicite si profil indisponible ou role insuffisant

Criteres d'acceptation:

- un utilisateur Supabase inconnu cree un profil Postgres `listener`
- un role `moderator` Postgres donne acces aux runtime/test-runs
- un role `listener` reste refuse sur les surfaces moderator
- aucune lecture de role depuis le JWT

### [`NEWS-091` Migration/backfill des profils SQLite vers Supabase Postgres](../tickets/NEWS-091-backfill-profils-sqlite-vers-supabase-postgres.md)

Objectif: deplacer les profils existants sans perdre roles ni preferences.

Scope:

- script ou commande de backfill depuis SQLite vers Postgres
- strategie d'idempotence sur `supabase_user_id`
- preservation de `role`, `preferred_tts_voice` et `theme`
- rapport des lignes importees, ignorees et en erreur

Criteres d'acceptation:

- relancer le backfill ne duplique pas les profils: OK
- un role local `moderator` reste `moderator` dans Postgres: OK
- les emails sont rafraichis sans casser les preferences: OK
- un role Postgres plus privilegie n'est pas degrade par un ancien SQLite: OK

### [`NEWS-092` Tests et runbook hybride](../tickets/NEWS-092-tests-runbook-stockage-hybride.md)

Objectif: verrouiller la validation du modele hybride.

Scope:

- smoke test HTTP avec profil Postgres
- validation WebSocket listener/moderator
- documentation incidents Postgres
- checklist de rollback vers fallback local si retenu

Criteres d'acceptation:

- le runbook distingue clairement panne Supabase Auth, panne Postgres profil et panne SQLite pipeline: OK
- le smoke WebSocket verifie que `/ws/test-runs` refuse un listener: OK cote script
- la doc indique comment promouvoir un utilisateur en `moderator` dans Supabase Postgres: OK

### [`NEWS-120` Smoke profils et preferences theme](../tickets/NEWS-120-smoke-profils-preferences-theme.md)

Objectif: couvrir `preferences.theme` dans le smoke HTTP Supabase/Postgres.

Scope:

- patch `preferred_tts_voice` plus `theme`
- verification du retour JSON
- refus d'un theme inconnu en `400`

Criteres d'acceptation:

- le script HTTP verifie `theme` avec un token listener: OK cote script
- le script conserve le test moderator optionnel: OK
- le runbook auth documente les variables de smoke: OK

## Definition of Done

- les profils, roles et preferences sont stockes dans Supabase Postgres
- SQLite ne stocke plus que le pipeline news/audio, hors fallback transitoire explicite
- les routes HTTP et WebSocket continuent de verifier Supabase Auth avant toute donnee metier
- les roles viennent de `public.user_profiles.role`, jamais du JWT
- les preferences utilisateur survivent a une reconstruction de la base SQLite locale
- les runbooks et smoke tests couvrent l'architecture hybride

## References

- [SUPABASE-CONFIG](../SUPABASE-CONFIG.md)
- [AUTH-RUNBOOK](../AUTH-RUNBOOK.md)
- [SQL Supabase Postgres](../sql/supabase/README.md)
- [EPIC-014 - Authentification Supabase](./EPIC-014-authentification-supabase.md)
- [NEWS-071 - Profils utilisateur et preferences minimales](../tickets/NEWS-071-profils-utilisateur-preferences.md)
