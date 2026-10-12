# Passage production final Fil Finance

## Portee

Ce runbook assemble la bascule finale sans dupliquer les procedures specialisees:

- auth et PostgreSQL: [MIGRATION-BETTER-AUTH-POSTGRES](./MIGRATION-BETTER-AUTH-POSTGRES.md);
- exploitation auth: [AUTH-RUNBOOK](./AUTH-RUNBOOK.md);
- variables: [ENVIRONMENT-FILES](./ENVIRONMENT-FILES.md);
- CI/deploiement: [GITLAB-RUNBOOK](./GITLAB-RUNBOOK.md).
- publication des frontends Bunny: [BUNNY-PRODUCTION-RUNBOOK](./BUNNY-PRODUCTION-RUNBOOK.md).

## Migration calendrier Rust (EPIC-037)

**Statut : le chemin calendrier canonique est Rust natif sur `main`; n8n a ete
retire du depot et de la CI.** La sequence de livraison est : developpement sur
`main`, reussite locale puis CI du test
`native_calendar_recovery_e2e_is_idempotent_and_keeps_live_news_unblocked` sur
le SHA exact, tag de la release precedente, push `main`, puis deploiement PROD
manuel.

Ce test est l'unique gate fonctionnelle calendrier. Aucune campagne DEV,
periode d'observation, parite de contenu n8n ou `smoke:dev` ne le remplace. Les
smokes PROD usuels sont des controles de livraison apres cette gate, pas une
seconde validation semantique. DEV reste en pause. Les volumes n8n historiques
ne sont pas purges; les contrats publics, admin, WebSocket et HLS restent
inchanges par la migration.

## Architecture de production

- landing Astro: `https://filfinance.fr`;
- frontend Expo/PWA: `https://app.filfinance.fr`;
- API publique unique: `https://api.filfinance.fr`;
- `filfinance-gateway` porte le port d'origine et route `/api/auth/*` et `/api/me` vers `filfinance-auth`;
- `filfinance-gateway` route le reste, WebSocket compris, vers le data plane Rust;
- PostgreSQL n'est present que sur le reseau Docker;
- HLS Bunny: `https://media.filfinance.fr`;
- metadata Edge: `https://live.filfinance.fr`.

La landing et la PWA sont des sites statiques Bunny. Leur publication est une
operation independante : elle ne redemarre ni le backend OVH ni le Worker Edge.
Cloudflare conserve le DNS, les services API/live et la messagerie; les deux
CNAME frontend sont DNS-only. Better Auth reste limite a
`https://app.filfinance.fr`, jamais a `https://filfinance.fr`.

## Preflight

```bash
cd /raid/app/filfinance/filfinance-prod
bash scripts/migrate_server_env.sh .env.server.prod
bash scripts/check_better_auth_env.sh .env.server.prod prod
docker compose --env-file .env.server.prod -f compose.yaml -f compose.prod.yaml config >/dev/null
```

Attendus avant activation:

- fichier `.env.server.prod` lisible et modifiable par `gitlab-ci`;
- secrets PostgreSQL/Better Auth/SMTP sans placeholder;
- `FILFINANCE_GATEWAY_CONTAINER_NAME` unique et port d'origine libre pour le gateway;
- `BETTER_AUTH_URL`, issuer et audience egaux a `https://api.filfinance.fr`;
- export utilisateurs disponible;
- sauvegardes PostgreSQL et SQLite testees; les volumes n8n historiques sont
  inventories et conserves sans purge;
- pour `EPIC-037`, preuve locale du test de recuperation Rust du SHA exact;
  la CI rejoue la meme preuve. Aucune validation DEV ou campagne de parite n8n
  n'est requise.

## Activation

1. Configurer les variables frontend:

```env
EXPO_PUBLIC_FILFINANCE_WS_URL=wss://api.filfinance.fr/ws/news
EXPO_PUBLIC_FILFINANCE_EDGE_WS_URL=wss://live.filfinance.fr/ws/live
EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL=https://media.filfinance.fr
EXPO_PUBLIC_FILFINANCE_PWA_URL=https://app.filfinance.fr
EXPO_PUBLIC_API_URL=https://api.filfinance.fr
EXPO_PUBLIC_BETTER_AUTH_URL=https://api.filfinance.fr
```

2. Activer dans GitLab:

```text
FILFINANCE_BETTER_AUTH_DEPLOY_ENABLED=true
FILFINANCE_PROD_DEPLOY_ENABLED=true
FILFINANCE_PROD_BUNNY_DEPLOY_ENABLED=true
FILFINANCE_PROD_PWA_URL=https://app.filfinance.fr
FILFINANCE_GITLAB_READ_API_TOKEN=<token-read_api-protege>
```

