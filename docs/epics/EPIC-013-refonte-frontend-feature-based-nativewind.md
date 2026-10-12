# EPIC-013 - Refonte frontend feature-based et NativeWind v4

## Objectif

Refactoriser `filfinance` pour sortir d'un `App.tsx` monolithique et converger vers une architecture frontend maintenable:

- decoupage `feature-based`
- composants UI factorises
- services et hooks separes de la presentation
- theming et styles via `NativeWind v4`

## Valeur livree

Le client Expo devient evolutif. Les prochains chantiers, en particulier l'authentification, les ecrans additionnels, les preferences utilisateur et les variantes mobile/web, pourront etre ajoutes sans continuer a grossir un seul fichier central difficile a relire et a tester.

## Statut

Fait le 1 mai 2026.

Extension fonctionnelle livree et validee le 3 mai 2026: `NEWS-114` ajoute un side menu et des pages role-based sans remettre en cause la cloture du refactor initial.

Extension theme livree et validee le 3 mai 2026: `NEWS-115` a `NEWS-119` cadrent l'audit palette, les tokens clair/sombre, le switch de theme en bas de sidebar, la QA contraste/responsive et la synchronisation de preference profil. `NEWS-116` est livre: les tokens et la migration des primitives sont en place. `NEWS-117` est livre: le switch clair/sombre est disponible en bas de sidebar avec persistance locale. `NEWS-118` a valide les gates techniques et contrastes; la validation responsive est faite en live par l'utilisateur. `NEWS-119` synchronise maintenant `preferences.theme` via `GET/PATCH /api/me`.

Livraison:

- `App.tsx` est devenu un bootstrap fin vers `src/app/FilFinanceApp`
- creation de la structure `src/app`, `src/features` et `src/shared`
- extraction des contrats frontend HTTP/WebSocket dans `src/shared/api`
- extraction des helpers d'URL, d'historique, de formatage, de statut et d'audio web
- encapsulation du `select` HTML dans une primitive `src/shared/ui/WebSelect`
- extraction de premiers blocs feature-based: audio, live-feed et system errors
- extraction du chargement d'historique journalier dans `src/features/history`: service HTTP pagine et hook d'etat dedie
- extraction du rendu des historiques feed et `test_run` dans `src/features/history`
- extraction du rendu de declenchement et de trace des `test-runs` dans `src/features/test-runs`
- extraction du bloc settings/connexion backend et reglages audio dans `src/features/settings`
- extraction du hook WebSocket `/ws/test-runs` dans `src/features/test-runs/useTestRunTraceSocket`
- extraction du hook WebSocket texte `/ws/news` dans `src/features/live-feed/useNewsSocket`
- extraction des helpers purs de file audio, admission FIFO, deduplication et preparation des replays dans `src/features/audio/audioQueue`
- extraction du canal audio binaire, du player `Web Audio` et du WebSocket `/ws/audio` dans `src/features/audio/useAudioPlayback`
- extraction du player `Web Audio` et `AudioWorklet` dans `src/features/audio/useAudioPlayer`
- extraction du cycle de vie WebSocket `/ws/audio` dans `src/features/audio/useAudioSocket`
- installation et branchement `NativeWind v4`: `metro.config.js`, `babel.config.js`, `tailwind.config.js`, `global.css`, types NativeWind
- premiere migration NativeWind sur l'enveloppe et le header de `src/app/FilFinanceApp.tsx`
- migration NativeWind de `src/features/system/ErrorSummaryCard.tsx` et `src/features/live-feed/LiveStatsRow.tsx`
- migration NativeWind de `src/features/live-feed/CurrentNewsCard.tsx`
- migration NativeWind de `src/features/settings/ConnectionSettingsCard.tsx` et `src/features/audio/VoiceSelector.tsx`
- migration NativeWind des cartes historiques et test-run: `DailyFeedHistoryCard`, `DailyTestRunHistoryCard`, `TestRunControlCard`, `TestRunTraceCard`
- suppression du `StyleSheet` partage `src/shared/theme/styles.ts`, qui n'est plus utilise par l'ecran principal
- factorisation des classes NativeWind repetees dans `src/shared/ui`: `Card`, `CardHeader`, `CardListItem`, `ActionButton`, `Badge`, `StatTile`, `Typography` et helper `cx`
- debug runtime `NEWS-082` du debordement horizontal des cards: cause racine identifiee dans le shell React Native Web et correction validee par mesures `scrollWidth == clientWidth` sur `320`, `375`, `414`, `768`, `1024`
- navigation applicative `NEWS-114`: side menu mobile-app, page profil, page tests `moderator/admin`, page admin `admin`
- theme clair/sombre `NEWS-115` a `NEWS-119`: tokens centralises, palette sombre type Codex, switch en bas de sidebar, persistance locale et synchronisation profil
- cloture `NEWS-066`: documentation client a jour, audit des chemins apres renommage `filfinance-client`, typecheck et export web smoke valides

