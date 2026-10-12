# EPIC-016 - Branding Fil Finance

## Objectif

Aligner le produit sur le nouveau branding:

- nom de marque: `Fil Finance`
- nom de l'app: `filfinance`

L'objectif est de supprimer les anciens libelles produit du code maintenu, des configs applicatives et des docs principales.

## Valeur livree

Le client, les manifests Expo, les identifiants runtime et les documentations principales parlent le meme langage produit. Les utilisateurs voient `Fil Finance`, l'app se declare comme `filfinance`, et les dossiers maintenus portent les noms `filfinance-client` et `filfinance-service` sous le workspace `filfinance`.

## Statut

En cours avance au 1 mai 2026.

## Avancement au 1 mai 2026

- `NEWS-083` est implemente: l'ecran principal affiche `Fil Finance` et le bootstrap charge `FilFinanceApp`.
- `NEWS-084` est implemente: `app.json`, `package.json`, `package-lock.json` et les identifiants mobiles utilisent `filfinance`.
- `NEWS-085` est implemente: `EXPO_PUBLIC_FILFINANCE_WS_URL` est le seul nom public maintenu cote client.
- `NEWS-086` est implemente: le package Cargo, le dossier backend, le binaire Docker, les user agents, logs et helpers locaux utilisent `filfinance-service`.
- `NEWS-087` est implemente: l'audit ne conserve plus les anciens noms produit dans les fichiers maintenus, et le workspace racine est renomme en `filfinance`.

## Scope

Dans cet EPIC:

- remplacer les anciens libelles visibles par `Fil Finance`
- renommer l'app Expo en `filfinance`
- mettre a jour les identifiants de package client maintenus
- migrer la variable client vers `EXPO_PUBLIC_FILFINANCE_WS_URL`
- renommer les identifiants runtime non persistants cote client, comme le processor `AudioWorklet`
- mettre a jour les user agents, logs et helpers locaux cote backend pour utiliser `filfinance`
- mettre a jour les README et la roadmap principale
- renommer les dossiers maintenus en `filfinance-client` et `filfinance-service`
- renommer le workspace racine en `filfinance`

Hors scope:

- migration des variables serveur prefixees `NEWS_`
- migration de base de donnees ou des schemas metier
- refonte visuelle de l'identite graphique
- creation d'assets logo/icones
- publication store iOS/Android

## Definition of Done

- l'interface client affiche `Fil Finance`
- `app.json` declare l'app sous `filfinance`
- `package.json` et `package-lock.json` utilisent `filfinance`
- le WebSocket client lit `EXPO_PUBLIC_FILFINANCE_WS_URL`
- les identifiants runtime client non persistants utilisent `filfinance`
- le backend expose des logs et user agents par defaut alignes sur `filfinance`
- les docs principales referencent `Fil Finance` pour le produit
- les anciennes occurrences produit, techniques et variables publiques ont disparu des fichiers maintenus
- le workspace racine s'appelle `filfinance`, l'app Expo vit dans `filfinance-client` et le backend dans `filfinance-service`
- `npm run typecheck` passe cote client
- `cargo check` passe cote service si un fichier Rust est modifie

## Tickets lies

### `NEWS-083` Branding client Fil Finance

Statut: implemente.

Objectif metier:

Afficher le nouveau nom de marque dans l'ecran principal et les docs client.

Scope:

- renommer le composant applicatif maintenu en `FilFinanceApp`
- remplacer l'ancien titre visible par `Fil Finance`
- ajuster les textes descriptifs pour parler du backend Fil Finance
- mettre a jour le README client

Hors scope:

- refonte UI
- creation de logo ou iconographie

### `NEWS-084` Renommage applicatif Expo `filfinance`

Statut: implemente.

Objectif metier:

Faire declarer l'app comme `filfinance` dans les manifests et metadonnees package.

Scope:

- mettre a jour `app.json`
- mettre a jour `package.json`
- mettre a jour `package-lock.json`
- aligner les identifiants bundle/package mobiles

Hors scope:

- publication store

### `NEWS-085` Migration configuration client WebSocket

Statut: implemente.

Objectif metier:

Remplacer la variable publique de connexion par un nom aligne avec l'app.

Scope:

- ajouter `EXPO_PUBLIC_FILFINANCE_WS_URL`
- documenter la migration dans `.env.example` et le README client

Hors scope:

- changement des endpoints backend
- migration des variables serveur `NEWS_`

### `NEWS-086` Branding backend runtime

Statut: implemente.

Objectif metier:

Aligner les identifiants runtime visibles du service avec `filfinance`.

Scope:

- user agent par defaut
- logs de bootstrap
- prefixes temporaires des scripts et tests TTS
- docstrings des helpers locaux
- nom de package Cargo si valide par `cargo check`
- dossier backend `filfinance-service`

Hors scope:

- changer les routes HTTP/WebSocket

### `NEWS-087` Audit post-migration branding

Statut: implemente.

Objectif metier:

Controler les occurrences restantes apres migration et separer les restes acceptables des dettes a traiter.

Scope:

- auditer les occurrences des anciens noms produit et techniques
- verifier que le workspace racine `filfinance`, le dossier app `filfinance-client` et le dossier backend `filfinance-service` sont coherents dans les docs et configs
- classer les occurrences restantes eventuelles: dependances generees, anciens tickets historiques ou dette a traiter

Hors scope:

- suppression d'artefacts generes
