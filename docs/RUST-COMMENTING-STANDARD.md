# Standard - Commentaires Rust pedagogiques

## Objectif

Ce standard formalise la facon de commenter le code Rust de `Fil Finance`.

Le public vise n'est pas uniquement un developpeur Rust confirme. Le code doit etre lisible et formateur pour des developpeurs qui viennent surtout de `JavaScript`, `PHP` et `Java`, et qui apprennent Rust au fil de l'avancement du projet.

## Statut

Standard officiel du projet. `EPIC-010` est maintenant clos: `NEWS-046` a pose la regle, `NEWS-047` a audite l'existant, `NEWS-048` et `NEWS-049` ont couvert le code Rust existant, et `NEWS-050` a ajoute les garde-fous de review. Ce document sert donc de reference normative stable pour les reviews et pour les prochains tickets Rust.

## Regle principale

Dans `filfinance-service`, chaque ligne de code Rust doit etre documentee en francais.

Cette exigence s'applique:

- au code existant quand il entre dans le scope d'un ticket
- a tout nouveau code Rust
- aux tests Rust
- aux refactors et aux corrections de bug

Cette regle s'applique aux fichiers `*.rs` de production, aux tests unitaires, aux tests d'integration et aux helpers Rust ajoutes plus tard dans le projet.

## Niveau de commentaire attendu

Le projet ne veut pas de commentaires cosmetiques.

Le bon niveau est:

- expliquer ce que fait la ligne dans le flux metier ou technique
- expliquer la syntaxe Rust qui peut surprendre un developpeur `JavaScript`, `PHP` ou `Java`
- expliquer pourquoi cette forme a ete retenue ici plutot qu'une autre
- expliquer les implications de maintenance, de cout ou de surete quand elles existent

Le mauvais niveau est:

- paraphraser mecaniquement le code
- commenter uniquement les lignes "complexes" et laisser muettes les lignes Rust structurantes
- expliquer le metier sans expliquer la specificite Rust
- expliquer Rust sans expliquer le choix d'architecture du projet

## Forme attendue dans les fichiers Rust

Chaque fichier Rust du projet doit progressivement converger vers cette structure documentaire:

- un commentaire d'entete de module qui explique sa responsabilite dans l'architecture `Fil Finance`
- des commentaires de bloc avant chaque groupe logique important
- des commentaires de ligne sur chaque ligne Rust quand c'est le meilleur moyen de rendre la syntaxe et le flux lisibles
- des commentaires de tests qui expliquent le pourquoi du scenario autant que le comportement attendu

L'objectif n'est pas d'obtenir un "mur de commentaires", mais un code ou un developpeur peu a l'aise en Rust peut suivre l'execution sans devoir inferer seul les regles du langage.

## Ce que les commentaires doivent expliquer

Les commentaires ne doivent pas seulement decrire "ce que fait la ligne". Ils doivent aussi expliquer "pourquoi cette forme Rust est utilisee".

Les commentaires doivent donc couvrir, selon le contexte:

- le role metier de la ligne
- la specificite Rust utilisee
- la raison architecturale du choix
- la bonne pratique suivie
- la contrainte technique prise en compte

Quand une ligne participe a plusieurs dimensions, le commentaire doit les couvrir ensemble. Exemple: un `Arc<AppState>` doit etre explique a la fois comme partage de donnees entre taches et comme choix de possession/partage en Rust.

## Specificites Rust a expliciter

Quand elles apparaissent dans le code, les notions suivantes doivent etre expliquees de facon pedagogique:

- `let`, `mut`, shadowing et mutabilite
- possession, emprunt, references et duree de vie implicite
- `clone`, `move`, partage via `Arc`
- `Option`, `Result`, `match`, `if let`, `while let`
- operateur `?`
- `struct`, `enum`, `impl`, `trait`, `derive`
- `async`, `await`, `tokio::spawn`, channels, concurrence et backpressure
- patterns `axum`, `sqlx`, `serde`
- conversions et ownership autour des `String`, `&str`, `Vec`, `Box`

Cette liste n'est pas limitative. Toute syntaxe, derive, trait ou pattern qui n'est pas immediatement evident pour un non-specialiste Rust doit etre explique.

## Niveau d'explication attendu

Le commentaire doit permettre a un developpeur non expert Rust de:

- comprendre la ligne
- comprendre la syntaxe Rust utilisee
- comprendre pourquoi cette solution a ete choisie dans le projet
- comprendre les implications de maintenance

Un commentaire purement descriptif du type "assigne la valeur a la variable" n'est pas suffisant.

## Vocabulaire produit a conserver dans les commentaires et exemples

