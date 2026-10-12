# NEWS-082 - Debug overflow horizontal des cards NativeWind

## Statut

Fait le 30 avril 2026.

## Objectif

Identifier et corriger la cause reelle du debordement horizontal des cards du client Expo web apres la migration `NativeWind v4`.

Le symptome observe cote utilisateur est que toutes les cards depassent de l'ecran. Le correctif precedent a ajoute des contraintes de largeur (`w-full`, `max-w-full`, `min-w-0`, `box-border`, `break-words`) mais le probleme persiste, donc le ticket doit partir d'une mesure navigateur et non d'une hypothese de classes.

## Scope

- auditer le layout web de `filfinance`
- reproduire le debordement sur viewport mobile et desktop
- identifier l'element exact qui force `scrollWidth > clientWidth`
- corriger la cause racine dans le shell d'ecran ou dans les primitives UI
- verifier que toutes les cards restent dans le viewport sans scroll horizontal

Fichiers candidats:

- `filfinance-client/src/app/FilFinanceApp.tsx`
- `filfinance-client/src/shared/ui/Card.tsx`
- `filfinance-client/src/shared/ui/Button.tsx`
- `filfinance-client/src/shared/ui/Badge.tsx`
- `filfinance-client/src/shared/ui/Typography.tsx`
- `filfinance-client/src/features/settings/ConnectionSettingsCard.tsx`
- `filfinance-client/src/features/live-feed/LiveStatsRow.tsx`
- `filfinance-client/src/features/live-feed/CurrentNewsCard.tsx`
- `filfinance-client/src/features/history/DailyFeedHistoryCard.tsx`
- `filfinance-client/src/features/history/DailyTestRunHistoryCard.tsx`
- `filfinance-client/src/features/test-runs/TestRunControlCard.tsx`
- `filfinance-client/src/features/test-runs/TestRunTraceCard.tsx`

## Hors Scope

- changer le design global ou la charte visuelle
- modifier la logique audio, WebSocket, historique ou test-run
- masquer des champs metier pour reduire artificiellement la largeur
- considerer `tsc` ou `expo export` comme preuve suffisante de correction visuelle

## Audit initial

Constats dans le code au 30 avril 2026:

- `FilFinanceApp.tsx` utilise `ScrollView` avec un wrapper interne `View` en `w-full max-w-full p-5`; sur React Native Web, un `ScrollView` peut exposer une largeur de contenu differente du viewport si son content container ou ses enfants ne sont pas bornes par des styles natifs explicites.
- `Card.tsx` repose sur des classes `box-border`, `w-full`, `max-w-full`, `min-w-0`, `overflow-hidden`; certaines classes CSS-like peuvent etre ignorees ou ne pas avoir l'effet attendu sur des composants React Native selon le mapping NativeWind.
- `Typography.tsx` et `Button.tsx` utilisent `break-words`; cette classe est CSS web et doit etre verifiee dans le rendu React Native Web. Si elle n'est pas appliquee, les URLs, timestamps et messages longs peuvent imposer une largeur superieure au viewport.
- les headers ont ete transformes en colonne dans `CardHeader`, mais plusieurs zones gardent des `flex-row flex-wrap`: `LiveStatsRow`, actions de `CurrentNewsCard`, lignes de settings et ligne interne de `TestRunTraceCard`.
- les contenus a risque sont les URLs WebSocket/audio, timestamps ISO, `source_title`, `translated_title`, `test_run_id`, `news_level` et messages d'erreur.

Hypotheses a verifier:

1. le debordement vient du shell `ScrollView`/wrapper, pas de chaque card individuellement
2. `w-full` combine au padding du wrapper produit une largeur effective trop grande sur web
3. les classes `box-border` ou `break-words` ne sont pas appliquees par NativeWind sur React Native Web
4. un texte non cassable dans `Text`, `TextInput`, `Badge` ou `ActionButton` force la largeur de son parent

## Reproduction runtime

Harness utilise:

- export web Expo servi en local
- page de test locale avec iframes de largeur `320`, `375`, `414`, `768`, `1024`
- mesure dans chaque iframe de `clientWidth`, `scrollWidth`, `bodyScrollWidth`, `innerWidth`
- listing des premiers elements dont `getBoundingClientRect().right > clientWidth + 1`

Mesure avant correction:

| Viewport | `clientWidth` | `scrollWidth` | Resultat |
| --- | ---: | ---: | --- |
| `320` | `320` | `1312` | overflow |
| `375` | `375` | `1312` | overflow |
| `414` | `414` | `1312` | overflow |
| `768` | `768` | `1312` | overflow |
| `1024` | `1024` | `1312` | overflow |

Elements debordants observes:

- shell React Native Web racine sous `body`
- conteneur `ScrollView`
- wrapper interne de page
- premiers textes du header, qui heritaient de la largeur racine trop grande

Cause confirmee:

- le shell racine suivait une largeur de viewport top-level autour de `1312px`
- `w-full` propageait cette largeur aux wrappers internes
- les cards n'etaient pas la cause primaire; elles heritaient d'un parent deja trop large

