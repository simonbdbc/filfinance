# Fil Finance App

App Expo/React Native `filfinance` pour ecouter le flux `translated_news_item` emis par le backend Fil Finance.

## Documentation

- feuille de route partagee: [../../docs/ROADMAP.md](../../docs/ROADMAP.md)
- cadrage audio/traduction: [../../docs/TOPIC-001-flux-audio-francais-news.md](../../docs/TOPIC-001-flux-audio-francais-news.md)
- service auth: [../../filfinance-auth/README.md](../../filfinance-auth/README.md)

## Objectif

- se connecter au WebSocket `ws://.../ws/news`
- ignorer les messages `heartbeat` de maintien de connexion
- recevoir les messages `translated_news_item`
- afficher le feed utilisateur en francais: titre toujours visible, contenu traduit replie par defaut et ouvert au clic
- ne pas afficher le texte anglais source dans le feed utilisateur
- afficher l'importance utilisateur `Élevée` quand `news_level` FinancialJuice contient le mot source `active`
- charger l'historique journalier en separant le feed reel `financial_juice` et les `test_run`
- ne jamais declencher de lecture automatique pendant le chargement de l'historique
- lire automatiquement uniquement les vraies news live nouvelles, jamais un item deja present dans l'historique local
- utiliser la file locale uniquement si une ou plusieurs news live arrivent pendant qu'une lecture audio est deja en cours
- rejeter de la file audio toute news dont le timestamp est anterieur a l'item en lecture, ou anterieur au dernier item recu si rien ne joue
- conserver toutes les news dans l'historique visible, meme lorsqu'elles sont rejetees de la file audio
- lire les clips HLS finalises annonces par `audio_ready`, puis detruire le player quand la queue est vide
- afficher la voix TTS serveur quand elle est connue, sans canal WebSocket audio de changement a chaud
- exposer les voix multi-speaker Piper sous la forme `Language-Voice-Quality-Speaker`
- rejouer l'audio de l'historique journalier depuis le client
- afficher le bouton de resume audio journalier avec une icone SVG d'etat: play `Résumé audio du jour`, refresh `Résumé en préparation`, cross `Résumé indisponible`, stop carre `Arrêter la préparation` ou `Arrêter la lecture`
- ne gerer aucun fichier audio

## Surfaces utilisateur actuelles

- `/feed` affiche `Fil d'actualités`: heure, titre FR et badge discret avec chevron quand un detail existe; un clic affiche ou masque le corps FR complet.
- `/calendar` affiche `Calendrier hebdo`: synthese du jour, statut audio en haut a droite du bloc synthese, filtres partages par les prochains evenements et les evenements fusionnes.
- `/profile` affiche la session, le role et les preferences avec libelles francais accentues (`Voix TTS préférée`, `Thème`, `Mise à jour`, `Création`) sans exposer l'ID technique du profil.
- `/admin` affiche `Supervision admin` pour les roles autorises.
- `/admin/history` affiche `Historique actualités`: metadonnees en tete, badge de niveau en deuxieme ligne a gauche, puis blocs anglais source, francais traduit et TTS.
- Les badges de niveau historique sont limites a cette page: `active -> Élevée` quand `news_level` contient `active`, `news-general -> Général` quand `news_level` vaut `news-general`, et `none -> Faible` pour tout le reste.
- Le badge `active -> Élevée` conserve les deux termes volontairement: `active` explique la condition technique, car `news_level` contient le mot source `active`; `Élevée` explique le libelle utilisateur applique dans le fil d'actualités.
- `/admin/blacklist` permet de filtrer les news par titre, contenu ou titre + contenu; une regle ciblee `Contenu` retire le contenu de la news live tout en conservant son titre. Le bouton `Update Blacklist` applique les regles actives aux news `financial_juice` deja enregistrees du jour courant: `Titre` et `Titre + contenu` suppriment les news matchees, tandis que `Contenu` vide le contenu puis retraduit sans broadcast live.
- `/admin/calendar-history` affiche `Historique calendrier`: historique par jour avec filtres communs, sans bloc prochains evenements, et details complets par evenement.
- `/admin/glossary` permet de gerer le glossaire et de lancer `Update glossaire`, qui retraduit les news `financial_juice` du jour courant uniquement quand leur titre ou contenu matche une regle active plus recente que la traduction existante.

