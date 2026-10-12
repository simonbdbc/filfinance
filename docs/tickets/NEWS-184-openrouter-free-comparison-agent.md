# NEWS-184 - Agent OpenRouter pour le résumé journalier

## Objectif

Utiliser OpenRouter Free Models Router dans n8n pour synthétiser les news
traduites de la journée en un résumé français narrable.

## Scope

- Importer `AI Agent OpenRouter`.
- Associer le credential OpenRouter dans l'UI n8n.
- Utiliser `OPENROUTER_MODEL=nvidia/nemotron-3-super-120b-a12b:free` par défaut.
- Forcer une sortie structurée `dailyContext`, `importantNewsSummary`,
  `narrationText`; le titre daté reste généré côté backend.
- Renvoyer le résultat au callback backend texte.

## Critères d'acceptation

- Le workflow produit un résumé enrichi mais limité aux news et sources
  allowlistées.
- Un échec OpenRouter marque le résumé en erreur sans toucher au live.
- Le backend génère l'audio avec Piper après callback valide.
