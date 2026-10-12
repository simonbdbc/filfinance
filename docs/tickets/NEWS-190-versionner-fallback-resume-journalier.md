# NEWS-190 - Hash de contrat du résumé journalier

## Objectif

Versionner le résumé journalier par son contenu réel afin qu'un ancien audio sans
contexte ne puisse plus masquer le résumé courant.

## Scope

- Introduire une version de contrat de résumé.
- Inclure dans le `content_hash`:
  - version de contrat;
  - jour local Europe/Paris;
  - sources allowlistées;
  - news du jour et état de qualification;
  - toutes les qualifications du jour;
  - voix Piper effective.
- Remettre une ligne en `pending` quand le hash courant existe mais que son
  contenu est incomplet ou en erreur.

## Critères D'Acceptation

- Changer les qualifications, les sources ou la voix Piper change le hash.
- Une ancienne ligne `audio_ready` sans `daily_context_text` n'est plus servie
  comme résumé valide.
- `cargo test` couvre au moins un changement de hash quand le corpus qualifié
  change.
