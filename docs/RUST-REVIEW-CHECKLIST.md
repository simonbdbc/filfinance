# Checklist - Review Rust pedagogique

## Objectif

Cette checklist rend operationnelle la regle de commentaires Rust pedagogiques dans `Fil Finance`.

Elle sert de reference courte et opposable pour:

- l'auteur d'un ticket Rust
- le reviewer humain
- la cloture d'un ticket qui modifie du code Rust

Elle complete le standard [RUST-COMMENTING-STANDARD](./RUST-COMMENTING-STANDARD.md) en donnant une procedure concrete de verification.

## Quand cette checklist est obligatoire

Cette checklist est obligatoire des qu'un ticket:

- ajoute un fichier `*.rs`
- modifie un fichier `*.rs`
- modifie un test Rust
- refactore une fonction, un type, un worker ou un contrat Rust existant

Elle s'applique meme si le ticket ne change qu'une petite portion d'un fichier Rust.

## Gate auteur avant review

Avant de demander une review ou de considerer le ticket comme termine, l'auteur doit verifier:

- que chaque ligne Rust ajoutee ou modifiee est commentee en francais
- que les mots-cles et idiomes Rust introduits sont expliques au premier point utile
- que les choix d'architecture locaux sont documentes dans le module ou la fonction concernes
- que les tests Rust modifies sont commentes avec le meme niveau d'exigence
- que le vocabulaire produit reste aligne sur les prefixes `HH:MM. Nouvelle information` et `HH:MM. Information importante`
- que les commentaires ne sont pas devenus faux apres la modification
- que `cargo check` passe si le ticket touche `filfinance-service`
- que `cargo test` passe ou qu'une impossibilite explicite soit documentee

## Gate reviewer

Le reviewer doit traiter comme bloquant tout changement Rust pour lequel au moins un point suivant est vrai:

- une ligne Rust nouvelle ou modifiee n'est pas documentee de facon pedagogique
- un mot-cle Rust important apparait sans explication utile
- un commentaire paraphrase le code sans expliquer la specificite Rust ou le choix d'architecture
- un commentaire est desynchronise du comportement reel
- un test Rust nouveau ou modifie n'explique pas le scenario et le risque de regression
- un terme produit obsolete reintroduit `Alerte marche` ou `Dernière Nouvelle` au lieu des prefixes `HH:MM. Nouvelle information` / `HH:MM. Information importante`
- la verification `cargo check` ou `cargo test` manque sans justification explicite

## Questions minimales de review

Le reviewer doit pouvoir repondre "oui" a toutes les questions suivantes:

1. Un developpeur venant surtout de `JavaScript`, `PHP` ou `Java` peut-il suivre ce changement sans reconstruire seul la logique Rust implicite ?
2. Les notions d'ownership, de partage, d'async, de channels ou de generics introduites ici sont-elles expliquees la ou elles apparaissent ?
3. Le commentaire explique-t-il pourquoi cette solution a ete choisie dans `Fil Finance`, pas seulement ce que le code fait ?
4. Les tests montrent-ils clairement le contrat protege contre regression ?
5. Le code, les commentaires et la narration produit utilisent-ils le meme vocabulaire ?

Si une reponse est "non", la review doit rester bloquante.

## Definition de Done pour tout futur ticket Rust

Un ticket qui touche Rust n'est pas `Done` tant que les points suivants ne sont pas vrais:

- les fichiers Rust modifies respectent le standard [RUST-COMMENTING-STANDARD](./RUST-COMMENTING-STANDARD.md)
- cette checklist a ete satisfaite
- la verification Rust pertinente a ete faite
- le resume de cloture mentionne explicitement les fichiers Rust touches
- le resume de cloture mentionne explicitement l'etat de `cargo check` et `cargo test`

## Regle de redaction des futurs tickets Rust

Tout futur ticket ou EPIC qui implique du code Rust doit contenir explicitement:

- un rappel que le standard pedagogique Rust s'applique
- le scope des fichiers ou modules Rust vises
- la verification attendue `cargo check`, `cargo test`, ou la raison documentee si elle ne peut pas etre faite
- la contrainte que le ticket ne sera pas clos sans commentaires pedagogiques coherents

## Regle pour les revues partielles

Si un ticket ne touche qu'une zone locale d'un gros fichier Rust:

- toute la zone modifiee doit etre remise au niveau du standard
- il n'est pas obligatoire de remettre tout le fichier a niveau dans le meme ticket
- il est interdit de degrader le niveau pedagogique d'une zone deja commentee

## Regle de maintenance

Cette checklist est durable.

Tout futur developpement Rust dans `filfinance-service` doit etre relu contre:

- [RUST-COMMENTING-STANDARD](./RUST-COMMENTING-STANDARD.md)
- [RUST-REVIEW-CHECKLIST](./RUST-REVIEW-CHECKLIST.md)
