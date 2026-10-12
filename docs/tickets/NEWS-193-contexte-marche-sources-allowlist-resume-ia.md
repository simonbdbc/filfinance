# NEWS-193 - Contexte marché allowlisté obligatoire

## Objectif

Garantir que le résumé journalier contient toujours un contexte du jour utile,
même quand aucune news importante n'a encore été qualifiée.

## Scope

- Garder le schéma n8n structuré:
  - `dailyContext`;
  - `importantNewsSummary`;
  - `narrationText`.
- Enrichir `FILFINANCE_DAILY_SUMMARY_SOURCE_URLS` avec:
  - `https://www.investing.com/economic-calendar/`;
  - `https://www.forexfactory.com/calendar`.
- Renforcer le prompt:
  - utiliser uniquement l'allowlist;
  - couvrir économie, finance, géopolitique et annonces attendues;
  - produire une phrase explicite si aucune news importante n'existe;
  - conserver l'ordre chronologique ASC des news importantes.
- Rejeter côté backend un callback résumé sans `dailyContextText`.

## Critères D'Acceptation

- `daily_context_text` est obligatoire pour tout résumé `audio_ready`.
- `important_news_summary_text` est obligatoire, même s'il indique l'absence de
  news importante.
- Le workflow n8n ne fait pas de recherche libre hors allowlist.
- Les sources Investing et Forex Factory sont documentées dans les env examples.
