# EPIC-020 - GitLab, Cloudflare Pages et serveur personnel

> **Archive historique.** Les choix Git Integration et frontend unique de cet
> epic sont supersedes par le Direct Upload GitLab de deux projets. Utiliser
> [le runbook GitLab actuel](../GITLAB-RUNBOOK.md).

## Objectif

Versionner `filfinance` sur GitLab et mettre en place un pipeline CI/CD qui valide le monorepo, garde le frontend sur Cloudflare Pages et deploie le backend Rust sur le serveur personnel Traefik.

## Valeur livree

Le projet gagne un chemin de livraison reproductible:

- tests et checks avant deploiement
- previews frontend Cloudflare Pages
- deploiement frontend Cloudflare Pages
- deploiement serveur personnel pour le backend Rust
- deploiement Workers/Durable Objects possible plus tard pour le control plane edge
- separation claire dev/staging/prod
- rollback documente

## Statut

Cadrage ajoute le 2 mai 2026.

Priorite 1 au 2 mai 2026.

Avancement:

- `NEWS-105` est livre: README racine, `.gitignore` racine, runbook GitLab, depot local `main`, remote GitLab et push initial ajoutes.
- `NEWS-106` est livre cote repo: pipeline GitLab CI de validation, jobs Rust/client/docs et build web Expo sans deploiement Cloudflare.
- `NEWS-107` est livre cote repo: workflow solo-dev sans merge request obligatoire et Cloudflare Pages Git integration documentes. La validation live attend la configuration du projet Pages.
- `NEWS-111` est livre cote repo: audit `home-srv`, compose Docker prod backend-only, job SSH `deploy:home-srv` et runbook Traefik API.
- `NEWS-112` est livre cote `home-srv`: route Traefik API `filfinance-api.sbcenter.fr` vers `filfinance-service@docker`.
- `NEWS-113` est livre cote repo: bootstrap serveur, `.env.server`, variables GitLab et premier deploy backend documentes.
- `NEWS-109` est livre cote repo: secrets, environnements et rollback Pages/backend sont formalises.
- `NEWS-110` est livre cote repo: script smoke post-deploiement Pages/backend avec checks authentifies optionnels.
- Le backend serveur personnel est valide live.
- Le frontend Cloudflare Pages est valide live sur le domaine provisoire `https://fil-finance.pages.dev`.
- Le smoke public Pages plus API health passe avec `scripts/smoke_deploy.sh`.
- Les pipelines GitLab et Cloudflare sont valides.
- Le frontend recupere bien les news depuis `wss://filfinance-api.sbcenter.fr/ws/news`.
- Le domaine custom `filfinance.sbcenter.fr` est abandonne: la cible prod finale sera `filfinance.fr`.
- Le runbook de passage prod final est documente dans [PRODUCTION-FINAL-RUNBOOK](../PRODUCTION-FINAL-RUNBOOK.md).
- Les erreurs runtime profil/historique observees apres deploiement sont resolues.
- `NEWS-121` corrige le build Cloudflare Pages NativeWind en precreant le cache `react-native-css-interop/.cache/web.css` avant `expo export`.
- `NEWS-123` formalise le workflow simplifie de refonte live: ancien `main` stable sauvegarde sur `backup/main-prod-before-epic18-2026-05-03` et tag `prod-before-epic18-2026-05-03`, puis refonte deployee depuis `main`.
- `NEWS-124` ajoute le smoke runtime backend post-deploy: health conteneur toujours verifie, Piper teste si `NEWS_TTS_ENABLED=true`, HLS teste si `NEWS_HLS_ENABLED=true`.
- `NEWS-125` complete le chemin deploy audio en packagant le runtime Docker Piper attendu par le smoke.
- `NEWS-126` complete le chemin deploy traduction en installant le runtime OPUS-MT et le modele CTranslate2 pendant le build Docker, avec smoke post-deploy bloquant.

Reste a terminer en priorite 1:

