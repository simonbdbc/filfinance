# Architecture landing et PWA

## Structure

- `apps/landing` est le site public Astro. Il produit uniquement des fichiers statiques dans `apps/landing/dist` et ne dépend d'aucun build de la PWA.
- `apps/pwa` est l'application Expo Router existante. Ses routes, dépendances, variables d'environnement et scripts sont conservés.
- Les services backend restent dans leurs dossiers historiques.

Chaque frontend possède son propre `package.json`, son `package-lock.json`, ses assets et sa validation. Le `package.json` racine fournit seulement des raccourcis; il ne fusionne pas les dépendances.

## Développement et validation

```bash
npm --prefix apps/landing ci
npm --prefix apps/pwa ci
npm run dev:landing
npm run dev:pwa
npm run check:landing
npm run lighthouse:landing
npm run check:pwa
npm run build:landing
npm run build:pwa
```

La landing écoute par défaut sur `http://localhost:4321`. Le service Docker local `filfinance-client` continue d'exposer la PWA sur `http://localhost:8052`, avec un contexte de build déplacé vers `apps/pwa`.

La landing sert du HTML/CSS statique avec un seul script applicatif :
`apps/landing/src/scripts/theme.js`, intégré dans le `head` avant le contenu.
Il restaure `filfinance.landing.theme` (`light` ou `dark`), puis active la
commande de thème. Le clair reste le défaut, y compris sans JavaScript ou
si la lecture du stockage échoue. Une écriture refusée n'annule pas une
bascule pendant la visite. Aucune préférence PWA, API ou authentification
n'intervient. Le validateur compare exactement ce script à sa source locale
et refuse les autres scripts applicatifs ; JSON-LD reste autorisé.
Voir le [référentiel des thèmes](./design/landing/README.md).

## Routes et déploiement

- Landing production: `https://filfinance.fr/`.
- PWA production: `https://app.filfinance.fr/`.
- Connexion PWA: `https://app.filfinance.fr/login`.
- Inscription: pas de route publique distincte; l'action reste intégrée à l'écran de connexion de la PWA.

La topologie frontend utilise Bunny Storage + Pull Zones pour les deux sites
statiques. Cloudflare reste uniquement l'autorite DNS et conserve API, live et
messagerie : les deux CNAME frontend sont DNS-only.

| Environnement | Landing | PWA | Branche source |
| --- | --- | --- | --- |
| PROD | Bunny `filfinance-prod-landing` → `filfinance.fr` | Bunny `filfinance-prod-pwa` → `app.filfinance.fr` | `main` |
| DEV | Bunny Storage → `dev.filfinance.fr` | Bunny Storage → `app.dev.filfinance.fr` | `dev` |

GitLab reste l'unique emetteur des deux Storage Zones de production et de DEV.
La PWA PROD possede un fallback Bunny `/index.html` avec HTTP 200; la landing
n'en a pas. Les en-tetes de la landing sont des Edge Rules Bunny, et non un
fichier `_headers` Pages. `FILFINANCE_DEV_PWA_URL` vaut exactement
`https://app.dev.filfinance.fr`.
Cette origine same-site est necessaire au cookie Better Auth `SameSite=Lax`
avec `api.dev.filfinance.fr`; un domaine tiers ne convient pas.

DEV est le pilote Bunny réel: DNS délégué, frontends Storage, API/HLS/live via Pull
Zones et fan-out Redis. Cloudflare ne reçoit aucun trafic DEV; la production reste inchangée.
Le détail des certificats, Shield et rollback est dans `BUNNY-DEV-RUNBOOK.md`.

Le pipeline construit et valide les deux artefacts séparément. Le job
`landing:check` exécute le typecheck, les tests du thème et du validateur,
le build statique, puis la validation de production. `lighthouse:ci` couvre
trois audits mobile et trois desktop **pour chacun des deux thèmes**, soit
douze runs. La valeur pessimiste des trois runs de chaque configuration doit
rester supérieure ou égale à `0,80` en performance et à `1,00` en accessibilité,
bonnes pratiques et SEO. Les jobs `client:typecheck` et `client:web-build`
préservent les contrôles Expo existants depuis `apps/pwa`.

Le premier audit mobile lance aussi la recette navigateur et ses huit
captures via `scripts/lighthouse-browser-qa.cjs`. Les configurations sombres
utilisent `scripts/lighthouse-dark.cjs` pour préparer le stockage avant les
audits, conserver cette préférence et vider le cache réseau. Les mêmes
seuils s'appliquent ; aucun mode de test n'est ajouté à l'URL publique.
`CHROME_PATH` peut désigner l'exécutable Chromium local (fourni par la CI).
Les rapports complets se trouvent dans `apps/landing/reports/` et restent des
preuves locales ou des artefacts CI, distincts d'une recette de déploiement.

