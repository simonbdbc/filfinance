# EPIC-001 - Socle Rust

## Objectif

Stabiliser le socle technique du service pour que toutes les integrations source et toutes les sorties reposent sur un runtime, un modele et un stockage coherents.

## Valeur livree

Une base Rust decision-complete sur laquelle les futurs collecteurs peuvent s'appuyer sans redefinir la config, les types ou la persistence.

## Contexte courant et dependances

Le projet contient deja un squelette Rust avec `tokio`, `axum`, `sqlx`, un `AppConfig`, un stockage SQLite, et des collecteurs branches mais encore minimaux. Cet EPIC ne depend d'aucun autre; il sert de prerequis a toute la roadmap.

## Definition of Done

- le chargement de configuration est centralise et documente
- le service possede un modele interne canonique `NewsEvent`
- la taxonomie d'erreurs est exploitable dans le pipeline et les collecteurs
- le bootstrap SQLite et les migrations sont idempotents
- les collecteurs partagent la meme abstraction de cycle de vie

## Risques et points durs

- figer trop tot des types trop pauvres pour de futures sources externes
- multiplier les conventions de config par source
- lier trop fortement l'ingestion au schema SQLite courant
- garder des collecteurs "special cases" impossibles a etendre proprement

## Liste des tickets

- `NEWS-001` Contrat de configuration `.env` et conventions runtime
- `NEWS-002` Abstraction collector et cycle de vie des sources
- `NEWS-003` Modele `NewsEvent`, normalisation et taxonomie d'erreurs
- `NEWS-004` Strategie SQLite WAL, migrations et bootstrap dev

## Details des tickets

### `NEWS-001` Contrat de configuration `.env` et conventions runtime

**Objectif**  
Definir un contrat de configuration unique pour le service et ses sources.

**Scope**  
Centraliser les variables d'environnement, leur validation, leurs valeurs par defaut et leur documentation. Encadrer aussi les conventions de runtime: listen address, user-agent, buffers, TTL de dedup, timeouts et flags source.

**Hors scope**  
Ajout d'un systeme de secrets externe ou de profils multi-environnement complets.

**Approche technique**  
Structurer `AppConfig` autour de blocs explicites (`runtime`, `storage`, `source`, `telegram`) et documenter pour chaque variable: type, defaut, obligatoire ou non. Faire remonter les erreurs de config en message clair au demarrage. Garder `.env.example` comme reference contractuelle.

**Criteres d'acceptation**

- toute variable utilisee par le service est declaree dans `.env.example`
- aucune lecture d'environnement directe ne subsiste hors du module de config
- les erreurs de validation identifient la variable fautive et la cause
- les valeurs par defaut sont explicites et stables

**Tests attendus**

- parsing d'une config minimale valide
- rejet d'une valeur invalide de type bool/int/url/intervalle
- verification des defauts sur un environnement vide

**Dependances**

- aucune

### `NEWS-002` Abstraction collector et cycle de vie des sources

**Objectif**  
Unifier le comportement de tous les collecteurs.

**Scope**  
Definir un contrat commun pour demarrage, polling, arret, nom logique de la source, metadonnees de sante et emission d'evenements.

**Hors scope**  
Le parsing specifique de `Financial Juice` ou d'une future source externe.

**Approche technique**  
Introduire une abstraction runtime par source, capable de produire des `NewsEvent` ou des erreurs typées. Encapsuler l'intervalle, le jitter, le client HTTP et le `ingest_tx` dans le runner commun. Chaque source ne garde que sa logique d'acces et de parsing.

**Criteres d'acceptation**

- les deux collecteurs existants utilisent le meme contrat de cycle de vie
- l'enregistrement des sources pour `/api/sources` ne depend plus d'un assemblage manuel fragile
- les logs de demarrage, erreur et arret utilisent un format commun

**Tests attendus**

- test du runner avec un collecteur factice qui emet des evenements
- test d'un collecteur desactive qui ne demarre pas
- test de propagation d'erreur non fatale dans une boucle de polling

**Dependances**

- `NEWS-001`

### `NEWS-003` Modele `NewsEvent`, normalisation et taxonomie d'erreurs

**Objectif**  
Definir le modele metier canonique du flux.

**Scope**  
Introduire un type `NewsEvent` commun pour l'ingestion, la persistence et les sorties, avec champs obligatoires, metadata optionnelles et signature de dedup. Formaliser aussi les categories d'erreurs: config, fetch, parse, storage, downstream.

**Hors scope**  
L'enrichissement avance par NLP ou classification de sentiment.

**Approche technique**  
Remplacer la distinction actuelle `IncomingNewsItem` / `StoredNewsItem` comme modele metier principal par un `NewsEvent` normalise, puis creer des projections vers la base et les payloads de sortie. Garder un modele ouvert aux futures sources sans les ajouter avant decision produit explicite.

**Criteres d'acceptation**

- le pipeline manipule un modele interne unique
- les payloads exposes en HTTP/WS sont derives de ce modele sans logique cachee
- chaque erreur metier tombe dans une categorie explicite
- la signature de dedup est stable et documentee

**Tests attendus**

- serialisation/deserialisation du modele
- generation stable de signature a partir d'un meme evenement
- mapping des erreurs techniques vers la taxonomie applicative

**Dependances**

- `NEWS-001`
- `NEWS-002`

### `NEWS-004` Strategie SQLite WAL, migrations et bootstrap dev

**Objectif**  
Rendre la persistence locale fiable et previsible pour le MVP.

**Scope**  
Documenter et implementer le schema SQLite, les indexes minimaux, le mode `WAL`, les migrations, l'idempotence du bootstrap, et les regles de compatibilite schema/code.

**Hors scope**  
Passage a Postgres ou a une architecture de stockage distribuee.

**Approche technique**  
Formaliser une table principale d'evenements et, si necessaire, une table annexe de quarantaines/erreurs pour la suite. Encadrer la creation de la base, la creation du dossier `data/`, l'ordre des `PRAGMA`, et la strategie de migration au demarrage.

**Criteres d'acceptation**

- le service cree ou migre la base sans intervention manuelle
- `WAL` et `synchronous` sont forces par le bootstrap
- les indexes couvrent au minimum `source`, `ingested_at`, `signature`
- les migrations sont idempotentes et rejouables

**Tests attendus**

- bootstrap sur base absente
- redemarrage sur base existante
- insertion et lecture sur schema migre

**Dependances**

- `NEWS-003`