## Correction initiale

Correction appliquee dans `filfinance-client/src/app/FilFinanceApp.tsx`:

- `SafeAreaView` borne en `w-screen max-w-[100vw] overflow-hidden`
- `ScrollView` borne en `box-border w-screen max-w-[100vw] overflow-hidden`
- wrapper de contenu borne en `box-border w-screen max-w-[100vw]`

La logique audio, WebSocket, historique et test-run n'a pas ete modifiee.

Cette correction a ensuite ete remplacee par le shell `ScreenShell` decrit dans la section de regression, car `overflow-hidden` sur `ScrollView` bloquait le scroll vertical.

Mesure apres correction:

| Viewport | `clientWidth` | `scrollWidth` | Resultat |
| --- | ---: | ---: | --- |
| `320` | `320` | `320` | OK |
| `375` | `375` | `375` | OK |
| `414` | `414` | `414` | OK |
| `768` | `768` | `768` | OK |
| `1024` | `1024` | `1024` | OK |

Aucun element debordant n'a ete remonte par le harness apres correction.

## Regression scroll vertical du 1 mai 2026

Symptome observe:

- la home ne pouvait plus scroller verticalement apres l'ajout de l'auth et du shell borne

Cause:

- le correctif initial appliquait `overflow-hidden` directement au `ScrollView`
- ce verrou empechait le scroll vertical sur React Native Web alors que l'intention etait seulement de bloquer l'overflow horizontal

Correction complementaire:

- creation de `src/shared/ui/ScreenShell.tsx`
- reutilisation de ce shell par `src/app/FilFinanceApp.tsx` et `src/features/auth/AuthGate.tsx`
- suppression de `overflow-hidden` sur le `ScrollView`
- borne horizontale explicite en web via `overflowX: "hidden"`
- conservation du scroll vertical gere par `ScrollView`

Validations executees:

- `./node_modules/.bin/tsc --noEmit --pretty false`: OK
- `npm run typecheck -- --pretty false` via runtime Node recent: OK
- export web Expo vers `filfinance-client/dist-web-smoke`: OK

## Approche Technique

1. Lancer l'export ou le dev server web du client.
2. Ouvrir le client dans un navigateur avec viewports `320x800`, `375x812`, `414x896`, `768x1024` et desktop.
3. Mesurer dans le navigateur:
   - `document.documentElement.clientWidth`
   - `document.documentElement.scrollWidth`
   - `document.body.clientWidth`
   - `document.body.scrollWidth`
4. Identifier le premier element debordant avec un script de debug qui liste les nodes dont `getBoundingClientRect().right > window.innerWidth`.
5. Corriger la cause racine avec des styles React Native explicites si les classes NativeWind ne suffisent pas:
   - preferer `style`/`contentContainerStyle` pour les garanties de layout du shell
   - utiliser `alignSelf: "stretch"`, `width: "100%"`, `maxWidth: "100%"`, `minWidth: 0`
   - eviter `w-full` sur un enfant avec padding si le parent n'est pas explicitement borne
   - remplacer les textes/metadata longs par des blocs cassables ou truncables selon le cas
6. Conserver les primitives UI, mais les rendre mesurables et robustes sur React Native Web.

## Criteres d'acceptation

- aucun scroll horizontal visible sur les viewports `320`, `375`, `414`, `768` et desktop
- pour chaque viewport teste: `document.documentElement.scrollWidth <= document.documentElement.clientWidth`
- les cards ne depassent pas du fond principal et restent visuellement alignees avec le padding de page
- les boutons des headers ne forcent pas la largeur de la card
- les URLs, timestamps, titres longs et messages d'erreur ne forcent pas la largeur du viewport
- aucune regression fonctionnelle sur les actions:
  - reconnecter WebSocket
  - stop audio
  - vider la file
  - rejouer historique feed
  - rejouer historique `test_run`
  - declencher test E2E

## Tests attendus

- `./node_modules/.bin/tsc --noEmit --pretty false`
- export web Expo
- smoke navigateur manuel ou automatise sur les viewports `320`, `375`, `414`, `768`, desktop
- capture ou log de mesure avant/apres:
  - `clientWidth`
  - `scrollWidth`
  - selecteur ou chemin DOM du ou des elements debordants

## Tests executes

- `./node_modules/.bin/tsc --noEmit --pretty false`: OK
- export web Expo vers `/private/tmp/filfinance-news-082-final`: OK
- harness navigateur sur `320`, `375`, `414`, `768`, `1024`: OK, `scrollWidth == clientWidth`

## Dependances

- `NEWS-063` Installation `NativeWind v4`, tokens et primitives UI
- `NEWS-066` Stabilisation, tests de non-regression et documentation du client

## Note de debug

Ce ticket ne doit pas etre ferme sur la base d'une inspection de code seule. La correction doit etre prouvee par une mesure runtime du navigateur, parce que le probleme observe vient du rendu effectif React Native Web / NativeWind.

La gate runtime a ete satisfaite avec les mesures ci-dessus.