Suites post-EPIC possibles:

- decoupage optionnel des handlers metier audio si le protocole `/ws/audio` evolue encore
- enrichissement futur de la page admin si une gestion utilisateurs complete devient necessaire

## Audit du code actuel au 1 mai 2026

Constats principaux:

- le frontend tenait quasi entierement dans [apps/pwa/App.tsx](../../apps/pwa/App.tsx), qui faisait `2716` lignes avant la premiere tranche de refactor
- un repertoire `src/` existe maintenant avec une premiere separation `app / features / shared`
- le nouvel ecran principal `src/app/FilFinanceApp.tsx` est descendu a environ `386` lignes et garde surtout:
  - l'orchestration des hooks feature
  - l'etat de vue restant
  - le rendu
- le selecteur de voix web est maintenant encapsule dans une primitive UI dediee
- le chargement HTTP de l'historique journalier est maintenant sorti dans `src/features/history`
- les cartes d'historique journalier feed et `test_run` sont maintenant sorties dans `src/features/history`
- les cartes de declenchement et de trace des `test-runs` sont maintenant sorties dans `src/features/test-runs`
- la carte settings/connexion backend et reglages audio est maintenant sortie dans `src/features/settings`
- la connexion WebSocket de trace `/ws/test-runs` est maintenant portee par un hook feature dedie
- la connexion WebSocket texte `/ws/news` est maintenant portee par un hook feature dedie
- la politique de file audio FIFO, de deduplication et de replay est maintenant factorisee dans `src/features/audio/audioQueue`
- le replay audio manuel/live et la coordination audio sont maintenant portes par `src/features/audio/useAudioPlayback`
- le cycle de vie de connexion/reconnexion `/ws/audio` est maintenant isole dans `src/features/audio/useAudioSocket`
- le player `Web Audio` / `AudioWorklet` est maintenant isole dans `src/features/audio/useAudioPlayer`
- `NativeWind v4` est branche dans Metro/Babel/Tailwind et valide par un export web smoke
- la stack styling de l'ecran principal utilise maintenant `className`: header racine, erreurs, stats live, news courante, settings, historique et test-runs ne dependent plus d'un `StyleSheet` partage
- les motifs UI repetes sont portes par `src/shared/ui`, ce qui reduit la duplication de classes dans les features
- les invariants audio FIFO issus des reviews sont conserves apres refactor: admission live avant file, file binaire triee par timestamp, replays reconstruits depuis les items metier avant conversion en IDs

Risques residuels acceptes:

- `useAudioPlayback` reste responsable de la coordination du protocole audio; il faudra le rediscuter si `/ws/audio` gagne de nouveaux messages ou modes
- les primitives UI couvrent l'ecran principal, mais il faudra les durcir si de nouveaux ecrans exigent plus de variantes
- aucune librairie de tests frontend n'a ete introduite dans cet EPIC; la gate actuelle reste typecheck, export web et smoke navigateur documente

## Cible d'architecture

Le but n'est pas de changer le produit, mais de changer la structure.

Architecture cible recommandee:

- `src/app`
  - bootstrap, providers, configuration globale
- `src/features/live-feed`
  - flux live, dernier item, file, cartes associees
