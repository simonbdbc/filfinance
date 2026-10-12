# Migration Better Auth + PostgreSQL

## Decision

La cible d'authentification Fil Finance est Better Auth auto-heberge avec PostgreSQL:

- `filfinance-auth`: API Node.js/TypeScript pour `/api/auth/*` et `/api/me`;
- `filfinance-postgres`: schemas `auth` et `app`, roles SQL distincts;
- Expo/PWA: session Better Auth stockee via SecureStore, aucun acces direct a PostgreSQL;
- Rust: data plane news/audio encore interne, JWT Better Auth obligatoire sur ses routes protegees;
- aucun SDK ni runtime Supabase/Logto actif.

Le SQLite serveur ne porte plus les profils, roles ou preferences. Les migrations historiques `user_profiles` restent dans l'historique SQLite mais aucun runtime ne les lit.

## Etat des donnees

Audit serveur du 15 juillet 2026:

- prod: `user_profiles=0`;
- dev: `user_profiles=0`.

Il n'y a donc aucun profil SQLite a transformer. Les comptes du fournisseur d'identite precedent doivent toutefois etre exportes avant sa fermeture. Les sessions ne sont pas migrables; une nouvelle connexion est obligatoire. Si les hashes de mots de passe ne sont pas exportables, utiliser le parcours de reinitialisation Better Auth.

## Migrations SQL

- [0001_better_auth.sql](../filfinance-auth/db/auth-migrations/0001_better_auth.sql): tables Better Auth et plugin Admin/JWT.
- [0001_app_foundation.sql](../filfinance-auth/db/app-migrations/0001_app_foundation.sql): profils, correspondance d'identites et bootstrap admin.
- [0002_profile_preferences_and_sync.sql](../filfinance-auth/db/app-migrations/0002_profile_preferences_and_sync.sql): preferences et journal de synchronisation.

Le service applique ces migrations au demarrage sous deux verrous PostgreSQL distincts. Le role `filfinance_auth` ne peut pas modifier `app`; le role `filfinance_app` ne peut pas modifier `auth`.

## Preparation commune

1. Exporter les utilisateurs et roles de l'ancien fournisseur.
2. Sauvegarder les volumes et bases existants.
3. Creer les fichiers serveur a partir de `.env.server.example` sans reutiliser un secret entre dev et prod.
4. Configurer un SMTP reel et tester verification d'email et reset.
5. Donner au compte de deploiement un acces lecture/ecriture limite aux fichiers d'environnement, par ACL ou groupe dedie.
6. Executer `scripts/check_better_auth_env.sh` avant d'activer le deploiement.

Exemple d'ACL a faire appliquer par l'administrateur serveur:

```bash
sudo setfacl -m u:gitlab-ci:rw /raid/app/filfinance/filfinance-dev/.env.server.dev
sudo setfacl -m u:gitlab-ci:rw /raid/app/filfinance/filfinance-prod/.env.server.prod
```

## Bascule dev

1. Mettre `FILFINANCE_API_HOST=api.dev.filfinance.fr` et aligner toutes les URLs Better Auth/JWT sur `https://api.dev.filfinance.fr`.
2. Utiliser des mots de passe PostgreSQL et un `BETTER_AUTH_SECRET` propres au dev. Configurer un SMTP dev reel et fixer `BETTER_AUTH_REQUIRE_EMAIL_VERIFICATION=true`; le preflight refuse desormais de desactiver cette verification en dev comme en prod.
3. Executer:

```bash
bash scripts/migrate_server_env.sh .env.server.dev
: "${FILFINANCE_DEV_PWA_URL:?Renseigner l'URL PWA dev reelle}"
bash scripts/check_better_auth_env.sh \
  .env.server.dev dev "$FILFINANCE_DEV_PWA_URL"
docker compose --env-file .env.server.dev -f compose.yaml -f compose.prod.yaml config >/dev/null
```

Le port d'origine doit appartenir a `filfinance-gateway`: `/api/auth/*` et `/api/me`
sont routes vers Better Auth, tandis que `/health`, les API metier, HLS et WebSocket
restent routes vers Rust.

4. Activer la variable GitLab protegee `FILFINANCE_BETTER_AUTH_DEPLOY_ENABLED=true`.
5. Pousser `dev`, attendre `auth:check`, `auth:integration` et `deploy:dev:backend`.
6. Tester inscription, verification email, login, restauration, reset, logout, `/api/me`, HTTP et WebSocket.
7. Promouvoir un compte de test, renouveler son JWT et verifier les routes moderator/admin.
8. Observer au moins 24 heures les erreurs auth, SMTP, migrations et JWKS.

## Bascule prod

1. Faire un `pg_dump` de toute donnee source encore utile et une sauvegarde du volume SQLite/Rust.
2. Configurer `.env.server.prod` avec des secrets distincts et `BETTER_AUTH_REQUIRE_EMAIL_VERIFICATION=true`.
3. Executer preflight et `docker compose config` sans demarrer les services.
4. Importer les comptes dans `auth` uniquement via un script audite ou le parcours reset; enregistrer les correspondances dans `app.auth_identity_migration`.
5. Verifier les totaux `source`, `migrated`, `failed` et les emails normalises.
6. Definir `ADMIN_EMAIL_TO_PROMOTE`, creer/verifier ce compte, confirmer `app.auth_bootstrap`, puis retirer la variable. Le bootstrap est retente apres les flux Better Auth jusqu'a sa reussite; aucun redemarrage n'est requis.
7. Activer le deploiement Better Auth et pousser `main`.
8. Executer les smokes publics, authentifies et WebSocket du [runbook auth](./AUTH-RUNBOOK.md).
9. Forcer la reconnexion des clients et surveiller 48 heures.
10. Desactiver l'ancien fournisseur seulement apres validation des exports et du rollback.

## Rollback

Le rollback applicatif ne doit jamais supprimer le volume PostgreSQL:

1. desactiver `FILFINANCE_BETTER_AUTH_DEPLOY_ENABLED`;
2. remettre le frontend au build precedent;
3. redeployer le commit stable du backend;
4. conserver `filfinance_postgres_data_*` et exporter les comptes crees pendant la fenetre;
5. retablir temporairement l'ancien fournisseur uniquement si ses donnees et sa configuration ont ete conservees;
6. documenter les identites creees des deux cotes avant une nouvelle tentative.

## Go / No-Go

Go si CI, preflight, migrations, SMTP, JWT/JWKS, profils, roles, HTTP et WebSocket passent en dev puis prod.

No-Go si le fichier serveur est illisible, un placeholder subsiste, un export utilisateur manque, le reset email echoue, les URLs issuer/audience divergent ou le rollback n'a pas ete repete.
