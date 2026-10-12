# Publication Bunny des frontends de production

## Portee et invariants

Cette procedure publie uniquement les deux artefacts statiques issus de `main` :

| Surface | URL publique | Storage Zone / Pull Zone Bunny |
| --- | --- | --- |
| Landing Astro | `https://filfinance.fr` | `filfinance-prod-landing` |
| PWA Expo | `https://app.filfinance.fr` | `filfinance-prod-pwa` |

Cloudflare conserve la zone DNS, `api.filfinance.fr`, `live.filfinance.fr` et la
messagerie. `media.filfinance.fr` reste sur sa Pull Zone Bunny HLS existante et
ne fait pas partie de cette procedure. La landing ne contacte pas Better Auth;
`APP_WEB_URL` et l'unique origine Web Better Auth restent
`https://app.filfinance.fr`.

Le workflow part toujours de `origin/main`. Une publication des fichiers
statiques ne redemarre ni le backend OVH ni le Worker Edge.

## Provisionnement Bunny hors depot

1. Creer les Storage Zones `filfinance-prod-landing` et `filfinance-prod-pwa`.
2. Creer les Pull Zones homonymes, chacune liee a sa Storage Zone respective.
   Relever le hostname `*.b-cdn.net` et l'identifiant numerique de chaque Pull
   Zone.
3. Ajouter `filfinance.fr` a la Pull Zone landing et `app.filfinance.fr` a la
   Pull Zone PWA. Activer Free SSL pour les deux hostnames. Les certificats
   seront actifs apres la delegation DNS.
4. Sur la PWA seulement, configurer l'erreur 404 vers `/index.html` avec une
   reponse HTTP 200. Ne pas configurer de fallback SPA sur la landing.
5. Reproduire dans les Edge Rules Bunny de la landing les en-tetes auparavant
   definis dans `apps/landing/public/_headers` :
   `X-Content-Type-Options: nosniff`,
   `Referrer-Policy: strict-origin-when-cross-origin`,
   `X-Frame-Options: DENY` et
   `Permissions-Policy: camera=(), microphone=(), geolocation=()`.
   Configurer un cache HTML court/revalidable et un cache `public,
   max-age=31536000, immutable` pour les assets hashes Astro/Expo et les
   images statiques. Les regles doivent etre examinees dans le dashboard : elles
   ne sont pas versionnables dans un fichier Pages.

Ne saisir aucune cle Bunny dans le depot, un artefact ou une variable publique.

## Variables GitLab production

Creer des variables **protected** (scope `production/*` ou `*`) :

| Variable | Masquee |
| --- | --- |
| `BUNNY_API_KEY_PROD` | oui |
| `BUNNY_STORAGE_ZONE_PROD_LANDING=filfinance-prod-landing` | non |
| `BUNNY_STORAGE_PASSWORD_PROD_LANDING` | oui |
| `BUNNY_PULL_ZONE_ID_PROD_LANDING` | non |
| `BUNNY_STORAGE_ZONE_PROD_PWA=filfinance-prod-pwa` | non |
| `BUNNY_STORAGE_PASSWORD_PROD_PWA` | oui |
| `BUNNY_PULL_ZONE_ID_PROD_PWA` | non |
| `FILFINANCE_PROD_BUNNY_DEPLOY_ENABLED=true` | non |
| `CLOUDFLARE_ACCOUNT_ID` | non |

Conserver `FILFINANCE_PROD_PWA_URL=https://app.filfinance.fr`,
`FILFINANCE_PROD_DEPLOY_ENABLED=true` et les verrous Better Auth existants.
`scripts/check_bunny_prod_release_preflight.sh` valide les deux Pull Zones, les
hostnames et les variables avant upload sans imprimer de secret.

## Publication et bascule DNS

1. Lancer une pipeline GitLab protegee `web` ou `api` sur `main`, avec
   `FILFINANCE_PROD_RELEASE_REQUESTED=true`.
2. Attendre `release:prod-lock`, puis lancer manuellement
   `deploy:main:bunny-landing` et `deploy:main:bunny-pwa`. Les deux jobs
   synchronisent Storage, publient `index.html` en dernier et purgent
   uniquement leur Pull Zone.
3. Verifier d'abord les deux builds sur leurs hostnames `*.b-cdn.net` : landing
   Astro sans `_expo`, PWA Expo avec routes profondes grace au fallback.
4. Juste avant la modification, inventorier dans Cloudflare les enregistrements
   existants `@` et `app` (type, cible, TTL, proxification et identifiant).
   Creer un tag de recuperation de `main` et conserver cet inventaire hors des
   secrets.
5. Apres validation explicite, remplacer uniquement les deux enregistrements
   par des CNAME **DNS-only** vers les hostnames Bunny releves :

   ```text
   @    CNAME  <landing>.b-cdn.net
   app  CNAME  <pwa>.b-cdn.net
   ```

   Cloudflare aplati le CNAME d'apex si necessaire. Ne modifier ni API, live,
   media, mail, Bunny HLS, ni les autres enregistrements.
6. Attendre l'etat SSL actif des deux hostnames Bunny et lancer manuellement
   `smoke:main:frontends`.

Le smoke exige une landing Astro sans bundle `_expo`, une PWA au root, `/profile`
et `/email-verification`. Il ne redemarre aucun service. Ensuite, verifier que
l'origine `https://filfinance.fr` est refusee par CORS Better Auth et que
`https://app.filfinance.fr` est autorisee; executer aussi le preflight Better
Auth sur le VPS.

Les tests reels creation de compte, verification e-mail et reset de mot de
passe sont realises depuis la PWA par l'operateur.

## Nettoyage et retour arriere

Apres recette et confirmation explicite, detacher les domaines puis supprimer
les projets Pages `filfinance-prod`, `filfinance-app-prod`, `filfinance-pwa`,
`filfinance-dev`, `filfinance-app-dev` et `fil-finance`. Retirer ensuite les
variables Pages `CLOUDFLARE_API_TOKEN_PAGES_PROD` et `PAGES_*_PROJECT_*`.

Avant cette suppression, le rollback immediat consiste a restaurer les deux
enregistrements Cloudflare sauvegardes et leur mode proxy. Apres suppression des
Pages, il n'existe plus de rollback Pages instantane : recreer les projets depuis
le tag de recuperation ou republier un build precedent dans Bunny. Une erreur
frontend se corrige preferentiellement en restaurant le manifeste/bundle
precedent dans la Storage Zone puis en purgeant la Pull Zone concernee.