- `src/features/history`
  - historique feed, historique `test_run`, replay manuel
- `src/features/audio`
  - selecteur de voix, etat audio, politique de file/replay, player, hooks `Web Audio`
- `src/features/test-runs`
  - declenchement test E2E, trace, etat du run
- `src/features/settings`
  - URL backend, options locales, futurs reglages utilisateur
- `src/shared/ui`
  - primitives `Card`, `CardHeader`, `CardListItem`, `ActionButton`, `Badge`, `StatTile`, `Typography`, `Select`
- `src/shared/lib`
  - dates, tri, dedup, formatters, utilitaires purs
- `src/shared/api`
  - client HTTP, clients WebSocket, contrats frontend derives
- `tailwind.config.js` et `src/shared/ui`
  - tokens, variants et primitives `NativeWind v4`

## Choix de scope

Dans cet EPIC:

- oui au refactor structurel
- oui a `NativeWind v4`
- oui a la factorisation des composants et hooks
- oui a une couche `services` pour HTTP/WebSocket/audio
- non a une refonte fonctionnelle du produit
- non a une redefinition du protocole backend
- non a l'ajout d'auth dans ce meme lot, sauf micro-preparatifs techniques evidents

## Definition of Done

- `App.tsx` devient un bootstrap fin, pas un ecran monolithe
- le client est range sous `src/` avec une separation `app / features / shared`
- les blocs UI principaux sont extraits en composants reutilisables
- la logique metier frontend est deplacee dans des hooks et services nommes
- `NativeWind v4` est branche et documente
- les classes NativeWind remplacent le `StyleSheet` ad hoc de l'ecran principal
- les regressions live/history/audio/test-run sont revalidees
- la doc client decrit la nouvelle architecture
- `NEWS-066` documente la gate de cloture et les limites post-EPIC

## Tickets lies

- `NEWS-062` Cadrage architecture frontend cible et conventions de dossiers
- `NEWS-063` Installation `NativeWind v4`, tokens et primitives UI
- `NEWS-064` Extraction des services transverses HTTP, WebSocket et audio
- `NEWS-065` Decoupage feature-based des blocs live, historique, audio et test-run
- [`NEWS-066`](../tickets/NEWS-066-stabilisation-non-regression-client.md) Stabilisation, tests de non-regression et documentation du client
- `NEWS-082` Debug overflow horizontal des cards NativeWind
- [`NEWS-114`](../tickets/NEWS-114-side-menu-pages-role-based.md) Side menu et pages role-based profil/tests/admin
- [`NEWS-115`](../tickets/NEWS-115-audit-palette-theme-client.md) Audit palette client et cadrage theming clair/sombre
- [`NEWS-116`](../tickets/NEWS-116-tokens-theme-clair-sombre-nativewind.md) Tokens theme clair/sombre NativeWind
- [`NEWS-117`](../tickets/NEWS-117-switch-theme-sidebar.md) Switch theme clair/sombre dans la sidebar
- [`NEWS-118`](../tickets/NEWS-118-theme-dark-qa-contraste.md) QA theme sombre, contrastes et non-regression responsive
- [`NEWS-119`](../tickets/NEWS-119-sync-theme-user-profile.md) Synchronisation theme avec user_profiles

## Notes de mise en oeuvre

- il n'est pas obligatoire d'introduire `expo-router` dans cet EPIC
- le refactor doit rester incremental pour ne pas casser le flux audio
- les composants web-specifiques comme le `select` HTML doivent etre encapsules dans une primitive UI dediee plutot que rester inline dans l'ecran racine
- le lot `NEWS-114` doit reutiliser l'etat de profil charge par `EPIC-014` pour masquer et bloquer les pages non autorisees cote client
- le backend reste la source de verite des droits: le filtrage frontend ameliore l'UX mais ne remplace pas les controles HTTP/WebSocket
- le lot theme doit d'abord migrer les primitives `shared/ui`; les composants feature ne doivent pas chacun reinventer leur propre mapping clair/sombre
- le switch theme doit rester en bas de la sidebar et ne doit pas fermer le menu quand l'utilisateur bascule clair/sombre
