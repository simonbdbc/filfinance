# Fil Finance

`Fil Finance` regroupe la landing publique Astro, la PWA Expo Web, le backend Rust et la documentation produit/technique.

## Structure

- `apps/landing`: site public Astro statique et SEO, avec un unique script local pour le choix clair/sombre.
- `apps/pwa`: application Expo/React Native Web `filfinance`, déplacée sans changement fonctionnel.
- `filfinance-service`: backend Rust, ingestion FinancialJuice, pipeline news/audio, traduction, TTS, API et WebSockets actuels.
- `financialjuice-browser-collector`: collecteur navigateur Playwright separe pour le MVP FinancialJuice quand le site exige un vrai navigateur.
- `docs`: contrats, schémas et notices retenus pour la présentation. Les tickets, archives et procédures de production restent privés.

## Statut et provenance

Le développement reste sur [GitLab](https://gitlab.com/simonbdbc/filfinance),
actuellement privé. Le dépôt [GitHub public](https://github.com/simonbdbc/filfinance)
reçoit un export filtré avec un historique public séparé.
Les sources de calendrier/audio et les modèles ont leurs propres droits : les
accès techniques ne donnent pas une autorisation de redistribution.

```bash
git clone https://github.com/simonbdbc/filfinance.git
cd filfinance
```

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
recette locale de la landing conserve les captures
et les résultats des audits clair/sombre ; aucun déploiement n'est attesté.

## Recette, données et arrêt

Le modèle `.env.local.example` documente les clés,
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
et conserve ses contrôles propres. 
Voir [sécurité de publication et vérification du credential HLS historique](docs/SECURITY-PUBLICATION.md) avant tout export ou intervention sur un service déployé.

## Synchronisation du dépôt de présentation

La version publique est disponible sur https://github.com/simonbdbc/filfinance. Son historique est séparé de l’historique privé GitLab. Le job de publication exporte les fichiers autorisés depuis la branche dédiée `publication/github-main`, contrôle le contenu puis ajoute un commit à la suite du dernier commit public. Les anciennes branches, étiquettes, données privées et artefacts internes ne sont pas transférés. Les propositions de modification sont revues dans GitLab avant publication. Cette synchronisation des sources ne déploie pas l’application.

## Stack locale et périmètre public

Le dépôt conserve les sources applicatives et leurs tests, les Dockerfiles et compositions locales. Les scripts de déploiement, sauvegarde, migration serveur, vérification live et recettes de production sont exclus. Aucun outil de publication Cloudflare/Bunny/OVH n’est fourni.

```bash
cp .env.local.example .env.local
# Renseigner uniquement des valeurs de recette privées et des données fictives.
docker compose --env-file .env.local -f compose.yaml -f compose.local.yaml up --build
```

Les collecteurs ne doivent contacter une source externe qu’avec un accès autorisé. Les builds/tests locaux ne prouvent ni production ni redistribution des données/modèles.

[Périmètre de publication et exclusions de production](PUBLICATION.md).