`FILFINANCE_GITLAB_READ_API_TOKEN` est un project access token protege, de role
`Maintainer` et scope `read_api`, utilise uniquement pour verifier le verrou
`production` et le HEAD de `main`. Le resource group `production` doit etre
configure en mode `oldest_first`; la gate Rust-only ne lit ni pipeline ni
resource group DEV.

3. Executer le test de recuperation Rust localement sur le SHA a livrer, poser
   un tag sur la release actuellement deployee, puis pousser ce SHA sur `main`.
4. Attendre la CI Rust de ce SHA et lancer les jobs PROD manuels. Pour une
   publication frontend seule, lancer uniquement les jobs Bunny et le smoke
   frontend apres la bascule DNS/certificats; ne pas lancer backend ou Edge.
   Ne pas lancer de seconde migration manuelle pendant le demarrage; les verrous
   SQLite et les leases calendrier gerent l'unicite.

## Smokes

Pour un changement statique Bunny, lancer d'abord :

```bash
FILFINANCE_FRONTEND_URL=https://filfinance.fr \
FILFINANCE_PWA_URL=https://app.filfinance.fr \
bash scripts/smoke_deploy.sh frontend
```

Verifier ensuite CORS Better Auth (racine refusee, `app` autorisee) et relancer
sur le VPS `bash scripts/check_better_auth_env.sh /srv/filfinance/.env.server.prod prod`.
Les tests utilisateur inscription, verification e-mail et reset se font depuis
`https://app.filfinance.fr`.

```bash
FILFINANCE_FRONTEND_URL=https://filfinance.fr \
FILFINANCE_PWA_URL=https://app.filfinance.fr \
FILFINANCE_API_URL=https://api.filfinance.fr \
FILFINANCE_EDGE_URL=https://live.filfinance.fr \
FILFINANCE_CHECK_EDGE=1 \
bash scripts/run_deployed_smoke_suite.sh
```

La commande exige les six variables de credentials des comptes smoke listener,
moderator et admin decrites dans le runbook Better Auth; aucun role n'est saute.

Checks manuels:

- creation et verification d'un compte;
- restauration de session apres rechargement;
- reset mot de passe et deconnexion;
- profil et theme persistants;
- routes listener/moderator/admin;
- flux WebSocket, historique, replay et ticket HLS;
- aucune ancienne variable auth dans le bundle frontend.

## Observation

Pendant 48 heures, surveiller:

- erreurs `filfinance-auth`, PostgreSQL, SMTP et JWKS;
- etat `healthy` de `filfinance-gateway` et erreurs de routage vers auth/Rust;
- taux de `401/403/5xx`;
- comptes importes, comptes en erreur et demandes de reset;
- redemarrages conteneurs et temps de migration;
- flux live/audio/HLS non regresses.

## Etat n8n et operations de bascule

Le depot et la CI ne contiennent plus de workflow, route, configuration ou job
n8n actif. Avant le premier deploiement Rust-only, proceder ainsi :

1. Poser un tag sur la release actuellement deployee, qui sera la cible de
   rollback.
2. Si une instance n8n historique existe encore hors du depot, inventorier ses
   workflows calendrier et `filfinance_news_error_alert` (identifiant, nom,
   version, statut) et exporter cet inventaire dans un emplacement protege, sans
   credentials ni secrets. Le job backend imprime aussi dans son journal protege
   un inventaire redige des conteneurs et volumes du projet Compose cible, puis
   ne retire que les conteneurs etiquetes `n8n` et `n8n-postgres`. Ne pas la
   redemarrer pour cette migration.
3. Deployer la pile Rust-only du SHA teste et verifier les routes
   publiques/admin, le flux WebSocket admin, les manifests HLS et la livraison
   d'une live news.
4. Ne purger ni le conteneur, PostgreSQL ou les volumes n8n historiques. Toute
   suppression de volume releve d'une autorisation distincte.
5. Conserver les migrations SQLite appliquees et les colonnes historiques
   `n8n_*` nullables. Elles preservent la compatibilite admin et ne doivent pas
   etre purges lors de cette bascule.

## Rollback

1. Redeployer le tag de release precedent ou le commit backend/frontend stable.
2. Ne supprimer ni les volumes PostgreSQL/n8n ni les tables
   `app.auth_identity_migration` ou les colonnes SQLite historiques `n8n_*`.
3. Si le rollback du cutover calendrier l'exige, restaurer les workflows depuis
   l'export protege de l'etape precedente, avec une decision operateur explicite.
4. Exporter les comptes crees pendant la fenetre avant tout retour temporaire.
5. Relancer les smokes public/HLS et consigner l'incident.

## Go / No-Go

Go uniquement si le test de recuperation Rust du SHA exact a reussi localement
et en CI, puis si le preflight, SMTP, migrations, smokes auth, WebSocket et
rollback sont valides. Tout placeholder, fichier serveur inaccessible, export
absent ou divergence issuer/audience impose un No-Go. Un resultat DEV ou n8n ne
peut ni compenser ni remplacer cette gate.
