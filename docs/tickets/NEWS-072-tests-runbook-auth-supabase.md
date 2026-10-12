# NEWS-072 - Tests et runbook d'exploitation auth Supabase

> **Archive historique — ne pas executer.** Le smoke Supabase decrit ici est
> supersede par la suite Better Auth actuelle. Utiliser
> [le runbook Better Auth](../AUTH-RUNBOOK.md).

## Objectif

Finaliser `EPIC-014` avec un runbook exploitable et un smoke test local pour verifier l'authentification Supabase sur les surfaces HTTP critiques.

## Statut

Implemente cote repo le 1 mai 2026.

La validation avec un vrai projet Supabase demande des tokens reels et doit etre executee dans l'environnement cible avec [AUTH-RUNBOOK](../AUTH-RUNBOOK.md).

## Scope livre

- ajout du runbook [AUTH-RUNBOOK](../AUTH-RUNBOOK.md)
- ajout du script `filfinance-service/scripts/smoke_test_supabase_auth.sh`
- documentation de recuperation d'access tokens sans `service_role key`
- matrice go/no-go HTTP et WebSocket
- diagnostic des incidents `401`, `403`, JWKS, WebSocket `1008` et persistence profil
- procedure de rotation publishable key, signing keys JWT et roles applicatifs
- mise a jour de l'EPIC, de la roadmap et de la config Supabase

## Smoke test HTTP

Commande:

```bash
cd filfinance/filfinance-service
FILFINANCE_API_BASE_URL="http://127.0.0.1:8088" \
SUPABASE_LISTENER_ACCESS_TOKEN="${SUPABASE_LISTENER_ACCESS_TOKEN}" \
SUPABASE_MODERATOR_ACCESS_TOKEN="${SUPABASE_MODERATOR_ACCESS_TOKEN:-}" \
bash scripts/smoke_test_supabase_auth.sh
```

Surfaces couvertes:

- `/health`
- `/api/me`
- `/api/runtime/tts`
- persistence de `preferred_tts_voice`

## Validation WebSocket

Le runbook documente la validation manuelle des trois flux:

- `/ws/news`
- `/ws/audio`
- `/ws/test-runs`

Le comportement attendu reste:

- aucune donnee avant auth validee
- fermeture `1008` sans token ou role insuffisant
- `hello` comme premier message apres auth acceptee

## Validations locales executees

- `bash -n filfinance-service/scripts/smoke_test_supabase_auth.sh`: OK
- `cargo fmt --check`: OK
- `cargo check`: OK
- `cargo test`: OK
- `./node_modules/.bin/tsc --noEmit --pretty false`: OK

## Limite assumee

Le smoke test n'a pas ete execute contre Supabase depuis le repo, car il necessite des credentials et des tokens utilisateur reels. Le runbook donne la procedure exacte pour l'executer sans exposer de secret serveur.

## References

- [AUTH-RUNBOOK](../AUTH-RUNBOOK.md)
- [SUPABASE-CONFIG](../SUPABASE-CONFIG.md)
- [EPIC-014 - Authentification Supabase](../epics/EPIC-014-authentification-supabase.md)
- [NEWS-071 - Profils utilisateur et preferences minimales](./NEWS-071-profils-utilisateur-preferences.md)
