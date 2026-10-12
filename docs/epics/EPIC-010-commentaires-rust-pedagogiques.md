# EPIC-010 - Commentaires Rust pedagogiques

## Objectif

Transformer le code Rust de `filfinance-service` en support d'apprentissage continu pour des developpeurs qui viennent surtout de `JavaScript`, `PHP` et `Java`, en imposant une documentation pedagogique exhaustive directement dans le code.

## Valeur livree

Un backend Rust lisible, explicable et maintenable par une equipe qui n'a pas encore une forte maturite Rust, sans devoir reconstruire en permanence le sens des mots-cles, des patterns d'ownership et des choix d'architecture.

## Statut

Cloture.

`NEWS-046` a pose le standard, `NEWS-047` a audite l'existant, `NEWS-048` et `NEWS-049` ont couvert le code Rust existant, et `NEWS-050` a formalise les garde-fous de review. `EPIC-010` peut maintenant etre considere comme livre.

## Contexte courant et dependances

Le projet `filfinance` avance vite sur le backend Rust, avec des modules de pipeline, de traduction, de TTS, de WebSocket et de persistence deja riches. Cette vitesse de delivery cree maintenant un risque de comprehension et de transmission pour des developpeurs non experts Rust.

Cet EPIC est transverse:

- il ne depend d'aucun autre EPIC pour etre ouvert
- il s'applique a tout le code Rust existant de `filfinance-service`
- il devient une condition d'execution et de done pour les futurs EPICs Rust

## Definition of Done

- un standard projet ecrit formalise la facon de commenter le Rust dans `filfinance`
- chaque ligne de code Rust du scope cible est commentee en francais
- les mots-cles, idiomes et specificites Rust sont expliques au fil du code
- les choix d'architecture, de decouplage, de runtime async et de persistence sont explicitement documentes
- les tests Rust suivent la meme regle pedagogique que le code de production
- les futurs tickets Rust ne peuvent plus etre consideres done sans cette couverture documentaire

## Risques et points durs

- volume de travail important sur un codebase deja actif
- risque de commentaires pauvres ou mecaniques qui repetent le code sans enseigner Rust
- risque de drift entre code et commentaires si la regle n'est pas integree a la review
- risque de conflits de merge si la campagne de commentaires est trop large et non decoupee

## Liste des tickets

- `NEWS-046` Standard projet de commentaires Rust pedagogiques
- `NEWS-047` Audit du code Rust existant et plan de couverture
- `NEWS-048` Commentaires pedagogiques du socle et des types centraux
- `NEWS-049` Commentaires pedagogiques des modules runtime, pipeline et integrations
- `NEWS-050` Garde-fous de review pour tout futur developpement Rust
- `NEWS-168` Re-audit EPIC-010 et rattrapage commentaires Rust

## Etat courant des tickets

| Ticket | Statut | Commentaire |
| --- | --- | --- |
| `NEWS-046` | Fait | Le standard officiel est ecrit, reference par les docs centrales, et aligne sur les prefixes produit `HH:MM. Nouvelle information` / `HH:MM. Information importante` pour les narrations de news. |
| `NEWS-047` | Fait | L'inventaire exhaustif et le plan de couverture sont documentes dans `docs/tickets/NEWS-047-audit-couverture-rust.md`. |
| `NEWS-048` | Fait | `main.rs`, `config.rs`, `error.rs`, `state.rs`, `api.rs`, `models.rs` et `test_runs.rs` sont consolides pedagogiquement, et `cargo check` / `cargo test` passent apres revue du lot socle. |
| `NEWS-049` | Fait | `pipeline.rs`, `storage.rs`, `collectors/`, `translation.rs`, `tts.rs`, `telegram.rs` et `websocket.rs` ont recu une vague de commentaires pedagogiques, et `cargo check` / `cargo test` passent toujours. |
| `NEWS-050` | Fait | Les garde-fous de review sont maintenant formalises dans les docs centrales, avec checklist de review, gate de fermeture et references explicites depuis la roadmap et le README du service. |
| `NEWS-168` | En cours | Rattrapage cible apres drift: `NEWS-047` est re-audite sur l'arborescence Rust actuelle; prochaine vague: couvrir pedagogiquement `health` et `logs/tests`, puis `HLS`, `edge`, `ingestion`, `audio` et `live`. |