- conserver `https://fil-finance.pages.dev` comme domaine de travail jusqu'a la prod finale;
- executer le runbook `PRODUCTION-FINAL-RUNBOOK` quand la cible `filfinance.fr` sera prete;
- decider si le script `NEWS-110` devient un job CI automatise avant production finale.

Replanification du 21 mai 2026:

- `EPIC-028` reprend la migration production: branche `main`, frontend `filfinance.fr`, API `api.filfinance.fr`, edge `live.filfinance.fr`, developpement local uniquement;
- `EPIC-029` reprend ensuite la creation de l'environnement dev public: branche `dev`, URL `fil-finance.pages.dev`;
- `EPIC-020` reste le palier historique GitLab, Cloudflare Pages provisoire et backend serveur personnel.

## Decision CI/CD

Deux integrations Cloudflare existent:

- Git integration Cloudflare Pages/Workers connectee a GitLab
- GitLab CI/CD externe avec Wrangler

Decision immediate pour Fil Finance:

- Cloudflare Pages reste le chemin de deploiement prioritaire du frontend.
- Le serveur personnel devient le chemin de deploiement prioritaire du backend Rust.
- `main` est la branche de travail et de deploiement.
- pendant `EPIC-018`, `EPIC-019` et `EPIC-021`, l'environnement courant peut servir d'environnement de refonte live avec rollback sur le backup `main`.
- Les merge requests ne sont pas obligatoires dans le workflow solo-dev.
- GitLab CI valide puis deploie le backend par SSH dans `$DEPLOY_PATH`.
- Cloudflare Pages expose provisoirement `https://fil-finance.pages.dev`.
- Traefik expose `filfinance-api.sbcenter.fr`.

Raison: le backend Rust actuel est un processus long-vivant avec SQLite, ingestion continue et futurs assets TTS locaux. Un serveur personnel Docker est plus simple et moins cher que Cloudflare Containers pour ce runtime.

Decision cible avant production finale:

- GitLab reste le pipeline maitre.
- Les smoke tests post-deploiement bloquent les promotions sensibles.
- Cloudflare Durable Objects/CDN peut revenir pour scaler metadata et audio si le serveur personnel devient insuffisant.

Point d'attention: Cloudflare Pages Git integration peut demarrer son build independamment du resultat GitLab CI. Ce compromis reste accepte pour le workflow solo-dev actuel.

## Scope

Dans cet EPIC:

- initialiser le depot GitLab
- definir le workflow solo-dev sur `main` et les branches optionnelles
- ajouter GitLab CI pour checks backend/frontend/docs
- bootstrap deploiement backend serveur personnel via SSH
- integration Traefik API/WebSocket
- cible ulterieure Cloudflare Durable Objects/CDN
- gestion des secrets serveur/Supabase
- environnements preview/staging/prod
- runbook de rollback

Hors scope:

- choix final de l'hebergeur du backend Rust origin
- migration complete production du backend Rust
- observabilite metier avancee

## Tickets de dev

### [`NEWS-105` Depot GitLab et conventions de branches](../tickets/NEWS-105-depot-gitlab-conventions-branches.md)

Objectif: versionner proprement le workspace `filfinance`.

Scope:

- creation depot GitLab
- branche `main` comme branche de travail et de deploiement
- branches optionnelles pour previews ou changements risques
- tags release
- regles de commit/deploiement

Criteres d'acceptation:

- aucun secret local n'est versionne
- les dossiers `filfinance-service`, `filfinance-client` et `docs` sont presents
- le workflow solo-dev sans merge request obligatoire est documente

### [`NEWS-106` Pipeline GitLab CI de validation](../tickets/NEWS-106-pipeline-gitlab-ci-validation.md)

Objectif: lancer les checks avant tout deploiement.

Scope:

- Rust `cargo fmt --check`
- Rust `cargo check`
- Rust `cargo test`
- client `npm ci`
- client `npm run typecheck`
- build web Expo si script ajoute
- checks docs simples

Criteres d'acceptation:

- une branche poussee lance les validations
- `main` lance les validations
- les artefacts utiles sont conserves le temps necessaire
- les jobs sont separes backend/client/docs pour diagnostiquer vite

### [`NEWS-107` Cloudflare Pages frontend Git integration](../tickets/NEWS-107-cloudflare-pages-git-integration-bootstrap.md)

Objectif: deployer l'app statique Expo Web avec Cloudflare Pages Git integration pour un workflow solo-dev simple.

Scope:

- build web du client
- variables publiques par environnement
- configuration Cloudflare Pages Git integration
- previews de branches optionnelles
- deploiement production automatique depuis `main`
- rollback Cloudflare Pages documente

Criteres d'acceptation:

- Cloudflare Pages peut build `filfinance-client` depuis GitLab
- `main` deploie l'environnement production Pages
- une branche temporaire peut produire une preview si elle est poussee
- la migration ulterieure vers un process GitLab maitre reste identifiee

### [`NEWS-111` Deploiement serveur perso Traefik](../tickets/NEWS-111-deploiement-home-srv-traefik.md)

Objectif: deployer le backend Fil Finance sur le serveur personnel existant derriere Traefik.
Le frontend reste heberge sur Cloudflare Pages.

Scope:

- audit `home-srv`
- `compose.yaml` + `compose.prod.yaml` backend-only
- backend Rust Docker avec healthcheck
- job GitLab SSH `deploy:home-srv`
- proposition de fichier dynamique Traefik pour l'API
- runbook variables serveur

Criteres d'acceptation:

- GitLab peut deploier `main` par SSH
- le serveur build et lance `filfinance_service`
- Traefik peut router l'API et les WebSockets
- SQLite persiste hors Git

### [`NEWS-112` Route Traefik API Fil Finance](../tickets/NEWS-112-route-traefik-api-filfinance.md)

Objectif: appliquer la route API Fil Finance dans le repo `home-srv/traefik-docker`.

Scope:

- fichier dynamique Traefik `volumes/config/conf/filfinance.yaml`
- router `filfinance-api.sbcenter.fr`
- service Docker `filfinance-service@docker`
- middleware `default-headers`
- pas de `traefik-auth` sur l'API

Criteres d'acceptation:

- la route Traefik API est versionnable dans `home-srv/traefik-docker`
- le frontend reste absent du serveur personnel
- la config YAML est valide
- la route attend le service Docker Fil Finance sur `traefik_network`

### [`NEWS-113` Bootstrap serveur et premier deploy backend](../tickets/NEWS-113-bootstrap-serveur-premier-deploy.md)

Objectif: preparer le premier deploiement backend Fil Finance sur le serveur personnel.

Scope:

- dossier serveur cible
- clone GitLab du repo Fil Finance
- creation de `.env.server`
- variables GitLab CI/CD SSH
- validation Compose distante
- premier `docker compose up -d --build --force-recreate`
- smoke tests API et frontend Pages

Criteres d'acceptation:

- le runbook de bootstrap est executable pas a pas
- `.env.server` reste hors Git
- le job `deploy:home-srv` valide Compose avant deploy
- le premier deploy backend peut etre lance depuis `main`

### [`NEWS-124` Smoke runtime backend Piper/HLS post-deploy](../tickets/NEWS-124-smoke-runtime-backend-piper-hls.md)

Objectif: echouer le deploiement serveur si les flags audio annoncent Piper ou HLS actifs alors que le runtime conteneur ne fonctionne pas.

Scope:

- script `scripts/smoke_backend_runtime.sh`
- healthcheck HTTP depuis le conteneur
- checks Piper conditionnes par `NEWS_TTS_ENABLED=true`
- checks HLS conditionnes par `NEWS_HLS_ENABLED=true`
- integration dans `deploy:home-srv`

Criteres d'acceptation:

- le deploiement continue quand TTS/HLS sont desactives
- le deploiement echoue si Piper est active sans runtime, helper, modele, voix ou synthese valide
- le deploiement echoue si HLS est actif sans TTS, `ffmpeg`, dossier writable ou playlist valide
- le script est executable manuellement sur le serveur

