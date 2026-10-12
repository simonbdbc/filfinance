# Fil Finance

`Fil Finance` regroupe la landing publique Astro, la PWA Expo Web, le backend Rust et la documentation produit/technique.

## Structure

- `apps/landing`: site public Astro statique et SEO, avec un unique script local pour le choix clair/sombre.
- `apps/pwa`: application Expo/React Native Web `filfinance`, déplacée sans changement fonctionnel.
- `filfinance-service`: backend Rust, ingestion FinancialJuice, pipeline news/audio, traduction, TTS, API et WebSockets actuels.
- `financialjuice-browser-collector`: collecteur navigateur Playwright separe pour le MVP FinancialJuice quand le site exige un vrai navigateur.
- `docs`: roadmap, EPICs, tickets, runbooks et schemas SQL.

## Statut et provenance

Le développement reste sur [GitLab](https://gitlab.com/simonbdbc/filfinance),
actuellement privé. La future destination GitHub recevra un export filtré avec
historique neuf ; aucune publication ou synchronisation n'est attestée ici.
Les sources de calendrier/audio et les modèles ont leurs propres droits : les
accès techniques ne donnent pas une autorisation de redistribution.

```bash
git clone https://gitlab.com/simonbdbc/filfinance.git
cd filfinance
```

## Architecture des configurations suivies

- La CI suivie distingue Cloudflare Pages pour les frontends PROD et Bunny Storage/Pull Zones pour DEV ; ce choix de configuration ne vérifie pas le déploiement actuel.
- Le gateway Nginx expose le port d’origine en loopback par défaut. compose.prod.yaml configure aussi des routes Traefik directes vers Better Auth et Rust, sur un réseau externe : le gateway n’est donc pas une entrée exclusive. Ces accès restent soumis aux contrôles applicatifs JWT/session/rôles.
- `filfinance-auth`: API Node.js/TypeScript Better Auth + PostgreSQL, cible pour les sessions, profils, roles et preferences.
- Backend Rust: ingestion, traduction, TTS, SQLite pipeline et origin HLS pour clips audio.
- Collector navigateur: container Playwright/Chromium toujours demarre mais idle, actif seulement quand le backend active le fallback FinancialJuice.
- Une éventuelle évolution du live vers des services CDN supplémentaires doit être qualifiée séparément ; elle n’est pas déduite du build des frontends.

## Lancement Docker unifie

Le mode Docker cible partage les memes services applicatifs en local et en production;
la composition deployee ajoute le gateway d'origine:

- `filfinance-service`: backend Rust, API, WebSockets, ingestion, traduction, TTS/HLS.
- `filfinance-auth`: API auth Better Auth connectee au PostgreSQL local de la stack.
- `filfinance-postgres`: PostgreSQL prive avec schemas `auth` et `app`.
- `filfinance-gateway`: entree HTTP deployee, avec `/api/auth/*` et `/api/me` vers Better Auth, puis le reste vers Rust, WebSocket compris.
- `financialjuice-browser-collector`: fallback navigateur Playwright, demarre avec la stack mais actif seulement quand le backend desactive la source `financial_juice`.
- `filfinance-client`: service Docker local construit depuis `apps/pwa`.

Local complet backend + collector + frontend:

```bash
cp .env.local.example .env.local
# Completer .env.local avant le premier demarrage.
docker compose --env-file .env.local -f compose.yaml -f compose.local.yaml up --build
```

URLs locales par defaut:

- PWA Expo Web: `http://localhost:8052`
- landing Astro en développement: `http://localhost:4321`
- backend: `http://localhost:8088`
- auth: `http://localhost:3000/health`
- healthcheck: `http://localhost:8088/health`

Production backend + collector, le frontend restant heberge hors serveur Docker:

```bash
cp .env.server.example .env.server
# Completer .env.server avant le premier demarrage.
docker compose --env-file .env.server -f compose.yaml -f compose.prod.yaml up -d --build
```

Le Dockerfile backend est autonome: il embarque maintenant la base runtime OPUS-MT/Piper dans un stage Docker interne. Il n'y a plus d'image `filfinance-service-runtime-base:bookworm` a construire manuellement.

## Prérequis et contrôles locaux

Les packages imposent Node 24.13.1 et npm >=11.8.0 <12 ; la préparation
a utilisé Node 24.18.0. Rust/Cargo avec prise en charge de l’édition 2024
est requis pour le backend. Docker avec Compose v2 sert à la stack ;
PostgreSQL est requis pour Better Auth. Les modèles OPUS-MT/Piper et
le navigateur du collector ajoutent des besoins propres à leur composant,
sans autorisation implicite de collecte ni de redistribution.

```bash
npm --prefix filfinance-auth ci
npm --prefix filfinance-auth test
npm --prefix filfinance-auth run check
cargo test --locked --manifest-path filfinance-service/Cargo.toml auth::
cargo test --locked --manifest-path filfinance-service/Cargo.toml logs::
cargo test --locked --manifest-path filfinance-service/Cargo.toml features::audio::hls::access
```

Les tests ciblés utilisent des valeurs inventées. Le smoke Better Auth complet
demande une instance PostgreSQL jetable et une capture SMTP locale ; consulter
[le guide Auth](filfinance-auth/README.md) avant exécution. Les suites locales
ne qualifient pas un service déployé ou le futur export public.

## Commandes frontend racine

Installer les dépendances de chaque produit de façon indépendante:

```bash
npm --prefix apps/landing ci
npm --prefix apps/pwa ci
```

Puis utiliser les scripts racine:

```bash
npm run dev:landing
npm run build:landing
npm run preview:landing
npm run check:landing
npm run lighthouse:landing
npm run dev:pwa
npm run build:pwa
npm run check:pwa
```

Les deux packages conservent leur propre `package.json`, leur lockfile, leurs dépendances et leur sortie de build.

La landing présente les quatre services en clair par défaut. Le bouton de
thème mémorise le choix dans ce navigateur, indépendamment de la PWA. Les
aperçus audio, actualités et calendrier restent illustratifs. La
[recette locale d’EPIC-038](./apps/landing/design-qa.md) conserve les captures
et les résultats des audits clair/sombre ; aucun déploiement n'est attesté.

## Workflow GitLab

Le depot cible est un monorepo GitLab. La production publie les artefacts Pages
Cloudflare; DEV publie les artefacts Bunny Storage :

| Environnement | Landing | PWA |
| --- | --- | --- |
| PROD (`main`) | `filfinance-prod` → `https://filfinance.fr` | `filfinance-app-prod` → `https://app.filfinance.fr` |
| DEV (`dev`) | Bunny Storage → `https://dev.filfinance.fr` | Bunny Storage → `https://app.dev.filfinance.fr` |

La configuration DEV vise le pilote Bunny : DNS délégué, Pull Zones, WebSocket
Rust/Redis et smokes dédiés. Son état réel doit être vérifié séparément. `FILFINANCE_DEV_PWA_URL=https://app.dev.filfinance.fr`; la
production Cloudflare reste inchangée. Voir `docs/BUNNY-DEV-RUNBOOK.md`.

Documentation:

- [Architecture landing / PWA](./docs/architecture.md)
- [Pages légales : manifeste Fil Finance](./docs/guides/legal-landing-app/MANIFESTE.md)
- [Pages légales : configuration et recette locale](./docs/guides/legal-landing-app/RECETTE-FILFINANCE.md)
- [Roadmap](./docs/ROADMAP.md)
- [EPIC-038 - Refonte visuelle de la landing et thèmes clair/sombre](./docs/epics/EPIC-038-refonte-visuelle-landing-themes-clair-sombre.md)
- [Landing : palettes, textes et maquettes de référence](./docs/design/landing/README.md)
- [EPIC-039 - Refonte PWA avec gluestack v5, UniWind et Tailwind CSS 4](./docs/epics/EPIC-039-refonte-pwa-gluestack.md)
- [PWA : palettes, typographie, composants et maquettes de référence](./docs/design/pwa/README.md)
- [EPIC-021 - Audio live evenementiel par clips HLS progressifs](./docs/epics/EPIC-021-audio-live-evenementiel-clips-hls-progressifs.md)
- [EPIC-028 - Migration production main vers filfinance.fr](./docs/epics/EPIC-028-migration-production-filfinance-fr.md)
- [EPIC-029 - Environnement dev landing et PWA, historique Pages](./docs/epics/EPIC-029-environnement-dev-pages-branch-dev.md)
- [EPIC-020 - GitLab, Cloudflare Pages et serveur personnel](./docs/epics/EPIC-020-gitlab-pipeline-cloudflare.md)
- [Runbook production OVH](./docs/OVH-PRODUCTION-RUNBOOK.md)
- [Runbook GitLab](./docs/GITLAB-RUNBOOK.md)
- [Fichiers d'environnement](./docs/ENVIRONMENT-FILES.md)
- [Migration Better Auth + PostgreSQL](./docs/MIGRATION-BETTER-AUTH-POSTGRES.md)
- [NEWS-111 - Deploiement serveur perso Traefik](./docs/tickets/NEWS-111-deploiement-home-srv-traefik.md)


## Recette, données et arrêt

Les exemples `.env.local.example` et `.env.server.example` documentent les clés,
mais exigent des credentials privés neufs. Ne pas copier une configuration de
production dans une recette. Garder PostgreSQL, SQLite, les profils navigateur,
les journaux et sorties audio dans un stockage privé de développement ; une
installation publique doit utiliser des données inventées et des modèles aux
droits établis. Ne pas lancer une collecte externe sans accès autorisé.

Le mode production avec authentification Rust désactivée est refusé ; un mode
de démo local ne valide pas cette frontière en production. La compilation ne
prouve ni le parcours navigateur, ni le service Better Auth déployé, ni la validité
des droits des données ingérées. Aucun déploiement Bunny/Cloudflare n’est requis
pour les tests de préparation locale.

Arrêter Compose avec :

```bash
docker compose --env-file .env.local -f compose.yaml -f compose.local.yaml down
```

Les bases et profils persistent ; ne pas ajouter `-v`
sans décision explicite sur les données. Arrêter les serveurs frontend avec Ctrl+C.
Les tests d’accès et le parcours depuis un export filtré restent nécessaires
avant G01 ; les résultats historiques dans les runbooks ne qualifient pas un
nouveau SHA. Aucune licence globale nouvelle n’est choisie par cette préparation.


Le répertoire NEWS_LOG_DIR du lecteur historique doit être contrôlé par
l'opérateur (racine et parents). Les fichiers journaliers symlinks/FIFO sont
refusés ; la date reste limitée à J/J−1. Le lecteur calendar/RLOGGER est distinct
et conserve ses contrôles propres. L'overlay production impose environnement
production et authentification Rust active, même si le fichier .env était mal
renseigné. Cela ne remplace ni les valeurs JWKS/issuer/audience ni la revue des
routes et secrets de déploiement.

Voir [sécurité de publication et vérification du credential HLS historique](docs/SECURITY-PUBLICATION.md) avant tout export ou intervention sur un service déployé.
