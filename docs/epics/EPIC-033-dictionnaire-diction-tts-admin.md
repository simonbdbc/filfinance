# EPIC-033 - Dictionnaire & diction TTS admin

## Objectif

Transformer le glossaire existant en outil admin unique de pilotage du dictionnaire et de la diction TTS.

Le contrat produit reste explicite:

```text
EN source -> FR affiche -> FR vocal -> normalisation TTS systeme -> Piper
```

L'objectif n'est pas de fusionner brutalement les moteurs internes. Les regles metier restent pilotables par l'admin, tandis que les normalisations systeme TTS restent codees cote backend et exposees en lecture seule.

## Decisions

- La page admin devient `Dictionnaire & diction TTS`.
- Les anciennes entrees glossaire gardent la portee `full_pipeline`.
- La nouvelle portee `spoken_only` ne modifie jamais la traduction ni le texte affiche.
- Les regles systeme TTS sont visibles mais non editables en v1.
- L'aperçu admin est textuel uniquement; aucun audio Piper n'est genere en v1.
- Le registre reste un tableau dense et compact, pas une grille de cards.

## Tickets

- [NEWS-196 - Backend dictionnaire TTS: portee, preview et regles systeme](../tickets/NEWS-196-backend-dictionnaire-tts-portee-preview-regles-systeme.md)
- [NEWS-197 - Admin Dictionnaire & diction TTS](../tickets/NEWS-197-admin-dictionnaire-diction-tts.md)
- [NEWS-198 - Validation et documentation dictionnaire TTS](../tickets/NEWS-198-validation-doc-dictionnaire-diction-tts.md)

## Definition Of Done

- Les entrees glossaire exposent `scope = full_pipeline | spoken_only`.
- Les regles `spoken_only` ne touchent que le texte envoye a Piper.
- L'admin expose un aperçu textuel des etapes `FR affiche`, `FR vocal` et `Texte Piper`.
- L'admin expose les regles systeme TTS en lecture seule.
- Le registre glossaire est plus compact, avec notes/date dans la cellule principale et boutons d'action courts.
- Les migrations SQLite et Supabase documentent la nouvelle portee.
- Les tests backend et frontend couvrent les contrats principaux.
