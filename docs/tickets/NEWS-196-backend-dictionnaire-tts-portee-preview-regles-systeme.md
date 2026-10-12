# NEWS-196 - Backend dictionnaire TTS: portee, preview et regles systeme

## Objectif

Etendre le backend glossaire pour piloter deux familles de regles:

- `full_pipeline`: regles metier appliquees a la traduction, a l'affichage et au vocal;
- `spoken_only`: regles appliquees uniquement au texte envoye a Piper.

## Scope

- Ajouter `scope` aux entrees `translation_glossary_entries`.
- Migrer SQLite et Supabase avec defaut `full_pipeline`.
- Exclure `spoken_only` de `apply_source_glossary`, `apply_display_glossary` et des cibles de retraitement.
- Conserver `spoken_only` dans `apply_spoken_glossary`.
- Ajouter `GET /api/glossary/system-rules`.
- Ajouter `POST /api/glossary/preview`.
- Exposer un aperçu textuel sans persistence ni broadcast.

## Contrats

```text
scope = full_pipeline | spoken_only
```

Endpoints admin:

- `GET /api/glossary/system-rules`
- `POST /api/glossary/preview`

Requete preview:

```json
{
  "input_text": "14:30. BCE attend EUR/USD apres NFP.",
  "input_stage": "display_fr",
  "draft_entry": {
    "source_term": "ECB",
    "translated_term": "BCE",
    "spoken_term": "Banque Centrale Europeenne",
    "source_language": "en",
    "target_language": "fr",
    "match_mode": "word_boundary",
    "case_sensitive": false,
    "priority": 100,
    "enabled": true,
    "scope": "full_pipeline",
    "notes": null
  }
}
```

Reponse preview:

```json
{
  "display_text": "...",
  "spoken_text": "...",
  "normalized_tts_text": "...",
  "glossary_hits": 1,
  "warnings": []
}
```

## Critere d'acceptation

- Les anciennes entrees ont implicitement `full_pipeline`.
- Une entree `spoken_only` ne modifie pas le texte traduit ni affiche.
- Une entree `spoken_only` avec `spoken_term` s'applique avant normalisation TTS.
- Les regles systeme sont retournees en lecture seule.
- Le preview fonctionne avec `source_en` et `display_fr`.
- Le preview ne persiste rien et ne diffuse rien.

## Hors scope

- Aperçu audio Piper reel.
- Mini-langage de regles systeme editables.
- Historique d'audit des previews.