## Details des tickets

### `NEWS-046` Standard projet de commentaires Rust pedagogiques

**Objectif**  
Ecrire la regle officielle du projet pour la documentation du code Rust.

**Scope**  
Produire un document de reference partage qui formalise:

- le public vise: developpeurs venant surtout de `JavaScript`, `PHP` et `Java`
- l'obligation de commenter chaque ligne de code Rust
- l'obligation d'expliquer les mots-cles et idiomes Rust
- l'obligation d'expliquer les choix d'architecture et les bonnes pratiques du projet
- les attentes de review pour les futurs tickets Rust

**Hors scope**  
Refactorer le code Rust lui-meme ou reecrire les modules metier.

**Approche technique**  
Creer un standard documentaire stable dans `docs/`, reference par la roadmap et par les futurs EPICs. Y formaliser le niveau d'explication attendu pour `let`, `mut`, `match`, `impl`, `trait`, `enum`, `struct`, `Arc`, `Option`, `Result`, `async`, `await`, `?`, ownership, borrowing, move, clone et patterns `tokio` / `axum` / `sqlx`.

**Criteres d'acceptation**

- un document projet formalise la regle
- la roadmap reference explicitement cette regle
- la regle dit clairement qu'elle s'applique au code existant et futur

**Tests attendus**

- revue documentaire manuelle

**Dependances**

- aucune

**Etat courant**

- document de reference ecrit dans `docs/RUST-COMMENTING-STANDARD.md`
- vocabulaire projet aligne sur les prefixes horaires locaux `HH:MM. Nouvelle information` et `HH:MM. Information importante`
- ticket considere comme couvert documentalement
- reste a faire: ancrage operationnel dans les prochains tickets Rust

### `NEWS-047` Audit du code Rust existant et plan de couverture

**Objectif**  
Decouper la campagne de commentaires Rust de facon actionnable.

**Scope**  
Passer en revue tous les fichiers `*.rs` du service, identifier leur niveau actuel de commentaire, regrouper les zones critiques et produire un plan d'execution module par module.

**Hors scope**  
Ajouter les commentaires eux-memes ligne par ligne dans ce ticket.

**Approche technique**  
Classer les fichiers par familles:

- socle et bootstrap
- modele et erreurs
- persistence et pipeline
- collecteurs et integrations
- translation, TTS, WebSocket et API

Le plan doit prioriser les zones les plus difficiles pour un non-specialiste Rust.

**Criteres d'acceptation**

- tous les fichiers Rust du service sont listes
- chaque fichier a un statut de couverture
- un ordre de traitement clair est defini

**Tests attendus**

- verification manuelle du plan de couverture

**Dependances**

- `NEWS-046`

**Etat courant**

- audit complet produit dans `docs/tickets/NEWS-047-audit-couverture-rust.md`
- 16 fichiers Rust inventories
- 7031 lignes auditees
- ordre de traitement fixe pour `NEWS-048` puis `NEWS-049`

### `NEWS-048` Commentaires pedagogiques du socle et des types centraux

**Objectif**  
Rendre lisibles les modules Rust qui servent de socle mental au reste du projet.

**Scope**  
Documenter de facon ligne par ligne et pedagogique au minimum:

- `main.rs`
- `config.rs`
- `error.rs`
- `models.rs`
- `state.rs`
- `api.rs`

**Hors scope**  
Les modules d'integration metier plus specifiques comme `translation`, `tts`, `telegram` ou `websocket`.

**Approche technique**  
Ajouter des commentaires de ligne et de bloc qui expliquent:

- les types possedes vs empruntes
- les `Arc`, les clones, les derives et les enums
- le role des `Result`, `Option`, `match` et `?`
- les conventions d'API et de modele choisies dans le projet

**Criteres d'acceptation**

- tous les fichiers du scope sont commentes ligne par ligne
- un developpeur non expert Rust peut suivre le bootstrap et les types centraux
- les choix d'architecture de base sont explicitement expliques

**Tests attendus**

- revue manuelle de lisibilite
- verification de compilation sans regression

**Dependances**

- `NEWS-046`
- `NEWS-047`

**Etat courant**

