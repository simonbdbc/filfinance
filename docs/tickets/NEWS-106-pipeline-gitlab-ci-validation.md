# NEWS-106 - Pipeline GitLab CI de validation

## Objectif

Ajouter un premier pipeline GitLab CI qui valide le monorepo avant tout deploiement Cloudflare.

## Statut

Livre cote repo le 2 mai 2026.

## Scope livre

- fichier `.gitlab-ci.yml` a la racine du depot
- workflow GitLab pour branches poussees et tags, sans dependance aux merge requests
- job docs simple
- job Rust `cargo fmt --check`
- job Rust `cargo check` puis `cargo test`
- job client `npm ci` puis `npm run typecheck`
- job client `npm run build:web`
- artefact temporaire `filfinance-client/dist`
- script npm `build:web`

## Hors scope

- deploiement Cloudflare Pages
- deploiement Workers/Durable Objects
- secrets Cloudflare
- environnements preview/staging/prod
- smoke post-deploiement

## Pipeline

Jobs:

- `docs:check`
- `rust:fmt`
- `rust:test`
- `client:typecheck`
- `client:web-build`

Les jobs sont volontairement separes pour diagnostiquer vite la couche en echec.

Note: `NEWS-111` ajoute ensuite le job `deploy:home-srv` apres ces validations. Le scope de `NEWS-106` reste uniquement la validation.

Images CI:

- `alpine:3.22` pour les checks docs simples
- `rust:latest` pour les validations backend
- `node:24.13.1` pour le client Expo, aligne avec le runtime local pinne

## Criteres d'acceptation

- une branche poussee execute les validations
- `main` execute les validations
- les tags executent les validations
- aucun deploiement n'est lance par ce ticket
- le build web produit un artefact GitLab expire rapidement

## Validations locales

Commandes a executer localement:

```bash
cd filfinance/filfinance-service
cargo fmt --check
cargo check
cargo test

cd ../filfinance-client
npm run typecheck
npm run build:web
```

Validations executees le 2 mai 2026:

- check conflits docs: OK
- `cargo fmt --check`: OK
- `cargo check`: OK
- `cargo test`: OK, 71 tests
- `npm run typecheck`: OK avec Node moderne
- `npm run build:web`: OK, export `dist`
- `filfinance-client/dist` reste ignore par Git

Note locale: le `node` par defaut de la machine etait `v14.6.0`, trop ancien pour `npm`/Expo 55. Le projet est maintenant pinne en `24.13.1` via `.nvmrc`, `.node-version`, `package.json engines` et l'image GitLab `node:24.13.1`.

## Dependances

- [EPIC-020 - GitLab et pipeline Cloudflare](../epics/EPIC-020-gitlab-pipeline-cloudflare.md)
- [NEWS-105 - Depot GitLab et conventions de branches](./NEWS-105-depot-gitlab-conventions-branches.md)
