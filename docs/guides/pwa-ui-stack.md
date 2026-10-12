# PWA — gluestack v5, UniWind et Tailwind CSS 4

Décision EPIC-039, mise en œuvre locale les 16–17 septembre 2026. Ce guide décrit
le code actuel ; les bilans historiques NativeWind restent des archives.

## Installation et compilation

Dans `apps/pwa/package.json` et le lockfile : `@gluestack-ui/core` **5.0.15**,
`@gluestack-ui/utils` **5.0.6**, `uniwind` **1.3.0**, `tailwindcss` **4.1.18**,
`react-native-svg` **15.15.3**, `@expo/vector-icons` **15.1.1**.

Metro utilise `withUniwindConfig` avec `global.css`, et génère
`uniwind-types.d.ts`. Tailwind 4 est configuré en CSS, sans fichier de configuration
Tailwind 3 ni pipeline PostCSS supplémentaire. Babel conserve Expo et la résolution
des alias. Le contrôle légal `prebuild:web` reste obligatoire.

```sh
npm ci --prefix apps/pwa
npm run check:pwa
# Définir les variables publiques adaptées à l’environnement avant cet export.
npm run build:pwa
```

Le validateur `qa:ui-stack`, inclus dans `check:pwa`, bloque la réintroduction de
NativeWind, css-interop, des anciens presets/directives et palettes supprimées.
Il inspecte les sources, le lockfile et l’arbre effectivement installé avec `npm ls`.
Les contrôles métier auth, historique, permissions, HLS, polling et légal restent
actifs. L’environnement Node de référence est déclaré dans `package.json`.

Références officielles : [gluestack v5](https://gluestack.io/ui/docs/guides/more/upgrade-to-v5),
[installation UniWind](https://docs.uniwind.dev/quickstart),
[migration depuis NativeWind](https://docs.uniwind.dev/migration-from-nativewind).

## Tokens et thème

`global.css` est l’unique source des valeurs : `@variant light`, `@variant dark`,
variables `--ff-*`, polices système et rayons. `shared/theme/tokens.ts` associe des
noms sémantiques aux variables via `useCSSVariable` ; il ne recopie aucune palette.
Les propriétés React Native qui ne passent pas par `className` utilisent ces tokens.
`Text` résout les classes via `useResolveClassNames` : valeurs par défaut, classes
UniWind puis styles explicites. Le helper historique de détection des couleurs est
supprimé ; les tailles inférieures à 14 px sont bornées.
Sur Web, les polices sont Georgia / Arial, avec les replis de la charte ; les logs
utilisent monospace. Les petits textes restent au moins à 14 px.

L’état canonique est celui d’UniWind. `ThemeProvider` lit `filfinance.theme` avant le
montage Web, ignore toute valeur autre que `light`/`dark` et choisit le clair par
défaut. Aucun thème système automatique. L’échec de stockage est intercepté ; le
choix pendant la visite reste actif. Le profil distant ne lit ni ne modifie cet état.
L’ancien champ API reste compatible côté serveur, sans nouvelle écriture côté PWA.
La préférence de la landing reste distincte.

## Composants et organisation

- `shared/ui/primitives` isole les créateurs gluestack : bouton, champ, sélecteur,
  pressable, switch et dialogue. Les primitives natives de texte et surface sont
  habillées par UniWind et les tokens communs.
- `shared/ui` fournit `Button`, `TextInput`, `Select`, `Switch`, `Text`, `Surface`,
  `SectionHeader`, `PageHeader`, `DataRow`, `MasterDetail`, `DialogPanel`,
  `StatusBadge`, `PreferenceRow`, `AsyncState` et les compositions `Workspace`.
  Ces composants reçoivent des données et callbacks, sans charger les API.
- Les fonctionnalités gardent hooks, services, validations, permissions et actions.
  Les routes Expo Router restent des points d’entrée fins.

Les anciennes façades `AdminDashboard`, `Card`, `PageHeaderCard`, `ScreenShell`,
`WebSelect` et les palettes concurrentes sont retirées, sans adaptateur de transition.
Les SVG d’actions remplacés par Feather sont supprimés ; l’asset de marque et le
favicon existants sont conservés.

## Navigation, audio et accessibilité

Sidebar à partir de 1024 px ; navigation inférieure et menu en dessous. Les droits
modérateur/admin existants déterminent les accès. Détail à côté de la liste à
partir de 1280 px ; dialogue gluestack en dessous, avec focus initial, confinement,
Échap et restitution du focus. Les tableaux deviennent des lignes libellées sur
mobile ; seuls les tableaux larges peuvent défiler horizontalement.

Le lecteur est monté une seule fois dans `AppLayout`. Le runtime existant reste
au-dessus des routes connectées. Aucun moteur, abonnement, autoplay ou volume
supplémentaire. Pas de lecteur dans l’authentification. Le résumé conserve ses
états préparation, lecture, arrêt et indisponibilité.

Les contrôles ont une cible minimale de 44 px, les icônes décoratives sont exclues
du nom accessible, les filtres annoncent leur état pressé, les liens la page active.
Un statut comporte un libellé ; les sources désactivées sont neutres.

## Recette locale reproductible

Les scripts `apps/pwa/scripts/qa/` interceptent les API et WebSocket avec des données
explicitement fictives. Ils bloquent les requêtes distantes. Aucun compte de
production ni secret n’est requis. Les captures ne sont pas des maquettes et ne
prouvent pas un déploiement. Voir le [dossier de recette](../design/pwa/recette/2026-09-16/README.md).

Préparer un export exclusivement local depuis la racine :

```sh
EXPO_NO_DOTENV=1 \
EXPO_PUBLIC_LANDING_ORIGIN=http://127.0.0.1:4321 \
EXPO_PUBLIC_API_URL=http://127.0.0.1:9999 \
EXPO_PUBLIC_BETTER_AUTH_URL=http://127.0.0.1:9999 \
EXPO_PUBLIC_FILFINANCE_WS_URL=ws://127.0.0.1:9999/ws/news \
EXPO_PUBLIC_FILFINANCE_PWA_URL=http://127.0.0.1:8052 \
npm run build:pwa
node apps/pwa/scripts/qa/serve.cjs
```

Dans un second terminal, après installation des dépendances de la landing
(Puppeteer et axe-core déjà utilisés par ses audits) :

```sh
node apps/pwa/scripts/qa/browser.cjs
node apps/pwa/scripts/qa/extended.cjs
mkdir -p /tmp/ff-hls
ffmpeg -hide_banner -loglevel error -y -f lavfi -i anullsrc=r=44100:cl=mono \
  -t 120 -c:a aac -b:a 24k -hls_time 4 -hls_list_size 0 /tmp/ff-hls/playlist.m3u8
node apps/pwa/scripts/qa/interactions.cjs
```

`CHROME_PATH` permet de choisir l’exécutable Chrome. Le test audio décode réellement
le média HLS silencieux local et mesure les éléments audio, abonnements, appels
play/pause et progression du temps. Il ne prouve pas la disponibilité des services
ou des flux de production. Les assertions des lecteurs de logs exécutent leur
fenêtre de lecture réelle et les correctifs d’en-têtes préexistants.
