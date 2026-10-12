# Runbook GitLab, Bunny et serveur Fil Finance

## Objectif

Ce runbook fixe le workflow GitLab, le deploiement frontend Bunny et le deploiement backend serveur de Fil Finance.

Decision actuelle:

- developpement solo;
- pas de merge request obligatoire;
- les releases PROD liees au calendrier partent directement de `main`; DEV est
  en pause et ne constitue pas une preuve de promotion;
- l'ancien `main` stable est sauvegarde dans `backup/main-prod-before-epic18-2026-05-03` et `prod-before-epic18-2026-05-03`;
- GitLab CI comme filet de validation;
- landing et PWA hebergees dans deux couples Bunny Storage/Pull Zone, publies
  par GitLab sans dependance au backend ni au Worker Edge;
- deploiement SSH du backend vers le VPS OVH dans `/srv/filfinance`;
- le runner protege `home-runner-docker` reste execute sur `home-srv`;
- Traefik OVH route exclusivement `api.filfinance.fr` vers le gateway Docker.

Il couvre les premiers tickets de `EPIC-020`, jusqu'au bootstrap serveur `NEWS-113`.
Les chemins et hôtes `sbalgo-srv` documentés plus loin sont historiques. La
procedure de production active est [le runbook OVH](./OVH-PRODUCTION-RUNBOOK.md).

Topologie frontend cible de la CI :

| Environnement | Landing | PWA | Branche source |
| --- | --- | --- | --- |
| PROD | Bunny `filfinance-prod-landing` → `https://filfinance.fr` | Bunny `filfinance-prod-pwa` → `https://app.filfinance.fr` | `main` |
| DEV | Bunny Storage → `https://dev.filfinance.fr` | Bunny Storage → `https://app.dev.filfinance.fr` | `dev` |

GitLab est le pipeline maitre des deploys Bunny. Cloudflare conserve les CNAME
DNS-only frontend et les surfaces API/live/mail. Les anciennes sections Pages
sont historiques; la procedure active est
[BUNNY-PRODUCTION-RUNBOOK](./BUNNY-PRODUCTION-RUNBOOK.md).

Decision active pour `EPIC-037` (30 aout 2026) : le developpement calendrier
se fait directement sur `main`. Le chemin canonique est Rust natif ; n8n a ete
retire du depot et de la CI. Les executions DEV restent en pause. La seule gate
fonctionnelle est le test Rust de recuperation complet du SHA exact, execute
localement puis en CI, avant le deploiement PROD manuel. Les volumes n8n
historiques ne sont pas purges. Les anciennes exigences de promotion `dev`, de
`smoke:dev`, de shadow ou de parite n8n ne sont pas des gates de la migration.

## Incidents SSH de release des 5 et 6 septembre 2026