## Architecture frontend

Le client est en prototype `EPIC-022` vers une architecture `expo-router` inspiree de `maroutine`, sans changement visuel:

- `app/`: routes web directes et layouts `expo-router`
- `src/application`: layouts applicatifs, navigation, controle audio global et provider runtime racine
- `src/features/*/screens`: ecrans fins consommes par les routes
- `src/features/*/index.ts`: exports publics par feature
- `src/features/audio`: player HLS clips/replay, helpers d'admission FIFO et deduplication
- `src/features/calendar`: calendrier hebdomadaire, filtres partages et historique calendrier admin
- `src/features/history`: chargement HTTP pagine de l'historique journalier et cartes feed / `test_run`
- `src/features/live-feed`: blocs d'affichage du live, item courant et hook WebSocket `/ws/news`
- `src/features/settings`: carte de connexion backend, URL WebSocket et reglages audio web
- `src/features/system`: affichage transversal des erreurs
- `src/features/test-runs`: cartes de declenchement E2E, trace du run et hook WebSocket `/ws/test-runs`
- `src/shared/api`: contrats frontend HTTP/WebSocket et helpers d'URL
- `src/shared/config`: constantes client partagees
- `src/shared/lib`: tri, dates, formatage et deduplication purs
- `src/shared/ui`: primitives UI reutilisables (`Card`, `ActionButton`, `Badge`, `ChevronIcon`, `StatTile`, `Typography`), carte d'acces refuse et `select` web encapsule
- `src/shared/theme`: provider theme et tokens clair/sombre conservant la palette actuelle

`App.tsx` est conserve comme compatibilite passive; l'entree runtime est maintenant `expo-router/entry`.
Le socle est **gluestack-ui v5 + UniWind + Tailwind CSS 4**. Metro utilise `withUniwindConfig`, `global.css` et `uniwind-types.d.ts`. NativeWind, css-interop, Tailwind 3 et leur préparation de cache sont retirés. Voir [le guide technique](../../docs/guides/pwa-ui-stack.md).
Les routes principales sont `/login`, `/feed`, `/calendar`, `/profile`, `/admin` et les sous-routes admin.

## Validation frontend

La gate de cloture `NEWS-066` couvre:

- typecheck TypeScript
- export web Expo smoke
- audit d'absence de l'ancien `StyleSheet` partage sur l'ecran principal
- audit des anciens noms produit apres renommage
- verification documentaire de l'architecture `app / features / shared`
- contrat historique actualités: libelle calendrier, badges de niveau et libelles `Contenu`
- non-regression overflow documentee par `NEWS-082`

## Prerequis

- Node.js `24.13.1` et npm `>=11.8.0 <12`, comme dans `package.json` et la CI
- Expo CLI via `npx expo`
- backend Fil Finance lance sur une machine accessible depuis le telephone ou l'emulateur

## Configuration

Copier `.env.example` vers `.env` puis ajuster:

```env
EXPO_PUBLIC_FILFINANCE_WS_URL=ws://192.168.1.10:8088/ws/news
EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL=
EXPO_PUBLIC_FILFINANCE_PWA_URL=https://app.dev.filfinance.fr
EXPO_PUBLIC_API_URL=http://192.168.1.10:8088
EXPO_PUBLIC_BETTER_AUTH_URL=http://192.168.1.10:3000
```

Utiliser l'adresse IP locale de la machine qui execute le backend Fil Finance, pas `localhost`, si l'app tourne sur un vrai telephone.

`EXPO_PUBLIC_FILFINANCE_PWA_URL` doit etre l'origine HTTPS de la PWA du meme
environnement. Les emails d'activation et de reinitialisation y reviennent aussi
depuis l'app native; aucun cookie de session ni token de reset n'est place dans un
custom scheme interceptable.

