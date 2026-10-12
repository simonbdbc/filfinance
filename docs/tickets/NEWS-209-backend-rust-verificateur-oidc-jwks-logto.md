# NEWS-209 - Backend Rust: verificateur OIDC/JWKS Logto

> **Archive historique — ne pas executer.** Le palier Logto est supersede par
> Better Auth + PostgreSQL. Utiliser [le runbook Better Auth](../AUTH-RUNBOOK.md).

## Statut

Livre cote repo.

## Objectif

Remplacer la verification JWT specifique a l'ancien fournisseur par une verification OIDC Logto.

## Acceptance

- Le backend verifie JWKS, issuer, audience, expiration et subject.
- Les algorithmes HMAC sont refuses.
- Les roles applicatifs restent lus depuis SQLite.
- `cargo check` passe.