Statut au 6 septembre, 09:46 UTC : les correctifs ci-dessous sont livrés par
la pipeline web [#350](https://gitlab.com/simonbdbc/filfinance/-/pipelines/2823982017)
sur `808a177129134746be6eb08aacfdf57a516326f0`. Ses 17 jobs sont verts, dont
la récupération Rust au premier essai, les quatre déploiements et les deux
smokes PROD. La coupure API observée pendant `--force-recreate` dure environ
26 secondes ; backend, auth, Piper/HLS et gateway passent leurs contrôles.
Les smokes authentifiés valident les rôles, le WebSocket live, les audits
calendrier HTTP/WebSocket admin et un manifeste/segment existants via Bunny.
Les trois comptes smoke ont zéro session résiduelle après la CI.
L'observation après succès backend, de 09:16:21 à 09:46:24 UTC, est validée :
1 802 secondes, 30 échantillons sains, aucun redémarrage imprévu, erreur
backend/audio ou anomalie de cohérence calendrier. La persistance live progresse
de 116 à 119 news retenues ; la fin de fenêtre est calme. Le smoke WebSocket
valide le hello et les permissions, pas un débit continu. Les deux sources
calendrier refusées en 403 restent un état dégradé explicitement accepté.
La livraison est clôturée ; conserver ces preuves avec celles de la CI.
Rollback : `prod-before-calendar-rust-20260905` (`67b48bb`). Les preuves
rédigées sont archivées hors Git dans `backups/calendar-rust-release-20260905/`.

Le backend de #341 a echoue avant toute connexion distante : OpenSSH emet aussi
des commentaires sur stdout de `ssh-keyscan`; `ssh-keygen -lf -` refuse ces
lignes avec le code 255. Le bloc CI ignore uniquement les commentaires et lignes
vides, puis conserve la comparaison exacte a `SSH_HOST_KEY_SHA256` et
`StrictHostKeyChecking=yes`. Les cles invalides et empreintes differentes restent
refusees. `python3 scripts/test_ssh_host_key_pinning.py` teste le bloc CI reel,
sans reseau, avec des cles jetables ; en Alpine, utiliser
`FILFINANCE_SSH_PIN_TEST_SHELL=sh` pour couvrir BusyBox ash.

#344 sur `1d3a26c` a ensuite validé les onze checks sans retry ; le pinning SSH
est corrigé. Son backend échoue avant SSH sur quatre guillemets internes non
échappés dans la commande distante multilignes. Le correctif de citations est
couvert par `python3 scripts/test_backend_deploy_shell.py` : parsing du shell CI,
capture d'un unique argument distant avec un faux SSH, absence de substitutions
Git/Docker locales et parsing du shell distant. Utiliser
`FILFINANCE_DEPLOY_TEST_SHELL=sh` dans Alpine. Ne pas relancer le backend de #344 :
la release repart d'une nouvelle pipeline web sur le SHA corrigé.
La landing et la PWA Bunny de #344 sont déjà publiées ; cela ne valide ni le
backend, ni l'edge, ni la suite smoke complète.

Le 6 septembre, les trois comptes smoke PROD autorisés ont été provisionnés
via Better Auth (listener, moderator, admin ; adresses vérifiées). Les six
variables `FILFINANCE_SMOKE_*` nouvelles sont protégées, masquées et cachées,
sans expansion, avec le scope exact `production/smoke`. Les entrées DEV
historiques restent intactes. Les trois connexions HTTPS, rôles JWT et
déconnexions sont validés ; aucun token n'est persisté et aucune session ne
reste ouverte. Preuve rédigée hors Git :
`backups/calendar-rust-release-20260905/smoke-prod-provisioning-20260906.json`.

Le smoke PROD complete les checks auth/live avec
`FILFINANCE_NATIVE_CALENDAR_OBSERVE=1` : GET calendrier public/admin, snapshot
du WebSocket admin et correspondance des audits rediges, puis lecture d'un
manifeste et d'un segment replay existants sur `media.filfinance.fr` avec un
ticket HLS obtenu normalement. Le script ne publie ni news ni calendrier ;
les reponses sont bornees a 1 Mio, sans redirection/retry et sans secret dans
les sorties. Un replay absent est signale explicitement, pas genere pour le
smoke. Ce controle post-deploiement ne remplace pas l'observation de 30 minutes
et ne cree aucune gate de parite n8n. Les tests de ce script sont sans reseau.

La pipeline #339 exclue a ete annulee sans supprimer ses resultats : ses jobs
encore crees retenaient #341 dans la file `production` en `oldest_first`.
Ne pas modifier ce mode ni lancer un ancien deploiement pour liberer la file.
Le rollback OVH exact avant cette livraison est
`prod-before-calendar-rust-20260905` (`67b48bb`).

## Depot cible

Type de depot:

- monorepo GitLab;
- racine du depot: dossier `filfinance`;
- branche principale: `main`;
- `main` est la branche de travail et de deploiement tant que le projet reste solo-dev.

Contenu versionne:

- `filfinance-service`;
- `apps/pwa`;
- `apps/landing`;
- `docs`;
- fichiers racine de configuration, dont `.gitignore`, `.gitlab-ci.yml`, `.nvmrc` et `.node-version`.

Contenu non versionne:

- `.env` et variantes locales;
- `node_modules`;
- `target`;
- `.expo`;
- `dist`, `web-build`, `dist-web-smoke`;
- bases SQLite locales;
- modeles locaux volumineux;
- secrets Cloudflare, Better Auth, PostgreSQL, SMTP, FinancialJuice et Telegram.

## Initialisation locale

Depuis le dossier parent:

```bash
cd /Users/simon/Projets/PlateformeDev/dev-trade-analyse/filfinance
git init -b main
git status --short
```

Verifier que les secrets et artefacts locaux ne sont pas proposes par `git status`.

Ajouter le remote GitLab:

```bash
git remote add origin git@gitlab.com:simonbdbc/filfinance.git
```

Premier commit (apres inspection explicite des fichiers):

```bash
git add <fichiers-valides>
git diff --cached --check
git commit -m "Initial commit"
git push -u origin main
```

## Workflow solo-dev

Process actif pour la migration calendrier sur `main`:

```bash
git status --short
git add <fichiers-valides>
git diff --cached --check
git commit -m "type(scope): message"
git push origin main
```

Regles pragmatiques:

- une promotion de code sur `main` est non mutante : elle valide le SHA mais
  ne lance ni preflight, ni deploiement, ni smoke PROD;
- une release runtime calendrier depuis `main` reste explicite et manuelle. La
  preuve fonctionnelle requise est le test Rust de recuperation complet du SHA
  exact, execute localement puis en CI. Aucun smoke DEV, shadow ni parite n8n
  ne remplace cette preuve;
- les jobs PROD restent manuels et bloquants apres cette preflight : une
  promotion de source ne peut donc jamais recreer le backend, Piper/HLS, Bunny
  ou Edge;
- aucun force-push sur `main`;
- commits courts et lisibles;
- les validations locales Rust precedent le push; la CI du meme SHA doit ensuite
  rester verte avant les jobs PROD manuels;
- GitLab CI doit rester vert;
- GitLab publie les artefacts frontend vers Bunny Storage; aucun deploiement
  Cloudflare Pages ne publie en parallele;
- les jobs backend deploient le SHA CI exact, jamais le dernier commit implicite d'une branche;
- les verrous Better Auth/dev/prod restent desactives tant que leurs prerequis externes ne sont pas verifies.

Les merge requests restent possibles plus tard, mais elles ne sont pas obligatoires dans le workflow actuel.

## Historique: refonte live avec backup main

Cette section decrit le palier precedent et ne remplace pas la regle `main`
actuelle. `main` doit maintenant rester deployable.

Pour eviter une pile `dev/prod` lourde pendant `EPIC-018`, `EPIC-019` et `EPIC-021`, l'environnement courant reste deploye depuis `main`.

Etat stable conserve avant refonte live:

```text
commit: 47d49c2
branch: backup/main-prod-before-epic18-2026-05-03
tag: prod-before-epic18-2026-05-03
```

Snapshot historique du runbook (mai 2026, ne pas confondre avec le HEAD actuel):

```text
main -> 76db3ac
```

Voir [NEWS-123](./tickets/NEWS-123-workflow-main-backup-refonte-live.md) pour les commandes de creation du backup, d'integration et de rollback.

Regle historique de ce palier:

- `main` pouvait etre instable pendant cette refonte live; la politique actuelle
  exige au contraire un `main` deployable et une gate Rust locale/CI;
- la branche backup et le tag ne doivent pas etre supprimes avant validation complete;
- si la refonte bloque l'environnement courant, revenir au backup puis relancer les smoke tests.

## Branches optionnelles

Les branches sont optionnelles et servent surtout aux previews Cloudflare ou aux changements risqués.

Nommage recommande:

- `preview/description-courte`;
- `feature/NEWS-xxx-description`;
- `fix/NEWS-xxx-description`;
- `chore/NEWS-xxx-description`.

Quand une branche hors `dev`/`main` est poussee:

- GitLab CI lance les validations;
- aucune preview Cloudflare n'est publiee automatiquement; ajouter un job Direct Upload explicite si ce besoin devient reel.

## GitLab CI

Le pipeline GitLab valide le monorepo. Les releases runtime sont declenchees
manuellement depuis `main`; les executions DEV sont en pause et ne participent
pas a la migration calendrier.

Avec `FILFINANCE_PROD_DEPLOY_ENABLED=true` et
`FILFINANCE_PROD_BUNNY_DEPLOY_ENABLED=true`, GitLab deploie la landing et la
PWA sur Bunny. Les jobs statiques n'ont aucune dependance au backend ou a l'edge.

Jobs actifs pertinents pour une release `main`:

- `docs:check`;
- `rust:fmt`;
- `rust:test`;
- `edge:check`;
- `auth:check`;
- `auth:integration`;
- `landing:check`;
- `client:typecheck`;
- `client:web-build`;
- le test de recuperation calendrier Rust;
- `deploy:main:backend`;
- `smoke:main`.

Les jobs DEV et previews Cloudflare sont en pause. Aucun job DEV ne constitue
une gate ou une etape de la migration Rust.

Jobs production frontend actifs :

- `deploy:main:bunny-landing`;
- `deploy:main:bunny-pwa`;
- `smoke:main:frontends` apres certificats Bunny et bascule DNS;
- `deploy:main:edge`;
- `smoke:main`.

Le workflow CI tourne sur:

- toute branche poussee;
- les tags.

Le job `client:web-build` conserve temporairement `apps/pwa/dist` comme artefact pour faciliter le diagnostic. Le job `landing:check` conserve `apps/landing/dist` et les rapports Lighthouse; sa gate pessimiste exige `0,80` en performance et `1,00` dans les trois autres categories, sur mobile comme sur desktop.

Les jobs backend tournent uniquement sur leur branche et quand
`FILFINANCE_BETTER_AUTH_DEPLOY_ENABLED=true`. Ce verrou reste desactive jusqu'a
la validation des fichiers serveur par `scripts/check_better_auth_env.sh`.

### Pause explicite des executions DEV

L'environnement DEV est fige. La variable CI globale
`FILFINANCE_DEV_EXECUTION_ENABLED` reste a `false`; elle masque les deploys,
smokes et observations DEV. Les validations de code restent disponibles et
n'ecrivent pas dans cet environnement.

Une reprise DEV releve d'une decision d'exploitation distincte. Elle ne doit
ni etre executee pour EPIC-037, ni etre ajoutee comme precondition d'une
promotion PROD. La preuve de migration reste le test de recuperation Rust du
SHA `main` exact.

## Version Node projet

Version Node officielle du projet:

- `.nvmrc`: `24.13.1`;
- `.node-version`: `24.13.1`;
- image CI client: `node:24.13.1`;
- npm attendu: `11.11.0`.

Les builds Bunny tournent dans GitLab avec cette version Node; Bunny ne relance
aucun build.

## Secrets et variables

Voir aussi [Fichiers d'environnement](./ENVIRONMENT-FILES.md) pour le role de chaque `.env*`.

Le ticket [NEWS-109](./tickets/NEWS-109-secrets-environnements-rollback.md) conserve
le contexte historique de l'ancienne architecture; le present runbook et
[Fichiers d'environnement](./ENVIRONMENT-FILES.md) font foi pour Better Auth, le
monorepo et les deux sites Bunny de production.

Variables publiques Expo requises pour le build Web PWA:

- `EXPO_PUBLIC_FILFINANCE_WS_URL`;
- `EXPO_PUBLIC_FILFINANCE_EDGE_WS_URL`;
- `EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL`;
- `EXPO_PUBLIC_FILFINANCE_PWA_URL`;
- `EXPO_PUBLIC_API_URL`;
- `EXPO_PUBLIC_BETTER_AUTH_URL`.

Ces variables sont publiques dans le bundle Web. Elles ne doivent jamais contenir
de secret serveur. Le pipeline les fixe par branche dans `.gitlab-ci.yml`; ne pas
creer d'override GitLab ou Bunny et ne pas les mettre dans `.env.server`.

Secrets serveur a ne jamais exposer au client:

- `BETTER_AUTH_SECRET` et mots de passe PostgreSQL/SMTP;
- secrets FinancialJuice;
- secrets Telegram;
- futurs secrets de publication edge;
- futures cles Cloudflare API pour Wrangler.

Variables GitLab requises pour le deploiement serveur:

- `SSH_PRIVATE_KEY`;
- `USER=gitlab-ci`;
- `HOST`, cible le VPS OVH;
- `SSH_PORT`, port SSH configure sur le VPS OVH;
- `SSH_HOST_KEY_SHA256`, empreinte `SHA256:...` relevee hors bande sur le
  serveur autorise. La decouverte `ssh-keyscan` est toujours comparee a ce pin
  avant toute connexion et ne constitue jamais seule la racine de confiance.

Les anciens jobs d'observation calendrier DEV/shadow ne consomment plus ces
variables : ils ont ete retires de la CI. La cle privee reste masquee/protegee,
et toutes les variables d'integrite sont protegees.

Le fallback historique `PORT` reste accepte par le pipeline pendant la migration
de la variable GitLab, mais doit etre supprime pour ne pas entrer en conflit avec
le port HTTP standard des services Node.js.

Variables GitLab optionnelles pour remplacer les chemins par defaut:

- `FILFINANCE_PROD_DEPLOY_PATH`, defaut `/srv/filfinance`;
- `FILFINANCE_DEV_DEPLOY_PATH`, defaut `/raid/app/filfinance/filfinance-dev`.

L'ancienne variable globale `DEPLOY_PATH=/home/traefik/filfinance-docker` correspond au deploiement `home-srv`. Le pipeline `sbalgo-srv` calcule explicitement le chemin depuis `FILFINANCE_PROD_DEPLOY_PATH` ou `FILFINANCE_DEV_DEPLOY_PATH`.

Variables GitLab du contrat Bunny/Edge :

- `CLOUDFLARE_ZONE_ID`, identifiant de la zone `filfinance.fr` utilise par le
  preflight DNS; cette valeur n'est pas un secret;
- `FILFINANCE_PROD_PWA_URL=https://app.filfinance.fr`, sans slash final;
- `BUNNY_API_KEY_PROD` (masquee/protegee);
- `BUNNY_STORAGE_ZONE_PROD_LANDING=filfinance-prod-landing`;
- `BUNNY_STORAGE_PASSWORD_PROD_LANDING` (masquee/protegee);
- `BUNNY_PULL_ZONE_ID_PROD_LANDING`;
- `BUNNY_STORAGE_ZONE_PROD_PWA=filfinance-prod-pwa`;
- `BUNNY_STORAGE_PASSWORD_PROD_PWA` (masquee/protegee);
- `BUNNY_PULL_ZONE_ID_PROD_PWA`;
- `FILFINANCE_PROD_BUNNY_DEPLOY_ENABLED=true`;
- `CLOUDFLARE_API_TOKEN_WORKERS_PROD`.

Les contrats DEV, conserves pour sa reprise explicite, sont
`PAGES_LANDING_PROJECT_DEV=filfinance-dev`,
`FILFINANCE_DEV_PWA_URL=https://app.dev.filfinance.fr`; les Storage/Pull Zones
Bunny sont vérifiées par `check_bunny_dev_release_preflight.sh`.

La cle Bunny et les deux mots de passe Storage sont masques/proteges. Le
preflight PROD lit les deux Pull Zones avec la cle Bunny, valide leurs hostnames
et ne journalise jamais leur contenu. Les CNAME Cloudflare sont inventories et
modifies manuellement apres confirmation : aucun token DNS n'est utilise par les
jobs statiques.

Le token Worker PROD doit etre un **Account API token** disposant de `Workers
Scripts: Edit` sur le compte exact; `check_cloudflare_release_preflight.sh` ne
s'applique plus qu'a un deploy Worker explicite.

Variables GitLab masquees et protegees:

- `FILFINANCE_GITLAB_READ_API_TOKEN`, project access token de role `Maintainer`
  limite a `read_api` (le role est requis pour lire les resource groups),
  scope `production/preflight` ou `*` si la configuration historique le requiert;
- `FILFINANCE_SMOKE_LISTENER_EMAIL` et `FILFINANCE_SMOKE_LISTENER_PASSWORD`;
- `FILFINANCE_SMOKE_MODERATOR_EMAIL` et `FILFINANCE_SMOKE_MODERATOR_PASSWORD`;
- `FILFINANCE_SMOKE_ADMIN_EMAIL` et `FILFINANCE_SMOKE_ADMIN_PASSWORD`.

Les six credentials smoke PROD peuvent utiliser le scope
`production/smoke`. Aucun job calendrier DEV/shadow ne reutilise ces
credentials. Ne pas elargir les credentials production.
Les trois comptes doivent etre verifies et porter leur role exact. Proteger
les branches `dev` et `main` ainsi que les environnements `development`,
`development/*`, `production` et `production/*` avant d'exposer ces variables aux jobs.
Les flags et URLs lus dans `validate`, `landing:check`, `client:web-build` et les
gates restent scopees `*` et `protected`;
les tokens deploy/smoke peuvent etre scopes par environnement.
Les variables legacy `PAGES_PROJECT_PROD` et `PAGES_PWA_PROJECT` doivent etre
absentes ou vides : la preflight les refuse.

### Archive — variables DEV

Ces variables sont conservees pour un futur environnement DEV independant. Ne
pas les activer pour EPIC-037 : elles ne servent ni de gate Rust ni de preuve
de promotion PROD.

`PAGES_LANDING_PROJECT_DEV=filfinance-dev`,
`FILFINANCE_DEV_PWA_URL=https://app.dev.filfinance.fr` reste le contrat
inactifs pendant cette pause.

- `FILFINANCE_DEV_PAGES_BOOTSTRAP_ENABLED=true` expose le job manuel
  `bootstrap:dev:pages`; conserver le flag full-stack suivant a `false` pendant
  ce bootstrap, puis supprimer/desactiver ce flag;
- `FILFINANCE_DEV_WORKER_BOOTSTRAP_ENABLED=true` expose le job manuel
  `bootstrap:dev:edge` pour creer le Worker dev greenfield; conserver les deux
  flags full-stack a `false`, puis supprimer/desactiver ce flag;
- `FILFINANCE_DEV_PAGES_DEPLOY_ENABLED=true` active le deploy Pages dev. Sans
  cette variable, la CI ne cree aucun job de release full-stack dev (backend,
  landing, PWA, Worker ou smoke);
- `CLOUDFLARE_API_TOKEN_PAGES_DEV`, requis sous ce nom pour la release dev;
- `CLOUDFLARE_API_TOKEN_WORKERS_DEV`, requis sous ce nom pour la release dev;
- `FILFINANCE_EDGE_PUBLISH_SECRET_DEV`;
- `FILFINANCE_EDGE_TICKET_SECRET_DEV`.

Quand `FILFINANCE_EDGE_PUBLISH_SECRET_DEV` et
`FILFINANCE_EDGE_TICKET_SECRET_DEV` sont definis, la CI les pousse vers les
bindings Cloudflare standards `FILFINANCE_EDGE_PUBLISH_SECRET` et
`FILFINANCE_EDGE_TICKET_SECRET` de l'environnement Wrangler `dev`. S'ils ne
sont pas definis, la CI deploie le Worker dev et conserve les secrets deja
presents dans Cloudflare.

Le WebSocket DEV `live.dev.filfinance.fr` est géré par Bunny vers Rust/Redis,
pas dans `wrangler.jsonc`, afin que la CI ne tente pas de modifier les routes
Workers avec un token limite.

Le serveur doit contenir deux fichiers hors Git derives de `.env.server.example`:

- `.env.server.prod` dans le clone prod;
- `.env.server.dev` dans le clone dev.

## Historique : deploiement serveur personnel

Le serveur personnel `sbalgo-srv` heberge deux stacks backend LAN:

- prod `main`: `/raid/app/filfinance/filfinance-prod`, port hote `8088`;
- dev `dev`: `/raid/app/filfinance/filfinance-dev`, port hote `18088`.

Traefik reste desactive par defaut sur ce palier avec `FILFINANCE_TRAEFIK_ENABLED=false`.

Topologie retenue pour GitLab:

- utiliser le runner Docker partage de `sbalgo-srv`, affecte au projet FilFinance;
- tag runner: `sbalgo-runner-docker`;
- reproduire le modele `home-srv`: GitLab CI se connecte en SSH a `sbcenter.fr`;
- `SSH_PORT=2285` redirige vers SSH sur `sbalgo-srv`;
- l'utilisateur deploy est `gitlab-ci`;
- la cle GitLab CI reste `id_rsa_home_srv_gitlab_ci`;
- le serveur cible refuse un worktree sale, fetch la branche, puis checkout en mode detached du `CI_COMMIT_SHA` exact; `sbalgo-srv` doit avoir un acces sortant GitLab.

Le token runner ne doit pas etre stocke dans le repo. Le fichier runtime
`config.toml` du runner reste gere hors du clone applicatif sur `sbalgo-srv`.

Commande historique executee par les jobs backend, avec les variables propres a
chaque environnement:

```bash
cd "$DEPLOY_PATH"
test -z "$(git status --porcelain)"
git fetch origin "$DEPLOY_BRANCH"
git checkout --detach "$CI_COMMIT_SHA"
test "$(git rev-parse HEAD)" = "$CI_COMMIT_SHA"
test -f $DEPLOY_ENV_FILE
bash scripts/migrate_server_env.sh $DEPLOY_ENV_FILE
bash scripts/check_deploy_env.sh $DEPLOY_ENV_FILE $DEPLOY_ENV
bash scripts/check_better_auth_env.sh $DEPLOY_ENV_FILE $DEPLOY_ENV "$DEPLOY_PWA_URL"
docker compose --env-file $DEPLOY_ENV_FILE -f compose.yaml -f compose.prod.yaml config
docker compose --env-file $DEPLOY_ENV_FILE -f compose.yaml -f compose.prod.yaml up -d --build --force-recreate
docker compose --env-file $DEPLOY_ENV_FILE -f compose.yaml -f compose.prod.yaml ps
bash scripts/smoke_backend_runtime.sh
curl -fsS http://127.0.0.1:$DEPLOY_BACKEND_PORT/health
```

Le job epingle la cle hote SSH attendue avec `SSH_HOST_KEY_SHA256` avant toute
connexion. Il inventorie ensuite, dans son journal protege, les metadonnees
redigees des conteneurs et volumes n8n du projet Compose cible. Seuls les
conteneurs legacy etiquetes `n8n` et `n8n-postgres` sont retires ; aucun volume
n8n n'est vise et une erreur de retrait ne peut pas etre masquee.

> **Interdite pour une promotion PROD sans interruption.** Cette commande
> recree la pile avec `--force-recreate`, y compris le backend singleton et ses
> workers. Elle peut donc couper les WebSockets et le traitement live. Elle ne
> doit pas etre jouee sur PROD pour EPIC-037 tant que le rollout API blue/green,
> le worker a leader unique et le handoff durable ne sont pas implementes et
> verifies. Elle reste decrite ici pour expliquer le comportement historique et
> le palier DEV, pas comme procedure de mise en production.

### Arret gracieux du backend Rust

Lors d'un `SIGTERM` Docker ou d'un `Ctrl-C`, le binaire Rust demarre une unique
deadline de 10 secondes. Elle cesse l'acceptation HTTP et couvre les workers
supervises. Les callbacks WebSocket `on_upgrade` ne sont pas joints par Axum :
ils peuvent donc etre coupes a la fin du processus et les clients doivent se
reconnecter. A echeance, les taches supervisees qui n'ont pas fini sont
annulees; `compose.yaml` fixe `stop_grace_period: 15s` pour que Docker ne fasse
pas un `SIGKILL` avant cette politique applicative.

Le receiver de la file live ferme alors les nouveaux envois. Les evenements
encore en memoire ne sont volontairement pas persistes pendant l'arret : les
consommateurs aval reposent sur des broadcasts sans accuse de traitement et
pourraient sinon quitter avant traduction, audio, Edge ou Telegram. C'est une
perte controlee au shutdown, pas un handoff sans perte; une outbox durable et
un rollout blue/green avec worker unique restent des prerequis avant de
l'affirmer pour PROD sans interruption.

Services deployes:

- `${FILFINANCE_SERVICE_CONTAINER_NAME}`: backend Rust sur `8088` dans le conteneur;
- `${FILFINANCE_AUTH_CONTAINER_NAME}`: Better Auth sur `3000`, prive dans le reseau Docker;
- `${FILFINANCE_GATEWAY_CONTAINER_NAME}`: gateway Nginx qui porte `${FILFINANCE_BACKEND_HOST_BIND}:${FILFINANCE_BACKEND_PORT}` et route auth/profil vers Better Auth, le reste vers Rust;
- `${FILFINANCE_BROWSER_COLLECTOR_CONTAINER_NAME}`: fallback navigateur FinancialJuice, non expose publiquement;
- `./volumes/service/data`: volume SQLite persistant.
- `./volumes/financialjuice-browser-collector/data`: profil Chromium persistant.

Le service `financialjuice_browser_collector` est permanent: Docker gere son cycle de vie,
le backend expose `/control`, et le collector reste idle tant que `fallback_active=false`.
Avec `NEWS_FINANCIAL_JUICE_BROWSER_HEADLESS=false`, l'image Playwright fournit Xvfb quand
aucun display n'est disponible. Le deploy backend normalise `.env.server.*` vers cette
valeur pour eviter qu'une ancienne configuration `headless=true` recree un collector qui
republie un statut `blocked` apres un deblocage NoMachine reussi.

Le deploiement calendrier est Rust-only : aucun fichier Compose, variable ou job
CI n8n ne fait partie de la pile active. Les volumes n8n historiques, s'ils sont
encore presents sur le serveur, ne sont ni montes, ni purges par le deploiement.
Le job prod garde `financialjuice-browser-collector` scale a `1`; ce composant
reste independant du calendrier et de n8n.

Sur le serveur actuel, Cloudflare atteint le port du gateway. Les labels Traefik de
`compose.prod.yaml` restent compatibles avec une installation Traefik future, mais ne
sont pas requis pour router Better Auth. `filfinance-api.sbcenter.fr` reste l'ancien
domaine du palier `EPIC-020` tant que la migration DNS n'est pas appliquee.

Voir [NEWS-111](./tickets/NEWS-111-deploiement-home-srv-traefik.md) pour l'audit `home-srv`, [NEWS-112](./tickets/NEWS-112-route-traefik-api-filfinance.md) pour la route Traefik API appliquee et [NEWS-113](./tickets/NEWS-113-bootstrap-serveur-premier-deploy.md) pour le bootstrap serveur.

Le smoke runtime backend est decrit dans [NEWS-124](./tickets/NEWS-124-smoke-runtime-backend-piper-hls.md). Il bloque le deploiement si `NEWS_TTS_ENABLED=true` ou `NEWS_HLS_ENABLED=true` et que Piper/HLS ne sont pas operationnels dans le conteneur.

## Bootstrap serveur

Chemin de deploiement retenu:

```text
/raid/app/filfinance/filfinance-prod
/raid/app/filfinance/filfinance-dev
```

Le clone DEV et la colonne DEV ci-dessous sont historiques pendant la pause de
l'environnement; ils ne doivent pas etre utilises pour valider EPIC-037.

Commandes initiales sur le serveur:

```bash
sudo -iu gitlab-ci git clone git@gitlab.com:simonbdbc/filfinance.git /raid/app/filfinance/filfinance-prod
sudo -iu gitlab-ci git clone git@gitlab.com:simonbdbc/filfinance.git /raid/app/filfinance/filfinance-dev
sudo -iu gitlab-ci git -C /raid/app/filfinance/filfinance-prod checkout main
sudo -iu gitlab-ci git -C /raid/app/filfinance/filfinance-dev checkout dev
sudo -iu gitlab-ci cp /raid/app/filfinance/filfinance-prod/.env.server.example /raid/app/filfinance/filfinance-prod/.env.server.prod
sudo -iu gitlab-ci cp /raid/app/filfinance/filfinance-dev/.env.server.example /raid/app/filfinance/filfinance-dev/.env.server.dev
sudo chmod 600 /raid/app/filfinance/filfinance-prod/.env.server.prod /raid/app/filfinance/filfinance-dev/.env.server.dev
```

`.env.server.example` est un template **production**. La colonne DEV ci-dessous
documente la configuration historique; ne pas l'appliquer ou la modifier pour
EPIC-037. `COMPOSE_PROJECT_NAME` seul ne namespace pas les `container_name`,
reseaux et volumes explicitement nommes:

| Variable | prod | dev |
| --- | --- | --- |
| `COMPOSE_PROJECT_NAME` | `filfinance_prod` | `filfinance_dev` |
| `FILFINANCE_ENV_FILE` | `.env.server.prod` | `.env.server.dev` |
| `FILFINANCE_{SERVICE,POSTGRES,AUTH,GATEWAY}_CONTAINER_NAME` | suffixe `_prod` | suffixe `_dev` |
| `FILFINANCE_BROWSER_COLLECTOR_CONTAINER_NAME` | `financialjuice_browser_collector_prod` | `financialjuice_browser_collector_dev` |
| `FILFINANCE_INTERNAL_NETWORK_NAME` | `filfinance_internal_prod` | `filfinance_internal_dev` |
| volumes Postgres/browser | suffixe `_prod` | suffixe `_dev` |
| routeurs/services Traefik | suffixe `-prod` | suffixe `-dev` |
| `FILFINANCE_API_HOST` | `api.filfinance.fr` | `api.dev.filfinance.fr` |
| `FILFINANCE_BACKEND_PORT` | `8088` | `18088` |
| `NEWS_EDGE_PUBLISH_URL` | `https://live.filfinance.fr/publish/live` | désactivé (`NEWS_EDGE_PUBLISH_ENABLED=false`) |
Renseigner les secrets de production dans `.env.server.prod`, puis valider sans
afficher leur contenu:

```bash
docker network inspect traefik_network
cd /raid/app/filfinance/filfinance-prod
FILFINANCE_ENV_FILE=.env.server.prod bash scripts/check_deploy_env.sh .env.server.prod prod
FILFINANCE_ENV_FILE=.env.server.prod docker compose --env-file .env.server.prod -f compose.yaml -f compose.prod.yaml config
```

Ne pas ajouter de fichier Compose ou de variable n8n a ces validations. Les
anciens conteneurs et volumes n8n sont des artefacts d'audit externes a la pile
active; toute purge exige une autorisation distincte.

Les variables `EXPO_PUBLIC_*` restent cote build GitLab et ne doivent pas etre ajoutees a `.env.server.prod` ou `.env.server.dev`.

Si Git retourne `detected dubious ownership`, corriger de preference l'ownership du dossier pour l'utilisateur de deploiement:

```bash
sudo chown -R gitlab-ci:gitlab-ci /raid/app/filfinance/filfinance-prod /raid/app/filfinance/filfinance-dev
```

Si le dossier doit rester possede par un autre utilisateur, ajouter explicitement l'exception Git pour l'utilisateur `gitlab-ci`:

```bash
git config --global --add safe.directory /raid/app/filfinance/filfinance-prod
git config --global --add safe.directory /raid/app/filfinance/filfinance-dev
```

## Archive — deploiement frontend Cloudflare Pages

Cette section decrit le palier remplace. La publication active est documentee
dans [BUNNY-PRODUCTION-RUNBOOK](./BUNNY-PRODUCTION-RUNBOOK.md) : ne pas activer
ni recreer de job Pages pour la landing ou la PWA.

### Archive — bootstrap Cloudflare DEV

DEV est en pause. Les anciens jobs et flags de bootstrap/preview DEV ne doivent
pas etre actives, et un domaine ou un smoke DEV ne valide pas une release
calendrier. La configuration historique est conservee dans l'historique Git;
une reprise DEV devra etre redocumentee et approuvee separement.

Si Cloudflare classe le domaine cible `banned domain`, conserver la release
fermee et suivre le dossier de revue
[`CF-2026-08-28-pages-banned-domain.md`](./incidents/CF-2026-08-28-pages-banned-domain.md).
Ne pas creer un CNAME seul, essayer une variante de hostname, ni supprimer et
recreer le domaine ou le projet pour contourner le classement.

Les repertoires, commandes et sorties sont portes uniquement par les jobs CI : `apps/landing`
avec `npm run build` vers `dist`, et `apps/pwa` avec `npm run build:web` vers `dist`.
Cloudflare ne reconstruit pas ces artefacts.

La landing et la PWA sont deployeees par les jobs GitLab `wrangler pages deploy`;
la Git integration Cloudflare ne doit pas declencher une seconde publication en
parallele. Les projets Pages doivent etre configures comme suit:

| Projet | Artefact GitLab uploade | Branche de production | Domaine |
| --- | --- | --- | --- |
| `filfinance-prod` | `apps/landing/dist` | `main` | `filfinance.fr` |
| `filfinance-app-prod` | `apps/pwa/dist` | `main` | `app.filfinance.fr` |
| `filfinance-dev` | `apps/landing/dist` | `dev` | `dev.filfinance.fr` |
| Bunny Storage PWA DEV | `apps/pwa/dist` | `dev` | `app.dev.filfinance.fr` |

Les variables Pages listees ci-dessous sont un contrat **archive**. Elles sont
retirees apres suppression des projets Pages; le contrat actif est compose des
variables `BUNNY_*_PROD` de
[BUNNY-PRODUCTION-RUNBOOK](./BUNNY-PRODUCTION-RUNBOOK.md). L'URL PWA dev ne doit
etre ajoutee aux origines Better Auth qu'avant une remise en service DEV explicite.

Le job `release:dev-lock` ne valide que les domaines DEV; la promotion PROD
valide separement `filfinance.fr` et `app.filfinance.fr`. Pour chaque cible, le
preflight exige une branche de production exacte (`main` en PROD, `dev` en DEV),
un domaine Pages actif et verifie, puis un CNAME unique et proxifie vers
`<projet>.pages.dev`, sans prefixe `dev.`. Tous les uploads Pages sont des
uploads de production sans `--branch`; en particulier, `--branch=dev` creerait
un apercu et est interdit. Le preflight s'execute dans le stage `validate`, donc
un domaine pending, une branche incorrecte ou un token incapable de lire la
ressource attendue bloque le pipeline avant le backend.

DEV reste toutefois en pause : ses jobs, DNS, backend, Worker et smokes ne sont
pas lances tant qu'une decision de reprise n'est pas prise. La politique
Cloudflare `banned domain` est bloquante et ne doit pas etre contournee.

Les cinq jobs de release d'un environnement partagent un `resource_group`. Dans
GitLab, configurer `development` et `production` avec `process_mode=oldest_first`;
la gate CI le verifie par API avant le premier changement externe. Cela empeche
deux pipelines de melanger backend, Pages, Worker et smoke.

Configuration unique a effectuer avec un token operateur `api`, utilise par un
Maintainer et jamais stocke dans le repo. `GITLAB_PROJECT_ID` est l'ID numerique
du projet (pas une variable supposee presente dans un shell local):

```bash
export GITLAB_API_URL=https://gitlab.com/api/v4
: "${GITLAB_PROJECT_ID:?Renseigner l'ID numerique du projet}"
: "${GITLAB_OPERATOR_API_TOKEN:?Renseigner le token operateur api}"

curl --fail-with-body --request PUT \
  --header "PRIVATE-TOKEN: $GITLAB_OPERATOR_API_TOKEN" \
  --data "process_mode=oldest_first" \
  "$GITLAB_API_URL/projects/$GITLAB_PROJECT_ID/resource_groups/development"
curl --fail-with-body --request PUT \
  --header "PRIVATE-TOKEN: $GITLAB_OPERATOR_API_TOKEN" \
  --data "process_mode=oldest_first" \
  "$GITLAB_API_URL/projects/$GITLAB_PROJECT_ID/resource_groups/production"

for group in development production; do
  curl --fail-with-body --silent --show-error \
    --header "PRIVATE-TOKEN: $GITLAB_OPERATOR_API_TOKEN" \
    "$GITLAB_API_URL/projects/$GITLAB_PROJECT_ID/resource_groups/$group" \
    | jq -e '.process_mode == "oldest_first"' >/dev/null
done
```

Le token CI `FILFINANCE_GITLAB_READ_API_TOKEN` reste distinct, masque/protege et
limite a `read_api`; il controle les verrous de `resource_group` sans les
modifier. Il ne valide ni un smoke DEV ni une promotion calendrier.

### Archive — observation calendrier DEV/shadow

Les jobs, scripts et workflows d'observation n8n/Rust ont ete retires du depot
et de la CI. Ils ne doivent pas etre recrees ni executes : DEV reste en pause,
et aucun resultat de shadow, de parite ou de `not_produced` ne constitue une
gate de migration. La preuve unique est le test de recuperation Rust du SHA
`main` exact. Les donnees et volumes n8n historiques sont conserves hors du
chemin actif, sans purge automatique.

Chaque push sur `main` valide le repo seulement. Une publication frontend PROD
exige une pipeline `web` ou `api` explicitement demandee avec
`FILFINANCE_PROD_RELEASE_REQUESTED=true`, les verrous de deploiement, puis le
declenchement manuel des jobs Bunny. Les jobs backend et Edge restent separes.

Ne pas configurer `npx wrangler deploy` comme deploy frontend : Bunny recoit les
artefacts deja construits par la CI.

## Archive — preview `dev`

La preview DEV est en pause. Ne pas la deployer, ne pas lancer de smoke DEV et
ne pas utiliser son SHA pour valider ou promouvoir le calendrier. Toute remise
en service future est une decision d'exploitation separee de EPIC-037.

## Production sur `main`

`main` deploie le backend serveur, la landing `filfinance.fr`, la PWA
`app.filfinance.fr` et, lorsque configure, le Worker `live.filfinance.fr`.

Pour `EPIC-037`, appliquer la sequence specifique documentee dans
`PRODUCTION-FINAL-RUNBOOK.md` : tag de la release precedente, test Rust de
recuperation reussi localement puis en CI, push de `main` et deploiement manuel
du SHA Rust-only. n8n est deja retire du depot et de la CI; ses volumes
historiques ne sont pas purges.

Prerequis de promotion:

- le test Rust de recuperation complet du SHA `main` a reussi localement;
- la CI Rust du meme SHA est verte;
- `FILFINANCE_GITLAB_READ_API_TOKEN`, s'il est utilise pour les verrous CI,
  est masque/protege et limite a `read_api`;
- les environnements GitLab `production/*` et la branche `main` sont proteges.

Procedure:

1. verifier `git status --short`;
2. executer les validations locales Rust, dont le test de recuperation complet;
3. poser un tag sur la release actuellement deployee, cible de rollback;
4. pousser le SHA exact sur `main` et attendre la CI Rust de ce SHA;
5. jouer les jobs de deploiement PROD manuels seulement apres cette gate;
6. attendre `smoke:main` et executer les controles de livraison ci-dessous.

Quand `FILFINANCE_PROD_DEPLOY_ENABLED=true` et
`FILFINANCE_PROD_BUNNY_DEPLOY_ENABLED=true`, verifier aussi:

- `deploy:main:bunny-landing`;
- `deploy:main:bunny-pwa`;
- `smoke:main:frontends` apres la bascule DNS;
- `deploy:main:edge`;
- `smoke:main`.

Smoke tests minimaux apres `EPIC-028`:

- `https://filfinance.fr` retourne 200 et ne charge pas le bundle Expo;
- `https://app.filfinance.fr` retourne 200 et charge les assets Expo;
- une route profonde PWA retourne l'app Web grace au fallback Bunny `/index.html`;
- l'ecran auth charge sans erreur de configuration Better Auth;
- `https://api.filfinance.fr/health` retourne 200;
- `wss://api.filfinance.fr/ws/news` est joignable par le client.
- `bash scripts/smoke_backend_runtime.sh` passe sur le serveur apres `docker compose up`.

Suite smoke complete (les six credentials des comptes listener/moderator/admin sont
decrits dans `AUTH-RUNBOOK.md`):

```bash
FILFINANCE_FRONTEND_URL=https://filfinance.fr \
FILFINANCE_PWA_URL=https://app.filfinance.fr \
FILFINANCE_API_URL=https://api.filfinance.fr \
FILFINANCE_EDGE_URL=https://live.filfinance.fr \
FILFINANCE_CHECK_EDGE=1 \
bash scripts/run_deployed_smoke_suite.sh
```

## Domaine custom et HTTPS

Les domaines proposes sont:

- `filfinance.fr` comme domaine production de `main`;
- `app.filfinance.fr` comme domaine PWA production de `main`;
- `api.filfinance.fr` comme domaine API/WebSocket production;
- `live.filfinance.fr` comme domaine Worker edge production;
- `dev.filfinance.fr` comme domaine landing de `dev`;
- `app.dev.filfinance.fr` comme domaine PWA Bunny de `dev`, avec
  `FILFINANCE_DEV_PWA_URL=https://app.dev.filfinance.fr`.

Pre-requis domaine: `filfinance.fr` reste achete chez OVH, mais la zone DNS doit
etre ajoutee a Cloudflare et les nameservers OVH doivent pointer vers Cloudflare
avant le go-live. Les deux CNAME frontend Bunny sont DNS-only; API et live
conservent leur topologie Cloudflare existante.

Procedure cible:

1. provisionner les deux Storage/Pull Zones Bunny et leurs hostnames, sans toucher
   a DEV tant que cet environnement reste en pause;
2. faire pointer `api.filfinance.fr` vers le port d'origine de `filfinance-gateway` sur le serveur personnel;
3. limiter ce port a l'origine Cloudflare avec le pare-feu serveur;
4. aligner `APP_WEB_URL`, le preflight Better Auth et les origines autorisees sur les URLs PWA;
5. verifier HTTPS sur les deux hostnames Bunny, l'API et le Worker edge;
6. verifier les WebSockets derriere Cloudflare et `filfinance-gateway`.

Ne supprimer les projets Pages historiques qu'apres activation Bunny, sauvegarde
des CNAME, smokes valides et confirmation explicite. Apres suppression, il n'y
a plus de rollback Pages instantane.

## Rollback

Le rollback Git normal est un commit `git revert` du ou des commits de release,
suivi du meme pipeline protege. Pour une urgence frontend Bunny, republier le
build precedent puis purger la Pull Zone ou restaurer les CNAME sauvegardes
avant la suppression Pages. Ne pas reecrire l'historique de `main` et ne pas
supprimer les volumes serveur.

Rollback simple serveur:

1. identifier le dernier commit stable;
2. revenir temporairement a ce commit sur le serveur ou pousser un revert;
3. executer les smoke tests rapides;
4. pousser ensuite un correctif sur `main` si necessaire.

Si l'incident n'est pas trivial, ouvrir un ticket avec:

- cause probable;
- impact utilisateur;
- commit concerne;
- action corrective.

## Cible production finale

Avant une production plus stricte, durcir le process:

- suivre [PRODUCTION-FINAL-RUNBOOK](./PRODUCTION-FINAL-RUNBOOK.md);
- smoke tests post-deploiement automatises;
- rollback documente et teste;
- image Docker contenant le runtime TTS/Piper si l'audio serveur doit etre actif;
- eventuel retour de Cloudflare Durable Objects/CDN pour scaler metadata et audio.

## Checklist EPIC-020 actuelle

- `.gitignore` racine present;
- README racine present;
- depot GitLab pousse sur `main`;
- pipeline GitLab de validation present;
- Node projet pinne en `24.13.1`;
- deploiement frontend Cloudflare Pages documente;
- deploiement backend serveur personnel documente;
- workflow sans MR obligatoire documente;
- route Traefik backend Fil Finance appliquee dans `home-srv/traefik-docker`;
- bootstrap serveur et premier deploy backend documentes;
- script smoke post-deploiement `scripts/smoke_deploy.sh` present;
- rollback serveur documente.
