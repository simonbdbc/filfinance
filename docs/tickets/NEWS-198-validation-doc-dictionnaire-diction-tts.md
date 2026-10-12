# NEWS-198 - Validation et documentation dictionnaire TTS

## Objectif

Verrouiller les non-regressions du dictionnaire/TTS apres ajout de la portee, du preview et de la refonte compacte admin.

## Scope

- Tester les migrations SQLite et Supabase de reference.
- Tester les portees `full_pipeline` et `spoken_only`.
- Tester les endpoints:
  - `GET /api/glossary/system-rules`;
  - `POST /api/glossary/preview`.
- Tester le registre compact cote frontend.
- Documenter l'absence d'audio preview en v1.

## Scenarios backend

- `full_pipeline` conserve le comportement existant `ECB -> BCE -> Banque Centrale Europeenne`.
- `spoken_only` ne protege pas le texte source et ne remplace pas le texte affiche.
- `spoken_only` remplace le texte vocal avant normalisation TTS.
- Le preview `display_fr` n'appelle pas le provider de traduction.
- Le preview `source_en` renvoie un warning si le provider de traduction echoue.
- Les roles non autorises ne peuvent pas modifier les regles.

## Scenarios frontend

- Le formulaire expose la portee.
- Le registre compact affiche source, FR affiche, vocal, notes, date, portee, mode, priorite, casse et actions.
- Les boutons compacts restent lisibles et ne debordent pas.
- Les regles systeme sont visibles et non editables.
- Les erreurs API de preview et de sauvegarde sont affichees.

## Commandes de validation

```bash
cd filfinance-service
cargo test glossary
cargo test

cd ../filfinance-client
npm run typecheck
npm run build:web
```

## Hors scope

- Tests audio Piper reels.
- Tests visuels automatises cross-browser.
- Charge ou benchmark du preview.
