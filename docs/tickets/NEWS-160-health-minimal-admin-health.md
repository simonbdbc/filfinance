# NEWS-160 - Health minimal et admin health detaille

## Objectif

Clarifier les surfaces de health:

- `/health`: public minimal pour l'infra;
- `/api/admin/health`: diagnostic detaille reserve admin.

## Scope

- Garder `/health` public avec uniquement:

```json
{ "status": "ok" }
```

- Ajouter `/api/admin/health` protege admin.
- Exposer dans `/api/admin/health`:
  - DB SQLite pipeline;
  - Supabase profile store;
  - traduction;
  - TTS;
  - HLS;
  - sources;
  - collector navigateur;
  - Edge publisher;
  - stockage local;
  - version/config non sensible si utile.
- Ajouter rate-limit infra sur `/health` si necessaire.

## Hors scope

- Observabilite complete avec metrics Prometheus.
- Dashboard externe.
- Alerting.

## Criteres d'acceptation

- `/health` ne revele aucune information interne.
- `/api/admin/health` retourne `401` sans session.
- `/api/admin/health` retourne `403` pour role non admin.
- Un admin voit les diagnostics detailles.
- Les secrets, tokens, mots de passe et chemins sensibles ne sont jamais exposes.

## Tests attendus

- Test HTTP `/health` public minimal.
- Tests autorisation `/api/admin/health`.
- Tests redaction des champs sensibles.

## Implementation

- `/health` reste public et retourne uniquement `{ "status": "ok" }`.
- `/api/admin/health` est monte dans les routes admin et herite donc du middleware admin:
  session Supabase obligatoire, role `admin` obligatoire.
- Le diagnostic admin expose:
  - sante SQLite pipeline;
  - sante du store profils configure;
  - etats runtime traduction, TTS, HLS;
  - etats sources et collector navigateur;
  - configuration Edge sous forme de booleens non sensibles;
  - stockage local sous forme de booleens/config non sensible.
- Les erreurs runtime sont resumees par categorie. Les chemins locaux, URLs privees, secrets,
  tokens, mots de passe et erreurs brutes ne sont pas exposes.

Validation code:

```bash
cargo test health
```

## Dependances

- `EPIC-026`
- `NEWS-156`
