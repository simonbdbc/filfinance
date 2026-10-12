# EPIC-015 - Niveau FinancialJuice des news

## Objectif

Capturer, normaliser, persister et diffuser le champ `Level` fourni par `FinancialJuice` sur les news issues de `sendUpdates` et `GetPreviousNews`.

Le cas prioritaire est la valeur:

- `active active-critical`

Les valeurs deja observees au 30 avril 2026 sont:

- `active active-critical`
- `news-general`
- chaine vide `""`

## Valeur livree

Le client pourra distinguer les news critiques des news generales sans heuristique sur le titre. Le backend gardera aussi l'information brute source pour permettre un inventaire fiable des valeurs `Level` observees dans le temps.

## Statut

Implementation backend/client en cours avance au 30 avril 2026.

## Avancement au 30 avril 2026

- `NEWS-073` a `NEWS-077` sont implementes dans le repo: parsing, normalisation, persistence, projection API/WebSocket et contrat client.
- `NEWS-078` reste a valider en live: les logs dedies `financial juice news level observed` et `financial juice news level is not documented yet` permettent d'inventorier les valeurs reelles.
- Le runtime local n'a pas encore fourni de run live documente confirmant les valeurs observees apres deploiement.

## Contexte technique

Le collecteur `FinancialJuice` decode aujourd'hui les news live `sendUpdates` et le backlog `GetPreviousNews` dans le meme DTO `FinancialJuiceApiNewsItem`.

Une observation recente a ajoute un audit des champs non mappes via `serde(flatten)`:

- `unmapped_keys` en `debug`
- `unmapped_values` en `debug`
- detection `info` des champs proches de priorite ou criticite

Le champ source a retenir est maintenant connu:

- cle JSON source: `Level`
- type observe: string
- valeur critique cible: `active active-critical`

## Cible de contrat

Le backend doit exposer un champ stable, distinct du champ source brut:

- nom recommande cote modele et API: `news_level`
- type: `Option<String>` ou string nullable selon la projection
- valeur brute conservee telle que fournie par FinancialJuice, apres trim minimal
- `""` source doit etre represente clairement, soit par `None`, soit par une valeur normalisee documentee

Decision recommandee:

- stocker la valeur brute trimmee quand elle est non vide
- convertir la chaine vide en `None`
- calculer un helper metier `is_critical` uniquement si un besoin UI clair apparait
- ne pas renommer `active active-critical` en enum tant que l'inventaire des valeurs n'est pas stabilise

## Scope

Dans cet EPIC:

- capturer `Level` depuis `FinancialJuiceApiNewsItem`
- propager ce niveau dans `NewsEvent`
- persister ce niveau dans `news_events`
- reparer les lignes existantes quand le `raw_payload_ref` permet de retrouver une version plus riche
- exposer le niveau dans les endpoints HTTP et les payloads WebSocket
- afficher ou rendre exploitable ce niveau dans le client
- inventorier les valeurs distinctes observees
- tester le live, le backlog, la persistence et le contrat client

Hors scope:

- classification NLP ou sentiment
- changement de priorisation audio/TTS
- filtrage automatique des news cote serveur
- modele de roles ou auth
- migration hors SQLite

## Definition of Done

- `Level` est parse depuis `sendUpdates`
- `Level` est parse depuis `GetPreviousNews`
- `news_level` est persiste sur `news_events`
- `news_level` est present dans `/api/news`, `/api/news/{id}`, `/api/translations` et `/ws/news`
- le client recoit `news_level` avec chaque news traduite
- la valeur `active active-critical` est identifiable cote client
- l'inventaire des valeurs distinctes observees est documente ou exposable via les logs dedies
- les tests Rust couvrent parsing, persistence et projection API/WebSocket
- les types TypeScript client sont mis a jour
- toute modification Rust respecte le standard [RUST-COMMENTING-STANDARD](../RUST-COMMENTING-STANDARD.md)

## Tickets lies

### `NEWS-073` Cadrage contrat `Level` et inventaire des valeurs

Objectif metier:

Definir le contrat stable `news_level` a partir du champ source `Level` et documenter les valeurs observees.

Scope:

- confirmer les valeurs connues: `active active-critical`, `news-general`, `""`
- definir la normalisation de la chaine vide
- documenter la difference entre valeur brute source et usage produit
- conserver le log `debug` des `unmapped_values` pendant la phase d'observation

Hors scope:

- migration schema
- affichage frontend

Approche technique:

- mettre a jour la documentation de contrat
- ajouter si utile un commentaire dans le collecteur expliquant pourquoi `Level` devient un champ mappe
- garder une liste ouverte des valeurs observees

Criteres d'acceptation:

- le contrat `news_level` est ecrit
- la valeur critique `active active-critical` est explicitement identifiee
- la chaine vide a une representation cible documentee

Tests attendus:

- aucun test obligatoire si ticket purement doc

Dependances:

- observation des logs `FinancialJuice` existants

### `NEWS-074` Backend parsing et modele canonique `news_level`

Objectif metier:

Rattacher le niveau FinancialJuice a chaque `NewsEvent` au moment de la normalisation.

Scope:

- ajouter `Level` dans `FinancialJuiceApiNewsItem`
- ajouter `news_level` dans `NewsEvent`
- normaliser la chaine vide en `None`
- conserver la valeur brute non vide
- documenter chaque ligne Rust modifiee selon le standard pedagogique

Hors scope:

- migration SQLite
- frontend

Approche technique:

- utiliser `#[serde(rename = "Level")]` dans le DTO FinancialJuice
- passer la valeur dans `into_news_event_with_published_at`
- verifier que les chemins `sendUpdates` et `GetPreviousNews` partagent bien le mapping

