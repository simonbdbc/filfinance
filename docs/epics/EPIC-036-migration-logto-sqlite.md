# EPIC-036 - Migration Logto + SQLite

> **Archive historique — ne pas executer.** Cette cible Logto + SQLite a ete
> abandonnee au profit de Better Auth + PostgreSQL. Utiliser
> [le runbook Better Auth actuel](../AUTH-RUNBOOK.md) et
> [le runbook GitLab actuel](../GITLAB-RUNBOOK.md).

## Objectif

Sortir l'application de l'ancien fournisseur auth et du stockage profil externe en basculant vers:

- Logto Cloud pour l'identite et les JWT OIDC;
- SQLite pour profils, roles, preferences et reglages admin X;
- le contrat HTTP/WebSocket Bearer existant.

## Decisions

- Le client Expo utilise `@logto/rn`.
- Le backend Rust verifie les JWT via JWKS, issuer, audience, expiration et subject.
- Les roles `listener`, `moderator`, `admin` restent locaux dans `user_profiles`.
- La migration utilisateurs passe par invitation/reset, sans reprise de mot de passe.

## Definition of Done

- Le client ne depend plus de l'ancien SDK auth.
- Le backend ne contient plus de verification JWT specifique a l'ancien fournisseur.
- `NEWS_PROFILE_STORE=sqlite` est la seule cible runtime.
- Les reglages X sont persistés dans SQLite.
- Les smokes HTTP/WebSocket Logto sont disponibles.
- La documentation active pointe vers [LOGTO-CONFIG](../LOGTO-CONFIG.md) et [AUTH-RUNBOOK](../AUTH-RUNBOOK.md).

## Tickets lies

- `NEWS-208` Cadrage Logto, tenant et contrat JWT
- `NEWS-209` Backend Rust: vérificateur OIDC/JWKS Logto
- `NEWS-210` Client Expo: remplacer l'ancien auth par Logto
- `NEWS-211` Profils, roles et préférences en SQLite uniquement
- `NEWS-212` Migration comptes et profils vers Logto + SQLite
- `NEWS-213` HTTP/WebSocket smokes Logto
- `NEWS-214` Nettoyage runtime de l'ancien fournisseur
- `NEWS-215` Go/no-go sécurité et release
