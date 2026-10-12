# NEWS-189 - Audit résumé journalier non vide

## Objectif

Documenter le bug produit: le bouton pouvait lire un ancien texte sans contexte
de marché, ou rester sur un état "aucune news importante" alors que le résumé du
jour devait au minimum contenir un contexte utile.

## Constat

- Le résumé journalier dépendait trop du fait qu'une nouvelle news importante
  apparaisse pendant le tick courant.
- Une journée sans news importante produisait un texte court, sans contexte du
  jour.
- `GET /api/daily-summary/today` ne jouait pas encore le rôle d'assurance au
  lancement de l'app.
- Le pipeline live traduction -> Piper -> HLS ne doit pas être modifié par ce
  chantier.

## Scope

- Vérifier les chemins backend qui peuvent servir un résumé sans
  `daily_context_text`.
- Vérifier que le résumé courant est sélectionné par hash de contrat courant,
  pas par "dernier updated_at".
- Vérifier que les anciennes lignes sans contexte ne sont plus considérées comme
  réponse produit valide.

## Critères D'Acceptation

- Tout résumé `audio_ready` produit doit avoir un titre, un contexte, un résumé
  des news importantes et une narration.
- Une journée sans news importante produit une phrase claire dans
  `important_news_summary_text`, mais conserve un vrai `daily_context_text`.
- L'audit confirme que le pipeline live ne passe pas par n8n.