Criteres d'acceptation:

- une news live avec `Level = "active active-critical"` produit un `NewsEvent.news_level = Some("active active-critical")`
- une news avec `Level = ""` produit `None`
- le champ `Level` ne remonte plus dans `unmapped_keys`

Tests attendus:

- test parsing live
- test parsing backlog
- test normalisation chaine vide

Dependances:

- `NEWS-073`

### `NEWS-075` Persistence SQLite et migration `news_level`

Objectif metier:

Conserver durablement le niveau source avec la news.

Scope:

- ajouter une colonne `news_level` a `news_events`
- mettre a jour les inserts et lectures `PersistedNewsEvent`
- mettre a jour la logique de repair FinancialJuice
- evaluer un index seulement si un filtre par niveau est introduit

Hors scope:

- filtre API par niveau
- migration de donnees distante

Approche technique:

- migration idempotente `ALTER TABLE news_events ADD COLUMN news_level TEXT`
- projection SQL commune mise a jour
- insert/update qui propage `NewsEvent.news_level`
- repair qui met a jour `news_level` si la source renvoie une valeur nouvelle

Criteres d'acceptation:

- les nouvelles news persistent `news_level`
- les lectures recentes renvoient `news_level`
- les anciennes lignes restent compatibles avec `NULL`

Tests attendus:

- test migration
- test insert + lecture
- test dedup ne perd pas le niveau
- test repair met a jour le niveau

Dependances:

- `NEWS-074`

### `NEWS-076` API, WebSocket et traduction: propagation du niveau

Objectif metier:

Renvoyer `news_level` au client avec chaque news exploitable.

Scope:

- ajouter `news_level` dans les payloads de `/api/news`
- ajouter `news_level` dans `/api/news/{id}`
- ajouter `news_level` dans `LiveNewsItem`
- ajouter `news_level` dans `/api/translations`
- ajouter `news_level` dans `/ws/news`
- verifier que Telegram et TTS ne changent pas de comportement

Hors scope:

- affichage frontend riche
- filtrage utilisateur

Approche technique:

- propager le champ depuis `PersistedNewsEvent`
- enrichir `LiveNewsItem` depuis l'evenement source sans denormaliser `news_translations`
- garder la compatibilite nullable pour les anciennes donnees

Criteres d'acceptation:

- chaque payload live traduit contient `news_level`
- chaque item historique traduit contient `news_level`
- les clients existants ne cassent pas si `news_level` vaut `null`

Tests attendus:

- tests handlers HTTP
- test projection `LiveNewsItem`
- test WebSocket ou serialisation de message

Dependances:

- `NEWS-075`

### `NEWS-077` Client: types et affichage du niveau critique

Objectif metier:

Permettre au client d'identifier les news critiques et de recevoir le niveau avec chaque news.

Scope:

- ajouter `news_level` aux types TypeScript `TranslatedNewsItem`
- afficher ou baliser la valeur critique `active active-critical` avec le libelle utilisateur `Élevée`
- documenter les badges historiques: `active -> Élevée`, `news-general -> Général` et `none -> Faible`
- garder un rendu discret pour `news-general`, `null` et les valeurs non reconnues
- verifier historique et live

Hors scope:

- refonte visuelle complete
- preferences utilisateur
- notification systeme

Approche technique:

- mettre a jour les contrats dans `filfinance-client/src/shared/api/contracts.ts`
- ajouter une primitive ou un badge dans la feature live/history si utile
- garder la compatibilite avec les payloads historiques sans niveau

Criteres d'acceptation:

- le client compile avec le nouveau champ
- une news critique est distinguable visuellement ou structurellement
- le rendu reste stable si `news_level` est absent ou `null`

Tests attendus:

- `tsc --noEmit`
- smoke visuel web si le runtime Node local le permet

Dependances:

- `NEWS-076`
- socle `EPIC-013` clos pour placer l'affichage dans la bonne feature

### `NEWS-078` Inventaire runtime et documentation de non-regression

Objectif metier:

Verifier en conditions reelles les valeurs `Level` observees et documenter le comportement final.

Scope:

- collecter les valeurs distinctes vues dans les logs
- documenter les valeurs connues
- verifier que `active active-critical` circule de FinancialJuice au client
- mettre a jour le README client si un affichage est ajoute

Hors scope:

- tableau d'administration complet
- dashboard observabilite dedie

Approche technique:

- conserver temporairement les logs `debug` `unmapped_values`
- ajouter un log dedie si une nouvelle valeur `news_level` non documentee apparait
- documenter le runbook de validation

Contrat de log retenu:

- `debug`: `financial juice news level observed`, avec `raw_news_level` et `news_level`
- `info`: `financial juice news level is not documented yet`, uniquement pour une valeur non vide hors `active active-critical` et `news-general`

Criteres d'acceptation:

- valeurs observees documentees
- test manuel live ou replay documente
- aucun changement de comportement audio/TTS/Telegram non voulu

Tests attendus:

- `cargo check`
- `cargo test`
- `tsc --noEmit`
- test live ou replay documente

Dependances:

- `NEWS-074` a `NEWS-077`

## Points d'attention

- `Level` est une information source FinancialJuice, pas une classification maison.
- Il faut conserver les valeurs brutes non vides tant que le domaine n'est pas stabilise.
- La valeur critique produit est `active active-critical`, mais le code ne doit pas supposer que c'est la seule valeur future importante.
- Les anciennes lignes SQLite auront `news_level = NULL`.
- Tout code Rust modifie par cet EPIC doit respecter la checklist [RUST-REVIEW-CHECKLIST](../RUST-REVIEW-CHECKLIST.md).
