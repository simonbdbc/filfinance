# EPIC-014 - Authentification Supabase

> **Archive historique — ne pas executer.** Supabase Auth est supersede par
> Better Auth + PostgreSQL. Utiliser [le runbook Better Auth](../AUTH-RUNBOOK.md).

## Objectif

Introduire une authentification utilisateur basee sur `Supabase Auth` pour proteger `Fil Finance`, les endpoints HTTP et les flux WebSocket, sans imposer dans ce meme chantier une migration du stockage news hors de SQLite.

## Valeur livree

Le projet passe d'un MVP entierement ouvert a une application avec identite utilisateur, session cliente, verification serveur et surfaces d'administration mieux delimitees. Cela prepare un usage prive, un partage d'acces par role, et des preferences utilisateur persistables proprement.

## Statut

Fait cote repo au 1 mai 2026 et valide en live le 2 mai 2026.

Note du 2 mai 2026: `EPIC-017` prend le relais pour l'architecture hybride. `EPIC-014` reste le socle auth/session/JWT; les profils, roles et preferences doivent ensuite etre deplaces de SQLite vers Supabase Postgres.

## Avancement au 1 mai 2026

- `NEWS-067` est implemente: roles, surfaces publiques/privees et contrat WebSocket auth sont cadres.
- `NEWS-068` est implemente cote client: dependances Supabase, provider de session, gate de login/creation de compte, restauration de session et logout sont branches dans `filfinance-client`.
- `NEWS-069` est implemente cote backend HTTP: config auth, verification JWT Supabase via JWKS, roles applicatifs lus dans `user_profiles`, routes listener/moderator et erreurs `401/403`.
- `NEWS-070` est implemente: le client propage le token sur HTTP/WebSocket, les sockets attendent un message `auth` avant tout `hello`, et `/ws/test-runs` exige un role moderator/admin.
- `NEWS-071` est implemente: profils locaux `user_profiles`, `GET/PATCH /api/me`, sync identite sur HTTP/WebSocket, role local et preference `preferred_tts_voice` persistante.
- `NEWS-072` est implemente cote repo: smoke test HTTP, runbook d'exploitation, matrice go/no-go, rotation et diagnostic incidents.
- `EPIC-017` est cadre: la cible devient Supabase Postgres pour profils/roles/preferences, avec SQLite conserve pour le pipeline news/audio.
- La configuration locale et cible est documentee dans [SUPABASE-CONFIG](../SUPABASE-CONFIG.md).
- La validation avec un vrai projet Supabase est passee en live le 2 mai 2026.
- Les erreurs runtime profil/historique detectees pendant la validation live sont resolues.
- Le client conserve maintenant les messages d'erreur backend detailles pour faciliter le diagnostic des prochains incidents HTTP.

## Audit du code actuel au 27 avril 2026

Constats backend avant `NEWS-069`:

- toutes les routes HTTP et WebSocket sont aujourd'hui publiques dans [filfinance-service/src/main.rs](../../filfinance-service/src/main.rs)
  - `/health`
  - `/api/runtime/translation`
  - `/api/runtime/tts`
  - `/api/sources`
  - `/api/news`
  - `/api/translations`
  - `/api/calendar-day/today`
  - `/api/news/{id}`
  - `/ws/news`
  - `/ws/audio`
  - `/ws/test-runs`
- il n'existe aucun middleware JWT, aucune extraction `current_user`, aucun contexte d'autorisation
- le CORS HTTP est aujourd'hui ouvert en `*` dans [filfinance-service/src/main.rs](../../filfinance-service/src/main.rs) parce que le MVP ne transporte pas encore de session applicative
- [Cargo.toml](../../filfinance-service/Cargo.toml) ne declare aucune dependance dediee a la verification de JWT Supabase
- [filfinance-service/src/storage.rs](../../filfinance-service/src/storage.rs) ne contient aujourd'hui que des tables metier `news_events`, `news_translations`, `calendar_day_*` et aucune table utilisateur/profil/role/audit d'acces

Constats backend apres `NEWS-069`:

- [filfinance-service/src/auth.rs](../../filfinance-service/src/auth.rs) verifie les JWT Supabase via JWKS, resout le profil local et injecte `CurrentUser`
- [filfinance-service/src/config.rs](../../filfinance-service/src/config.rs) expose `AuthConfig`
- [filfinance-service/src/main.rs](../../filfinance-service/src/main.rs) separe les routes HTTP publiques, listener et moderator
- `GET /health` reste public
- les routes HTTP metier exigent un Bearer token quand `NEWS_AUTH_ENABLED=true`
- les routes HTTP runtime exigent un role `moderator` ou `admin`

Constats backend apres `NEWS-070`:

- [filfinance-service/src/websocket.rs](../../filfinance-service/src/websocket.rs) exige un message `auth` avant tout `hello` quand `NEWS_AUTH_ENABLED=true`
- `/ws/news` et `/ws/audio` acceptent une session listener
- `/ws/test-runs` exige un role `moderator` ou `admin`
- le socket est ferme en policy `1008` en cas d'auth absente, invalide ou insuffisante

Constats backend apres `NEWS-071`:

- [filfinance-service/src/storage.rs](../../filfinance-service/src/storage.rs) cree `user_profiles`
- l'identite Supabase courante synchronise `supabase_user_id` et `email` sans ecraser le role ni les preferences locales
- [filfinance-service/src/api.rs](../../filfinance-service/src/api.rs) expose `GET /api/me` et `PATCH /api/me`
- [filfinance-service/src/auth.rs](../../filfinance-service/src/auth.rs) synchronise le profil apres authentification HTTP
- [filfinance-service/src/websocket.rs](../../filfinance-service/src/websocket.rs) synchronise aussi le profil pendant le handshake WebSocket

Constats frontend avant `NEWS-068`:

- [apps/pwa/package.json](../../apps/pwa/package.json) ne declarait aucune dependance `supabase-js`, aucun stockage securise et aucun provider de session
- [apps/pwa/App.tsx](../../apps/pwa/App.tsx) ne contenait ni ecran de login, ni garde d'acces, ni bootstrap de session, ni logout
- le client ouvre directement les flux `/ws/news`, `/ws/audio` et `/ws/test-runs` sans identite utilisateur
- il n'existe aucune distinction entre utilisateur lecteur et operateur capable de lancer des `test_run`

Constats frontend apres `NEWS-068`:

- le client declare `@supabase/supabase-js`, `@react-native-async-storage/async-storage` et `react-native-url-polyfill`
- `AuthProvider` restaure la session Supabase et expose le token courant
- `AuthGate` bloque le montage de l'app tant qu'aucune session n'est active et gere la creation de compte
- `AuthSessionCard` permet de se deconnecter depuis l'ecran principal

Constats frontend apres `NEWS-070`:

- le token Supabase est envoye sur les fetchs HTTP proteges via `Authorization: Bearer`
- le token Supabase est envoye sur `/ws/news`, `/ws/audio` et `/ws/test-runs` via le premier message `auth`
- le client reconnecte les sockets quand le token courant change

Constats frontend apres `NEWS-071`:

- le client charge `GET /api/me` apres restauration de session
- la preference `preferred_tts_voice` est transmise au coordinateur audio
- un changement manuel de voix TTS est persiste via `PATCH /api/me`
- les erreurs de profil/preferences sont visibles dans le resume d'erreurs systeme

Constats exploitation apres `NEWS-072`:

- [AUTH-RUNBOOK](../AUTH-RUNBOOK.md) documente la configuration, la recuperation de tokens, les checks HTTP/WebSocket, les incidents et la rotation
- [filfinance-service/scripts/smoke_test_supabase_auth.sh](../../filfinance-service/scripts/smoke_test_supabase_auth.sh) verifie les surfaces HTTP critiques avec tokens listener/moderator
- la validation WebSocket reste volontairement manuelle pour couvrir le vrai comportement navigateur du handshake

## Choix d'architecture recommande

Le choix recommande pour cet EPIC est:

- `Supabase Auth` comme fournisseur d'identite
- backend Rust qui verifie les JWT Supabase
- stockage metier `Fil Finance` qui reste en SQLite dans ce lot
- stockage cible des profils, roles et preferences dans Supabase Postgres via `EPIC-017`
- separation claire entre:
  - authentification
  - autorisation
  - profil utilisateur
  - persistance metier des news

Autrement dit:

- oui a `Supabase` pour l'identite et la session
- oui a Supabase Postgres pour profils, roles et preferences dans le chantier suivant
- non a une migration immediate de `news_events` et `news_translations` vers Postgres Supabase dans ce meme EPIC

## Cible fonctionnelle

Le parcours cible minimal:

- un utilisateur se connecte dans le client via Supabase
- le client conserve sa session et peut la restaurer au redemarrage
- le client propage un jeton d'acces au backend
- le backend verifie ce jeton avant d'exposer les routes protegees
- les flux WebSocket n'envoient aucune donnee tant que l'auth n'est pas validee
- les capacites sensibles sont differenciees:
  - lecture des news
  - acces aux endpoints runtime
  - declenchement des tests E2E

## Strategie recommande

### Etape 1 - identite et session cliente

- ajouter le client Supabase dans l'app `filfinance`
- brancher login, logout et restauration de session
- introduire un `AuthProvider` et un ecran ou gate de login

### Etape 2 - verification serveur

- ajouter une couche backend de verification JWT Supabase
- injecter un contexte utilisateur minimal dans les handlers HTTP
- refuser l'acces aux routes protegees sans token valide

### Etape 3 - WebSocket auth

- ajouter une authentification explicite pour `/ws/news`, `/ws/audio` et `/ws/test-runs`
- le choix le plus robuste ici est de ne rien diffuser avant validation d'un jeton au debut du canal
- il faut traiter a part la contrainte navigateur: contrairement a `fetch`, le browser ne permet pas un header `Authorization` arbitraire classique sur `WebSocket`

### Etape 4 - roles et surfaces sensibles

- distinguer au minimum:
  - `authenticated_user`
  - `moderator` ou `admin`
- reserver `test_run` et les endpoints runtime aux roles eleves

### Etape 5 - profils et preferences

- ajouter un profil utilisateur minimal
- preparer la persistence de preferences comme la voix TTS ou certains reglages client
- statut: livre par `NEWS-071` pour `preferred_tts_voice`

## Scope

Dans cet EPIC:

- oui a `Supabase Auth`
- oui a la verification JWT cote Rust
- oui a la protection des routes HTTP et WebSocket
- oui a une notion minimale de role
- oui a un profil utilisateur minimal
- non a une migration complete du stockage metier vers Supabase Postgres
- non a une redefinition produit des flux news/audio

## Definition of Done

- le client impose une authentification Supabase avant acces normal a l'application
- le backend verifie les JWT Supabase pour les routes protegees
- les routes et flux sensibles ne sont plus publics
- les WebSockets n'emettront pas de donnees sans auth validee
- le lancement de `test_run` est reserve a un role explicite
- le modele utilisateur minimal et les preferences de base sont poses
- la doc de setup locale Supabase est ecrite
- le runbook d'exploitation auth et le smoke test HTTP sont disponibles

## Tickets lies

- `NEWS-067` Cadrage auth Supabase, roles et contrats de session
- `NEWS-068` Client Supabase, login, logout et restauration de session
- `NEWS-069` Middleware backend de verification JWT et contexte utilisateur
- `NEWS-070` Authentification des flux WebSocket et protection du canal test-runs
- `NEWS-071` Profils utilisateur, preferences minimales et persistence associee
- `NEWS-072` Tests, runbook local et documentation d'exploitation auth
- `EPIC-017` Stockage hybride profils Supabase Postgres

## Points d'attention

- l'EPIC doit expliciter quels endpoints restent publics et lesquels deviennent prives
- il faut trancher la forme d'auth WebSocket assez tot pour ne pas faire un patch opportuniste ensuite
- l'introduction de l'auth peut s'appuyer sur `EPIC-013`, maintenant clos cote client
- cet EPIC doit rester borne a l'auth; la bascule profils/roles/preferences vers Supabase Postgres est portee par `EPIC-017`