## Pages légales partagées

La landing génère quatre documents HTML publics avec slash final : mentions légales,
confidentialité, CGU et cookies. Le layout, les thèmes, l’en-tête et le footer sont
communs ; les liens de services depuis ces documents ciblent `/#section`.
Les cinq informations `PUBLIC_LEGAL_*` sont décrites dans le
[manifeste Fil Finance](./guides/legal-landing-app/MANIFESTE.md).

La PWA lit `EXPO_PUBLIC_LANDING_ORIGIN` via son lecteur de configuration publique
et construit les quatre liens absolus dans `src/shared/legal`. Le prébuild refuse
une origine absente, avec chemin ou identifiants, ou HTTP hors loopback. Les
contrôles d’artefact DEV/PROD vérifient aussi l’origine attendue. La surcharge Web
existante reste disponible ; une valeur invalide rend les documents indisponibles
avec un contact de secours, sans destination implicite d’un autre environnement.

Les liens apparaissent avant création du compte, dans le cadre commun des écrans
d’authentification et dans le profil. Web ouvre un nouvel onglet ; le natif utilise
`Linking.openURL` avec message d’échec. Aucune session n’est partagée avec la landing.

`landing:check` impose `PUBLIC_LEGAL_PUBLICATION_ENFORCED=true` et contrôle les
cinq informations, les quatre fichiers, leurs métadonnées, le maillage, le sitemap
et le script exact de thème. Les jobs de publication PWA attendent aussi ce job,
sans changement des déclencheurs manuels. La recette navigateur ajoute 32 rendus
légaux ; la recette PWA simulée est activable localement. Voir les
[commandes et preuves](./guides/legal-landing-app/RECETTE-FILFINANCE.md).

## Points de vigilance

- Provisionner les deux Storage/Pull Zones Bunny PROD, leurs hostnames et les
  certificats avant la bascule CNAME. Voir
  [BUNNY-PRODUCTION-RUNBOOK](./BUNNY-PRODUCTION-RUNBOOK.md).
- Autoriser `filfinance://` et seulement la PWA du même environnement dans Better Auth; ne jamais réunir les origines prod et dev.
- Ne pas servir la PWA sous un sous-chemin sans valider Expo Router, ses assets absolus et tous les deep links.
- Conserver les routes Expo actuelles lors des évolutions de la landing.

## Authentification et migration

`APP_WEB_URL` est l'URL de la PWA, car les liens Better Auth de verification et de
reinitialisation y reviennent. En production, il doit donc valoir
`https://app.filfinance.fr`; en developpement, il doit reprendre exactement
`FILFINANCE_DEV_PWA_URL`. Chaque fichier serveur doit contenir `filfinance://` et
uniquement l'origine PWA de son environnement dans `BETTER_AUTH_TRUSTED_ORIGINS`;
les origines landing ne sont ajoutees que lorsqu'elles sont autorisees a appeler
l'API du meme environnement.

Le preflight Better Auth PROD est autonome et utilise l'origine PWA PROD. Lors
d'une remise en service DEV explicite seulement, il recoit
`FILFINANCE_DEV_PWA_URL`, refuse les anciennes URLs landing comme callback et
refuse les origines d'un autre environnement avant tout deploiement. Les fichiers
serveur doivent etre alignes avant d'activer les jobs.

La migration est en cours dans le working tree : l'ancien dossier
`filfinance-client` est deplace vers `apps/pwa` et la landing est ajoutee dans
`apps/landing`. Les anciennes references a `filfinance-client` dans les tickets et
EPICs clos decrivent l'arborescence historique; les documents de reference ci-dessus
decrivent la cible active.

## Socle visuel de la PWA — EPIC-039

Depuis la refonte locale des 16–17 septembre 2026, Expo Router utilise gluestack-ui v5,
UniWind et Tailwind CSS 4. Les primitives sont isolées dans `shared/ui/primitives`,
les compositions communes dans `shared/ui` et les fonctionnalités conservent leurs
hooks, API et permissions. `global.css` est l’unique source des tokens des deux thèmes.
Le thème local ne se synchronise plus avec le profil ; le runtime et le lecteur audio
restent montés dans le cadre connecté. NativeWind, les palettes concurrentes et les
adaptateurs remplacés sont supprimés.

Voir [le guide technique](./guides/pwa-ui-stack.md), [le référentiel visuel](./design/pwa/README.md)
et [la recette locale](./design/pwa/recette/2026-09-16/README.md).
