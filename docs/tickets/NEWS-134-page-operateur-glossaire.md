# NEWS-134 - Page operateur glossaire metier

## Objectif

Ajouter une page frontend operateur pour gerer le glossaire metier sans intervention technique.

La page doit rendre visible le contrat:

```text
EN -> FR -> Vocal
```

Exemple:

```text
ECB -> BCE -> Banque Centrale Europeenne
```

## Priorite

Priorite 2.

Ce ticket depend de `NEWS-132`. Il peut etre livre avant ou apres `NEWS-133`, mais les libelles doivent deja correspondre au contrat pipeline.

## Scope frontend

- Ajouter une entree sidebar visible pour `moderator/admin`.
- Ajouter une page `Glossaire`.
- Lister les entrees sous forme de tableau dense:
  - terme EN;
  - traduction FR;
  - vocal TTS;
  - mode de matching;
  - priorite;
  - actif/inactif.
- Ajouter recherche texte.
- Ajouter filtre `actifs uniquement`.
- Ajouter creation.
- Ajouter edition.
- Ajouter desactivation/suppression.
- Afficher les erreurs API et les validations de formulaire.
- Afficher un selecteur explicite `Mot entier` / `Exact` / `Motif` pour `match_mode`.
- En mode `Motif`, afficher l'aide `<number>` et l'exemple horaire
  `<number>h<number> -> <number> heure <number>`.
- Valider localement les captures `<number>` et la priorite avant sauvegarde.
- Garder le theme clair/sombre coherent avec les tokens existants.

## UX attendue

- Le tableau doit etre scannable, pas une page marketing.
- Les actions principales doivent etre directes:
  - ajouter une entree;
  - sauvegarder;
  - desactiver;
  - annuler.
- Le champ `spoken_term` doit etre optionnel, mais visible.
- La page doit indiquer clairement que `spoken_term` concerne uniquement l'audio.
- Le mode `Motif` doit etre utilisable sans connaitre le nom technique `pattern`.
- L'operateur doit voir les exemples `<number>` et `<string>`, notamment
  `16h01 -> seize heure zero un` et `ECB's Nagel -> Nagel de la BCE`.

## Critere d'acceptation

- Un operateur peut creer `ECB -> BCE -> Banque Centrale Europeenne`.
- Un operateur peut creer une regle `Motif` avec
  `<number>h<number> -> <number>h<number> -> <number> heure <number>`.
- Un operateur peut creer une regle `Motif` avec
  `ECB's <string> -> <string> de la BCE`.
- `<string>` capture un token lexical sans ponctuation terminale; une regle
  separee `<string>: -> <string> :` couvre `NASDAQ:` et `ECB's Nagel:`.
- Les erreurs locales bloquent les motifs sans capture supportee ou avec un nombre de
  captures incoherent entre `source_term`, `translated_term` et `spoken_term`.
- Un listener ne voit pas la page.
- Une entree modifiee est relue depuis l'API apres sauvegarde.
- Les erreurs de validation backend sont affichees.
- `npm run typecheck` passe.
- `npm run build:web` passe.

## Hors scope

- Import/export CSV.
- Bulk edit.
- Historique d'audit.
- Suggestions automatiques depuis OPUS-MT.
