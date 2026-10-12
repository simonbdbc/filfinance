# NEWS-070 - Authentification des flux WebSocket Supabase

## Objectif

Proteger les flux WebSocket `Fil Finance` avec le meme access token `Supabase Auth` que les routes HTTP, sans envoyer de donnee metier avant validation serveur.

## Statut

Implemente le 1 mai 2026.

## Scope livre

- handshake WebSocket `{ "type": "auth", "access_token": "<token>" }`
- delai serveur de 5 secondes pour recevoir le message d'auth
- fermeture du socket en code policy `1008` si le token est absent, invalide ou insuffisant
- mutualisation de la verification JWT avec `NEWS-069`
- aucun `hello`, replay, item live, chunk audio ou trace de test avant auth validee
- `/ws/news` protege en role listener
- `/ws/audio` protege en role listener
- `/ws/test-runs` protege en role `moderator` ou `admin`
- tolerance d'un message `auth` tardif quand l'auth serveur est desactivee en local
- propagation du token client sur:
  - `/ws/news`
  - `/ws/audio`
  - `/ws/test-runs`
  - `GET /api/translations`

## Contrat WebSocket

Sequence client:

1. ouvrir le WebSocket
2. envoyer immediatement:

```json
{
  "type": "auth",
  "access_token": "<supabase_access_token>"
}
```

3. attendre le premier message serveur

Le serveur n'envoie pas d'accuse dedie. Le premier `hello` du flux signifie que le token a ete accepte.

## Surfaces protegees

Listener:

- `GET /ws/news`
- `GET /ws/audio`

Moderator:

- `GET /ws/test-runs`

`moderator` et `admin` sont les roles privilegies par defaut, configurables via:

```env
NEWS_SUPABASE_MODERATOR_ROLES=moderator,admin
```

## Comportement local

Si `NEWS_AUTH_ENABLED=false`, les WebSockets restent utilisables sans handshake obligatoire.

Le client peut quand meme envoyer `type=auth`; le serveur le tolere pour eviter une divergence de protocole entre dev et environnement protege.

## Tests ajoutes

- parsing du message auth WebSocket
- rejet d'un token WebSocket vide

Validation locale:

```bash
cd filfinance/filfinance-service
cargo fmt --check
cargo check
cargo test

cd ../filfinance-client
./node_modules/.bin/tsc --noEmit --pretty false
```

## Limites restantes

- pas encore de test E2E avec un vrai projet Supabase et un vrai JWT

Le profil utilisateur applicatif et les preferences minimales sont livres dans `NEWS-071`. Le runbook de validation avec vrai projet Supabase est livre dans `NEWS-072`.

## References

- [NEWS-069 - Middleware Rust de verification JWT Supabase](./NEWS-069-middleware-rust-jwt-supabase.md)
- [NEWS-071 - Profils utilisateur et preferences minimales](./NEWS-071-profils-utilisateur-preferences.md)
- [NEWS-072 - Tests et runbook d'exploitation auth Supabase](./NEWS-072-tests-runbook-auth-supabase.md)
- [SUPABASE-CONFIG](../SUPABASE-CONFIG.md)
- [EPIC-014 - Authentification Supabase](../epics/EPIC-014-authentification-supabase.md)
