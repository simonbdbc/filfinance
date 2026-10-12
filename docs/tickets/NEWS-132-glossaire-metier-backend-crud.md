# NEWS-132 - Glossaire metier backend CRUD

## Objectif

Ajouter un glossaire metier persistant pilotable par les operateurs, pour maitriser la traduction affichee et la vocalisation TTS sans redeployer le backend.

Le contrat cible separe explicitement:

```text
EN source -> FR affiche -> FR vocal
```

Exemple:

```text
ECB -> BCE -> Banque Centrale Europeenne
```

## Priorite

Priorite 1.

Ce ticket ouvre le lot glossaire. Il doit etre fait avant l'integration pipeline et avant la page frontend.

## Modele de donnees cible

Table applicative:

```text
translation_glossary_entries
```

Champs minimaux:

- `id`
- `source_term`
- `translated_term`
- `spoken_term`
- `source_language`
- `target_language`
- `match_mode`
- `case_sensitive`
- `priority`
- `enabled`
- `notes`
- `created_at`
- `updated_at`

Modes de matching MVP:

- `exact`
- `word_boundary`
- `pattern`

Le mode `pattern` supporte les captures `<number>` et `<string>`. `<number>` traite
des cas operateur comme les horaires `16h01`; `<string>` capture un token
lexical unique sans ponctuation terminale comme `:`, par exemple
`ECB's <string>` -> `<string> de la BCE`. Un motif separe
`<string>:` -> `<string> :` permet d'ajouter l'espace typographique avant
deux-points sans inclure le deux-points dans la capture.
Le mode `regex` est volontairement hors MVP pour eviter d'introduire un langage
de matching trop puissant dans une page operateur.

## Scope backend

- Ajouter une migration SQLite pour `translation_glossary_entries`.
- Ajouter une migration Supabase Postgres equivalente si le glossaire doit survivre au conteneur serveur.
- Ajouter le modele Rust de persistence.
- Ajouter les methodes storage:
  - lister avec filtres `enabled`, recherche texte et pagination;
  - creer;
  - mettre a jour;
  - desactiver/supprimer.
- Ajouter les routes HTTP protegees `moderator/admin`:
  - `GET /api/glossary`
  - `POST /api/glossary`
  - `PATCH /api/glossary/{id}`
  - `DELETE /api/glossary/{id}`
- Valider les contraintes:
  - termes non vides;
  - langues supportees `en` et `fr`;
  - priorite bornee;
  - `match_mode` connu.

## Critere d'acceptation

- Les operateurs/admins peuvent gerer le glossaire via API.
- Les readers ne peuvent ni creer, ni modifier, ni supprimer.
- Les entrees desactivees restent historisables mais ne sont plus retournees par defaut pour le pipeline.
- La liste est triee de facon stable par `priority DESC`, puis terme source.
- Les tests storage/API couvrent creation, update, suppression/desactivation, droits et validation.

## Livraison 2026-06-02

- Table SQLite `translation_glossary_entries` ajoutee au bootstrap idempotent.
- Migration SQLite de reference `0015_add_translation_glossary_entries.sql`.
- Migration Supabase cible `0005_create_translation_glossary_entries.sql`.
- Routes protegees moderator/admin: `GET/POST /api/glossary`, `PATCH/DELETE /api/glossary/{id}`.
- `DELETE` desactive l'entree pour conserver l'historique.
- Page admin Glossaire ajoutee cote client avec CRUD minimal.

## Hors scope

- Application dans le pipeline de traduction.
- Page frontend.
- Import/export CSV.
- Matching regex.
