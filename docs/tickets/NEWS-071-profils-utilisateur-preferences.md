# NEWS-071 - Profils utilisateur et preferences minimales

## Objectif

Ajouter un profil utilisateur local lie a `Supabase Auth` et persister une premiere preference applicative: la voix TTS preferee.

## Statut

Implemente le 1 mai 2026.

Note du 2 mai 2026: ce ticket a livre le premier store profil en SQLite. La cible hybride `EPIC-017` deplace ensuite profils, roles et preferences vers Supabase Postgres, en gardant SQLite pour le pipeline news/audio.

## Scope livre

- creation de la table SQLite `user_profiles`
- synchronisation du profil local a chaque authentification HTTP protegee
- synchronisation du profil local au handshake WebSocket authentifie
- endpoint `GET /api/me`
- endpoint `PATCH /api/me`
- contrat client `CurrentUserProfileResponse`
- API client `loadCurrentUserProfile`
- API client `updateUserPreferences`
- chargement de la preference `preferred_tts_voice` au montage de l'app
- sauvegarde de la voix TTS choisie depuis le selecteur audio
- affichage d'une erreur dediee si le profil ou les preferences ne peuvent pas etre charges

## Modele persiste

Table locale:

```sql
CREATE TABLE IF NOT EXISTS user_profiles (
    supabase_user_id TEXT PRIMARY KEY,
    email TEXT,
    role TEXT NOT NULL,
    preferred_tts_voice TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

`Supabase Auth` reste la source de verite pour l'identite. Dans ce ticket, SQLite devient la source de verite transitoire pour le role applicatif `Fil Finance` et les preferences utilisateur.

La cible suivante est Supabase Postgres:

```sql
public.user_profiles (
    supabase_user_id uuid PRIMARY KEY REFERENCES auth.users(id),
    email text,
    role text NOT NULL DEFAULT 'listener',
    preferred_tts_voice text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
)
```

## Contrat HTTP

Lecture du profil courant:

```http
GET /api/me
Authorization: Bearer <access_token>
```

Reponse:

```json
{
  "id": "supabase-user-id",
  "email": "listener@example.test",
  "role": "listener",
  "preferences": {
    "preferred_tts_voice": "fr_FR-upmc-medium-jessica"
  },
  "created_at": "2026-05-01T10:00:00Z",
  "updated_at": "2026-05-01T10:00:00Z"
}
```

Mise a jour partielle des preferences:

```http
PATCH /api/me
Authorization: Bearer <access_token>
Content-Type: application/json
```

```json
{
  "preferred_tts_voice": "fr_FR-upmc-medium-jessica"
}
```

Regles de normalisation:

- champ absent: aucune modification
- `null`: suppression de la preference
- chaine vide ou espaces seuls: suppression de la preference
- chaine non vide: trim puis persistence
- longueur maximale: 128 caracteres

## Comportement client

Au montage de `FilFinanceApp`, le client charge `GET /api/me` avec le token Supabase courant.

La preference `preferred_tts_voice` devient l'entree prioritaire pour le choix de voix audio serveur. Quand l'utilisateur change la voix depuis l'UI audio, le client envoie `PATCH /api/me` pour rendre ce choix durable entre deux sessions.

## Tests ajoutes

- storage: l'upsert d'identite ne doit pas ecraser `preferred_tts_voice`
- storage: l'upsert d'identite ne doit pas ecraser le role local
- API: `GET /api/me` cree le profil local et `PATCH /api/me` persiste la voix trimmee

## Validations locales

```bash
cd filfinance/filfinance-service
cargo fmt --check
cargo check
cargo test

cd ../filfinance-client
./node_modules/.bin/tsc --noEmit --pretty false
```

Validations executees le 1 mai 2026:

- `cargo fmt --check`: OK
- `cargo check`: OK
- `cargo test`: OK, 71 tests apres bascule des roles vers `user_profiles`
- `./node_modules/.bin/tsc --noEmit --pretty false`: OK

## Limites restantes

- le test live avec un vrai projet Supabase doit etre execute dans l'environnement cible avec des tokens reels
- le runbook et le smoke test associes sont livres dans `NEWS-072`

## References

- [EPIC-014 - Authentification Supabase](../epics/EPIC-014-authentification-supabase.md)
- [SUPABASE-CONFIG](../SUPABASE-CONFIG.md)
- [NEWS-069 - Middleware Rust de verification JWT Supabase](./NEWS-069-middleware-rust-jwt-supabase.md)
- [NEWS-070 - Authentification des flux WebSocket Supabase](./NEWS-070-auth-websocket-supabase.md)
- [NEWS-072 - Tests et runbook d'exploitation auth Supabase](./NEWS-072-tests-runbook-auth-supabase.md)
- [EPIC-017 - Stockage hybride profils Supabase Postgres](../epics/EPIC-017-stockage-hybride-profils-supabase-postgres.md)