Les commentaires ne doivent pas inventer un vocabulaire produit divergent.

Pour les exemples et narrations de news, le prefixe de reference est maintenant:

- `HH:MM. Nouvelle information`
- `HH:MM. Information importante`

Les termes precedents du type `Alerte marche` ou `Dernière Nouvelle` ne doivent plus etre reintroduits dans les commentaires, les exemples et les fixtures quand ils decrivent ce prefixe produit.

## Choix d'architecture a expliquer

En plus des lignes elles-memes, le code doit expliquer les choix de structure du projet:

- pourquoi un module existe
- pourquoi une responsabilite est isolee
- pourquoi un canal ou un worker est separe
- pourquoi une abstraction est introduite
- pourquoi un fallback, un timeout ou un retry est present

Pour `filfinance-service`, cela vise explicitement:

- le decouplage entre ingestion, persistence, traduction, TTS, Telegram et WebSocket
- le choix d'un runtime `tokio` et de workers asynchrones
- l'usage de SQLite comme support MVP et outil de debug
- le partage d'etat via `Arc`
- les contrats de messages transportes sur `/ws/news`, `/ws/audio` et `/ws/test-runs`
- les raisons pour lesquelles un item Telegram, un item WebSocket et un flux audio doivent traverser le meme pipeline canonique

## Regle de review

Un changement Rust n'est pas considere termine si:

- le code n'est pas commente ligne par ligne
- les specificites Rust ne sont pas expliquees
- les choix d'architecture ne sont pas documentes
- les commentaires deviennent faux apres modification

La review doit verifier au minimum:

- que le nouveau code ou le code modifie est couvert ligne par ligne
- que les commentaires aident reellement un developpeur non expert Rust
- que les mots-cles Rust introduits sont expliques au premier contact utile
- que les commentaires restent alignes sur le vocabulaire produit reel
- que les tests Rust sont commentes avec le meme niveau d'exigence

La review humaine doit maintenant s'appuyer aussi sur [RUST-REVIEW-CHECKLIST](./RUST-REVIEW-CHECKLIST.md).

## Gate de fermeture d'un ticket Rust

Un ticket qui touche du Rust ne peut pas etre ferme si au moins un point suivant est faux:

- `cargo check` passe, ou une impossibilite reelle est expliquee
- `cargo test` passe, ou une impossibilite reelle est expliquee
- les fichiers `*.rs` modifies sont commentes selon ce standard
- les tests Rust modifies suivent la meme exigence pedagogique
- le resume de cloture mentionne les fichiers Rust touches et l'etat des verifications

Cette gate s'applique aux corrections, refactors, spikes et tickets techniques, pas seulement aux gros EPICs.

## Gate de review bloquante

Le reviewer doit bloquer la review si au moins un cas suivant apparait:

- une ligne Rust nouvelle ou modifiee n'est pas documentee
- une specificite Rust importante n'est pas expliquee
- un commentaire est devenu faux apres modification
- un commentaire paraphrase le code sans valeur pedagogique
- un test Rust change sans commentaire de scenario ou de risque
- le vocabulaire produit diverge des prefixes `HH:MM. Nouvelle information` et `HH:MM. Information importante`

Le fait que le changement soit petit n'est pas une exception.

## Regle de progression pour la campagne de couverture

La campagne de commentaires Rust se fait ticket par ticket.

Les regles de progression sont:

- `NEWS-046` a pose ce standard et le rend opposable en review
- `NEWS-047` a dresse l'inventaire et le plan de couverture du code Rust existant
- chaque ticket suivant doit livrer un sous-ensemble coherent entierement commente, plutot qu'un saupoudrage partiel sur tout le codebase
- tout nouveau developpement Rust doit respecter ce standard meme si le reste du module n'est pas encore totalement couvert

## Regle pour les commentaires de tests

Les tests Rust ne sont pas des citoyens de seconde zone.

Ils doivent expliquer:

- le scenario metier ou technique reproduit
- le risque de regression cible
- la raison du montage de donnees ou de mocks
- la valeur pedagogique des assertions, surtout quand elles montrent un contrat Rust, SQL, async ou WebSocket

## Regle pour les futurs devs

Ce standard est durable.

Tout futur developpeur Rust qui intervient sur `filfinance-service` doit conserver ce niveau de documentation pedagogique, afin que le projet reste a la fois executable et formateur.

Le couple de reference a utiliser en review est maintenant:

- [RUST-COMMENTING-STANDARD](./RUST-COMMENTING-STANDARD.md)
- [RUST-REVIEW-CHECKLIST](./RUST-REVIEW-CHECKLIST.md)
