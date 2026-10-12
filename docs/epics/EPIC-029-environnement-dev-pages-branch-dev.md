# EPIC-029 - Environnement dev `dev` sur `fil-finance.pages.dev`

> **Archive historique — ne pas executer.** La cible frontend unique
> `fil-finance.pages.dev`, sans backend dev, est supersedee par la landing
> `dev.filfinance.fr`, une PWA distincte definie par `FILFINANCE_DEV_PWA_URL` et
> l'API `api-dev.filfinance.fr`. Utiliser
> [le runbook GitLab actuel](../GITLAB-RUNBOOK.md) et
> [les fichiers d'environnement actuels](../ENVIRONMENT-FILES.md).

## Objectif

Creer un environnement de developpement public pour le frontend Fil Finance:

- branche de developpement: `dev`;
- URL dev: `https://fil-finance.pages.dev`;
- deploiement: GitLab CI depuis la branche `dev`;
- backend dev public: hors scope du premier palier.

Cet EPIC demarre apres `EPIC-028`, une fois la production `main` migree sur
`https://filfinance.fr`.

## Valeur livree

- La branche `dev` dispose d'une URL stable pour valider le frontend avant passage sur `main`.
- `fil-finance.pages.dev` retrouve un role clair: environnement dev, et n'est plus la production provisoire.
- Le pipeline distingue explicitement `deploy:dev` et `deploy:main`.
- Les smokes dev restent non destructifs et adaptes a l'absence de backend dev public.

## Statut

Cadrage ajoute le 21 mai 2026.

Priorite: apres `EPIC-028`.

## Decisions

- `dev` devient la branche source de `https://fil-finance.pages.dev`.
- Le projet Pages `fil-finance` est reserve au dev.
- La production reste sur le projet Pages dedie par `EPIC-028`, avec `filfinance.fr`.
- Le dev public ne cree pas de backend dedie dans ce premier lot.
- Les tests backend et les tests de donnees restent locaux via `.env.local`.
- Supabase reste partage, donc l'environnement dev public ne doit pas servir de sandbox de donnees.

## Scope

Dans cet EPIC:

- creer ou remettre a niveau la branche `dev`;
- configurer le projet Pages `fil-finance` pour deployer la branche `dev`;
- ajouter le job GitLab `deploy:dev:pages`;
- ajouter un smoke frontend-only pour `https://fil-finance.pages.dev`;
- documenter le workflow de promotion `dev -> main`;
- documenter les limites de l'environnement dev public.

Hors scope:

- backend dev public;
- base Supabase dev separee;
- donnees de test partagees automatiquement;
- promotion automatique de `dev` vers `main`;
- protection avancee ou auth separee de l'environnement dev.

## Architecture cible dev

```text
dev
  -> GitLab CI validate
  -> deploy frontend Pages dev
       project: fil-finance
       domain: fil-finance.pages.dev
  -> smoke frontend-only
```

Le backend de developpement reste local:

```bash
docker compose --env-file .env.local -f compose.yaml -f compose.local.yaml up --build
```

URLs locales de reference:

- frontend local: `http://localhost:8052`;
- backend local: `http://localhost:8088`;
- health local: `http://localhost:8088/health`.

## Pipeline CI/CD cible

Jobs attendus sur `dev`:

- `docs:check`;
- `rust:fmt`;
- `rust:test`;
- `client:typecheck`;
- `client:web-build`;
- `edge:check`;
- `deploy:dev:pages`;
- `smoke:dev`.

Regles:

- `deploy:dev:pages` tourne uniquement sur `dev`;
- `deploy:dev:pages` ne deploie pas le backend serveur;
- `deploy:dev:pages` ne deploie pas le Worker edge production;
- `smoke:dev` verifie seulement le frontend et le fallback SPA;
- les variables Cloudflare dev sont separees des variables prod.

Variables GitLab a prevoir:

```text
CLOUDFLARE_ACCOUNT_ID
CLOUDFLARE_API_TOKEN_PAGES_DEV
PAGES_PROJECT_PROD=filfinance-prod
```

`fil-finance` reste legacy/non utilise pour ce palier. Le frontend dev est la
branche `dev` du projet Pages `filfinance-prod`, exposee via
`dev.filfinance.fr`.

Variables Pages dev:

```text
NODE_VERSION=24.13.1
EXPO_PUBLIC_FILFINANCE_WS_URL=wss://api-dev.filfinance.fr/ws/news
EXPO_PUBLIC_FILFINANCE_EDGE_WS_URL=wss://live-dev.filfinance.fr/ws/live
EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL=https://media-dev.filfinance.fr
EXPO_PUBLIC_SUPABASE_URL=https://project-ref.supabase.co
EXPO_PUBLIC_SUPABASE_PUBLISHABLE_KEY=sb_publishable_xxx
EXPO_PUBLIC_SUPABASE_ANON_KEY=eyJhbGciOi-placeholder
```

Le Worker dev reste control plane Edge uniquement; le data plane HLS Cloudflare est
retire par `EPIC-034`. Le data plane HLS dev passe par Bunny
`https://media-dev.filfinance.fr`.

Les URLs backend peuvent rester vides si le smoke dev ne couvre que le chargement
statique. Si elles sont pointees vers la production, cela doit etre documente
comme un choix temporaire non sandbox.

## Workflow `dev -> main`

Process nominal:

1. travailler localement sur une branche courte ou directement sur `dev`;
2. pousser vers `dev`;
3. attendre validations, `deploy:dev:pages` et `smoke:dev`;
4. verifier `https://fil-finance.pages.dev`;
5. integrer vers `main` par merge ou fast-forward controle;
6. laisser `EPIC-028` deployer la production depuis `main`.

Regles:

- `dev` peut etre instable, mais doit rester buildable;
- `main` reste la seule branche production;
- un incident dev ne doit pas modifier `filfinance.fr`;
- un incident prod se corrige sur `main`, pas par redeploiement de `dev`.

## Tickets de dev

### `NEWS-179` Projet Pages dev et branche `dev`

Objectif: reserver `fil-finance.pages.dev` au build de la branche `dev`.

Criteres d'acceptation:

- `fil-finance.pages.dev` sert le dernier commit de `dev`;
- `main` ne modifie plus ce domaine;
- le projet Pages dev n'a pas le custom domain `filfinance.fr`;
- les variables Pages dev ne contiennent aucun secret serveur.

### `NEWS-180` Pipeline `deploy:dev`

Objectif: ajouter le deploiement GitLab du frontend dev.

Criteres d'acceptation:

- `dev` lance les validations;
- `dev` lance `deploy:dev:pages`;
- le job utilise l'artefact `filfinance-client/dist`;
- aucun job serveur ou edge prod ne tourne sur `dev`.

### `NEWS-181` Smoke et runbook dev public

Objectif: rendre l'environnement dev verifiable sans toucher la production.

Criteres d'acceptation:

- `smoke:dev` verifie `/` et une route profonde;
- le smoke dev ne depend pas de l'API prod;
- le runbook explique le passage `dev -> main`;
- les limites liees a Supabase partage sont explicites.

## Definition of Done

- `https://fil-finance.pages.dev` sert la branche `dev`.
- `main` continue de deployer uniquement `https://filfinance.fr`.
- Le job `deploy:dev:pages` est separe des jobs `deploy:main:*`.
- Le smoke dev passe sans backend dev public.
- Le workflow de promotion `dev -> main` est documente.
- Les risques de donnees partagees avec Supabase sont visibles dans le runbook.

## References

- [EPIC-028 - Migration production `main` vers `filfinance.fr`](./EPIC-028-migration-production-filfinance-fr.md)
- [EPIC-020 - GitLab, Cloudflare Pages et serveur personnel](./EPIC-020-gitlab-pipeline-cloudflare.md)
- [GITLAB-RUNBOOK](../GITLAB-RUNBOOK.md)
- [ENVIRONMENT-FILES](../ENVIRONMENT-FILES.md)