### `NEWS-108` Deploiement Workers et Durable Objects

Objectif: deployer le Worker edge et le Durable Object live.

Scope:

- `wrangler.toml` ou fichier equivalent
- bindings Durable Objects
- environnements staging/prod
- secrets Worker
- `wrangler deploy`

Criteres d'acceptation:

- le Worker est deployable depuis GitLab CI
- les migrations/bindings Durable Objects sont versionnes
- les secrets ne sont jamais presents dans le repo

### [`NEWS-109` Secrets, environnements et rollback](../tickets/NEWS-109-secrets-environnements-rollback.md)

Objectif: formaliser l'exploitation du pipeline.

Scope:

- variables GitLab protegees
- tokens Cloudflare scopes minimaux
- variables Supabase publiques et serveur
- rollback Pages
- rollback Worker
- checklist release

Criteres d'acceptation:

- les secrets Cloudflare sont uniquement dans GitLab CI variables
- le rollback est faisable sans commit correctif
- la documentation distingue preview, staging et prod

### [`NEWS-110` Smoke post-deploiement Pages et backend](../tickets/NEWS-110-smoke-post-deploiement-cloudflare.md)

Objectif: verifier automatiquement les surfaces deployees.

Scope:

- health frontend Pages
- chargement app statique
- fallback SPA Pages
- backend health Traefik
- endpoints HTTP authentifies si un token Supabase est fourni
- smoke auth minimal si environnement configure

Criteres d'acceptation:

- un deploiement vert prouve que Pages et le backend repondent
- le script est executable localement apres deploy
- le script peut devenir bloquant dans GitLab quand Pages passera sous controle CI direct

### [`NEWS-121` Fix build Cloudflare Pages NativeWind cache](../tickets/NEWS-121-fix-build-cloudflare-pages-nativewind-cache.md)

Objectif: rendre le build Cloudflare Pages reproductible en environnement frais apres migration NativeWind.

Scope:

- script `prebuild:web`
- creation preventive de `node_modules/react-native-css-interop/.cache/web.css`
- alignement de la contrainte npm sur le runtime Cloudflare observe

Criteres d'acceptation:

- `npm run build:web` passe quand le cache NativeWind web est absent
- Cloudflare Pages ne bloque plus sur le SHA-1 de `web.css`
- le build local continue de passer

### [`NEWS-123` Workflow main backup pour refonte live](../tickets/NEWS-123-workflow-main-backup-refonte-live.md)

Objectif: garder un workflow leger pendant la refonte live sans maintenir deux environnements complets.

Scope:

- sauvegarde de l'ancien `main` stable;
- branche backup et tag rollback;
- integration de la refonte sur `main`;
- rollback rapide documente.

Criteres d'acceptation:

- l'ancien `main` est conserve sur `backup/main-prod-before-epic18-2026-05-03`
- le tag `prod-before-epic18-2026-05-03` pointe le meme commit stable
- le runbook explique comment revenir au backup
- les limites du choix sont explicites

## Definition of Done

- le workspace est versionne sur GitLab
- GitLab CI valide backend, client et docs
- Cloudflare Pages expose le frontend Fil Finance
- le serveur personnel peut deployer le backend de `main` par SSH
- Traefik expose le backend Fil Finance
- les secrets sont geres hors Git dans `.env.server` et variables GitLab
- les previews et rollback sont documentes

## References externes

- [Cloudflare Pages GitLab integration](https://developers.cloudflare.com/pages/configuration/git-integration/gitlab-integration/)
- [Cloudflare Pages Direct Upload avec CI](https://developers.cloudflare.com/pages/how-to/use-direct-upload-with-continuous-integration/)
- [Cloudflare Workers GitLab CI/CD](https://developers.cloudflare.com/workers/ci-cd/external-cicd/gitlab-cicd/)
- [Cloudflare Workers Builds Git integration](https://developers.cloudflare.com/workers/ci-cd/builds/git-integration/)
