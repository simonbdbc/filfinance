# NEWS-066 - Stabilisation, non-regression et documentation du client

## Statut

Fait le 1 mai 2026.

## Objectif

Clore `EPIC-013` en verifiant que la refonte frontend est stable, documentee et prete a servir de socle a l'authentification.

## Scope livre

- architecture client finalisee autour de `src/app`, `src/features` et `src/shared`
- `App.tsx` conserve comme bootstrap minimal vers `src/app/FilFinanceApp`
- ecran principal reduit a l'orchestration des hooks feature et au rendu des composants extraits
- services HTTP, WebSocket, audio, historique et helpers de tri dedies
- primitives UI reutilisables dans `src/shared/ui`
- `NativeWind v4` branche et utilise sur l'ecran principal
- suppression du `StyleSheet` partage de l'ecran principal
- documentation client mise a jour apres le renommage en `filfinance-client`
- references obsoletes vers les anciens dossiers corrigees dans les docs d'EPIC

## Audit final

Etat observe au 1 mai 2026:

- `filfinance-client/App.tsx` contient uniquement le chargement CSS global et l'export de `FilFinanceApp`
- `src/app/FilFinanceApp.tsx` fait `386` lignes et garde l'orchestration de l'ecran
- les composants visibles sont repartis dans `src/features/live-feed`, `src/features/history`, `src/features/settings`, `src/features/test-runs`, `src/features/system` et `src/features/audio`
- le protocole audio est separe entre `audioQueue`, `useAudioPlayback`, `useAudioPlayer` et `useAudioSocket`
- `useAudioPlayback` reste volontairement le coordinateur du protocole audio; le decouper davantage serait utile seulement si `/ws/audio` evolue fonctionnellement
- les invariants audio FIFO restent portes par des helpers purs et reutilisables
- la non-regression overflow NativeWind est couverte par `NEWS-082`

## Validations attendues

- `./node_modules/.bin/tsc --noEmit --pretty false`
- export web Expo vers `dist-web-smoke`
- audit `StyleSheet` sur `src/`
- audit des anciennes occurrences produit dans `filfinance` hors `node_modules` et `target`
- verification que les docs principales pointent vers `filfinance-client` et `filfinance-service`

## Validations executees

- `./node_modules/.bin/tsc --noEmit --pretty false`: OK
- export web Expo vers `filfinance-client/dist-web-smoke`: OK
- audit `StyleSheet` sur `filfinance-client/src`: OK, aucune occurrence
- audit des anciennes occurrences produit dans `filfinance` hors `node_modules` et `target`: OK, aucune occurrence
- audit documentaire des anciens chemins client: OK, les references maintenues pointent vers `filfinance-client`

## Criteres d'acceptation

- `EPIC-013` peut etre marque `Fait`
- le prochain chantier frontend peut partir de cette structure sans recreer un monolithe
- `EPIC-014` peut introduire les providers et ecrans d'auth dans `src/app`, `src/features/auth` et `src/shared/api`
- les futures evolutions visuelles doivent etendre les primitives UI plutot que dupliquer des styles inline

## Hors scope restant

- ajouter une librairie de tests frontend
- creer un design system complet
- supporter l'audio natif mobile
- refondre visuellement l'application