Le client utilise Better Auth via `filfinance-auth`. Les sessions sont stockees par l'integration Expo dans `expo-secure-store`.

## Lancement

```bash
npm ci
npx expo start
```

Validation statique:

```bash
npm run qa:frontend
npm run typecheck
npm run build:web
```

Puis ouvrir l'app dans:

- Expo Go sur iOS/Android
- un emulateur Android
- le simulateur iOS

## Versions attendues

Le client est aligne sur Expo SDK 55 avec:

- `expo` `~55.0.23`
- `expo-asset` `~55.0.17`
- `expo-crypto` `~55.0.17`
- `expo-linking` `~55.0.16`
- `expo-network` `~55.0.16`
- `expo-router` `~55.0.17`
- `expo-secure-store` `~55.0.16`
- `expo-status-bar` `~55.0.6`
- `expo-web-browser` `~55.0.18`
- `react` / `react-dom` `19.2.0`
- `react-native` `0.83.10`
- `react-native-gesture-handler` `~2.30.0`
- `react-native-screens` `~4.23.0`
- `react-native-web` `^0.21.0`

Verifier cet alignement avec `npx expo install --check`. Eviter
`npm audit fix --force`: npm propose actuellement un downgrade incompatible avec
Expo SDK 55. Le lockfile versionne est la source reproductible; apres un nettoyage
du `node_modules` genere, restaurer le lockfile Git puis utiliser `npm ci`.

### Avis npm residuels

Au 12 aout 2026, `npm audit --omit=dev` conserve deux avis transitifs dans
l'outillage Expo: `image-size` via Metro (niveau high, aucun correctif publie) et
`uuid@7` via le parseur de projet Xcode (niveau moderate, le chemin vulnerable
`v3/v5/v6` avec buffer externe n'est pas utilise). Ces paquets ne sont pas livres
dans `dist`; seuls des assets versionnes et relus doivent entrer dans le build.
Suivre les mises a jour Expo/Metro et ne pas forcer de version majeure isolee.

Le payload WebSocket reste volontairement compact pour le live. Les vues historiques ou les expansions qui ont besoin d'un detail complet utilisent HTTP, notamment `GET /api/translations` et `GET /api/translations/{id}`.

## Contrat attendu

Le client attend des messages JSON de type:

```json
{
  "type": "translated_news_item",
  "payload": {
    "id": 12,
    "news_event_id": 7,
    "language": "fr",
    "translated_title": "Powell indique que les baisses de taux dependent des donnees",
    "narration_text": "14:37. Powell indique que les baisses de taux dependent des donnees.",
    "source": "financial_juice",
    "translation_mode": "heuristic_fr",
    "news_level": "active active-critical",
    "created_at": "2026-04-21T09:30:02Z",
    "updated_at": "2026-04-21T09:30:02Z",
    "status": "ready",
    "event_kind": "headline",
    "source_title": "Powell says rate cuts stay data-dependent",
    "source_body": null,
    "translated_body": null,
    "url": "https://example.test/item",
    "timestamp": "2026-04-21T09:30:00Z",
    "error_message": null
  }
}
```

## Limites du client

- lecture audio ciblee pour le web via HLS clips/replay
- aucune persistence locale de la file
- authentification Better Auth avec session SecureStore et JWT court pour l'API Rust
- creation de compte avec mot de passe de 10 a 128 caracteres, confirmation sur `/email-verification` et reset geres par Better Auth
- pas de lecture en arriere-plan systeme
- l'historique charge au demarrage sert d'affichage et de base de dedup, pas de source d'autoplay
- la file audio doit rester FIFO par timestamp metier: admission monotone, tri du plus ancien au plus recent, historique visible toujours trie du plus recent au plus ancien
- resynchronisation audio ciblee sur les seuls items live encore en attente lors d'une reconnexion WebSocket
- l'historique journalier news et calendrier est pilote par le jour local selectionne cote client
- validation audio native hors scope: le MVP audio reste cible sur le web via `Web Audio API`