- socle commente dans `main.rs`, `config.rs`, `error.rs`, `state.rs`, `api.rs`, `models.rs` et `test_runs.rs`
- `cargo check` et `cargo test` passent apres cette vague
- revue de consolidation effectuee sur le lot socle avant cloture du ticket

### `NEWS-049` Commentaires pedagogiques des modules runtime, pipeline et integrations

**Objectif**  
Documenter les parties les plus "Rust runtime" du projet, la ou l'on rencontre le plus de specificites `tokio`, `axum`, `sqlx` et ownership async.

**Scope**  
Documenter de facon ligne par ligne et pedagogique au minimum:

- `storage.rs`
- `pipeline.rs`
- `collectors/`
- `translation.rs`
- `tts.rs`
- `telegram.rs`
- `websocket.rs`

**Hors scope**  
Refonte metier ou refactor massif de ces modules.

**Approche technique**  
Expliquer au fil du code:

- canaux `broadcast`, `mpsc`, `watch`
- taches `tokio`
- handlers `axum`
- patterns de retry, timeout, warmup et shutdown
- rapport entre choix d'architecture et contraintes produit
- arbitrages entre simplicite, performance et fiabilite

**Criteres d'acceptation**

- tous les fichiers du scope sont commentes ligne par ligne
- les zones async et concurrentes sont explicitement expliquees
- les integrations externes ont un commentaire d'intention et de contrainte

**Tests attendus**

- revue manuelle de lisibilite
- verification de compilation et de tests sans regression

**Dependances**

- `NEWS-046`
- `NEWS-047`
- `NEWS-048`

**Etat courant**

- le lot `pipeline.rs`, `storage.rs`, `collectors/mod.rs` et `collectors/financial_juice.rs` est commente pedagogiquement
- le lot runtime `translation.rs`, `tts.rs`, `telegram.rs` et `websocket.rs` est commente pedagogiquement
- les commentaires couvrent explicitement `tokio::select!`, `broadcast`, `mpsc`, `watch`, `SplitSink`, les handlers `axum`, les strategies de timeout/retry/warmup/shutdown et le contrat des flux WebSocket
- `cargo check` et `cargo test` passent apres cette vague

### `NEWS-050` Garde-fous de review pour tout futur developpement Rust

**Objectif**  
Faire de la documentation pedagogique Rust une contrainte durable, pas une campagne ponctuelle.

**Scope**  
Mettre a jour les regles projet, la roadmap et les criteres de done pour imposer cette exigence sur tous les futurs changements Rust.

**Hors scope**  
Automatisation CI avancee ou linting semantique de commentaires.

**Approche technique**  
Ancrer la regle a plusieurs niveaux:

- consigne permanente de developpement
- standards de tickets
- dependances inter-EPIC
- definition de done des phases Rust

L'objectif est que tout futur dev Rust sache que la norme du projet est pedagogique par defaut.

**Criteres d'acceptation**

- la regle est visible dans les docs centrales
- les futurs tickets Rust mentionnent explicitement la contrainte
- la review humaine peut s'appuyer sur un texte de reference unique

**Tests attendus**

- revue documentaire manuelle

**Dependances**

- `NEWS-046`

**Etat courant**

- la checklist de review est formalisee dans `docs/RUST-REVIEW-CHECKLIST.md`
- le standard `docs/RUST-COMMENTING-STANDARD.md` contient maintenant une gate de review et une gate de fermeture de ticket Rust
- la roadmap et le README du service referencent explicitement ces garde-fous

## Conclusion d'EPIC

L'objectif de cet EPIC etait double:

- couvrir pedagogiquement le code Rust existant de `filfinance-service`
- faire de cette exigence une regle durable de review pour tous les futurs changements Rust

Ces deux objectifs sont maintenant atteints:

- les 16 fichiers Rust identifies par `NEWS-047` ont ete repris par `NEWS-048` et `NEWS-049`
- les regles de maintien sont posees par `NEWS-050`

L'EPIC peut donc etre clos. La suite du projet doit desormais appliquer ce standard comme une contrainte de base, sans reouvrir un chantier separe de commentaires Rust.

Note de maintien du 12 mai 2026: l'audit historique reste vrai pour le scope traite a l'epoque,
mais l'arborescence Rust a fortement grandi. `NEWS-168` est ouvert comme rattrapage cible pour
remettre `NEWS-047` a jour et couvrir les modules recents les plus risques.
