# NEWS-120 - Smoke profils et preferences theme

## Statut

Implementation livree le 3 mai 2026.

## Objectif

Etendre le smoke HTTP Supabase/Postgres pour couvrir la preference `theme` ajoutee aux profils utilisateur.

## Scope

- Verifier que `PATCH /api/me` persiste `preferred_tts_voice` et `theme`.
- Verifier que la reponse `PATCH /api/me` renvoie bien les deux preferences.
- Verifier qu'une valeur de theme inconnue est refusee en `400`.
- Documenter les variables de smoke utiles en local et en live.

## Hors scope

- Creation automatique de tokens Supabase.
- Smoke navigateur frontend.
- Gestion admin des roles depuis l'app.

## Implementation

- `filfinance-service/scripts/smoke_test_supabase_auth.sh` envoie maintenant `preferred_tts_voice` et `theme` dans le meme patch de profil.
- `FILFINANCE_AUTH_SMOKE_THEME` permet de choisir la valeur testee, avec `dark` comme defaut.
- Le script conserve le test moderator optionnel via `SUPABASE_MODERATOR_ACCESS_TOKEN`.
- `AUTH-RUNBOOK`, `SUPABASE-CONFIG` et `NEWS-092` documentent maintenant ce controle.

## Criteres d'acceptation

- Le smoke HTTP passe avec un token listener valide.
- Le smoke HTTP confirme `preferences.preferred_tts_voice`.
- Le smoke HTTP confirme `preferences.theme`.
- Le smoke HTTP refuse `theme="invalid-smoke-theme"` en `400`.
- Le smoke WebSocket role-based reste separe.

## Validation locale

- `bash -n filfinance-service/scripts/smoke_test_supabase_auth.sh`: OK.
