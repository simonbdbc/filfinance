# EPIC-031 - Résumé journalier toujours disponible

## Objectif

Le pipeline live reste strictement inchangé:

```text
ingestion -> traduction backend -> Piper -> HLS
```

Le résumé journalier est un processus séparé, idempotent, piloté par le backend
et n8n/OpenRouter. Il ne doit jamais produire un résumé vide. Le minimum produit
est toujours:

- titre du jour généré côté backend, avec date française écrite en lettres;
- contexte économique, financier et géopolitique du jour;
- résumé des nouvelles importantes, ou phrase explicite indiquant qu'aucune
  nouvelle importante n'a encore été qualifiée.

## Architecture Cible

```text
backend scheduler / GET /api/daily-summary/today
  -> n8n récupère les news du jour et les qualifications existantes
  -> n8n qualifie uniquement les news non qualifiées
  -> backend stocke les qualifications et renvoie les nouvelles importantes
  -> n8n envoie Telegram pour ces nouvelles importantes
  -> n8n récupère les sources web allowlistées
  -> OpenRouter produit contexte + résumé des news qualifiées
  -> callback backend -> Piper -> HLS résumé
```

n8n/OpenRouter reste hors du hot path live. Une panne n8n, OpenRouter ou source
web ne bloque pas l'ingestion, la traduction, Piper live ou HLS live.

## Déclencheurs

- Au démarrage backend: vérifier le jour courant Europe/Paris et déclencher un
  run si le contexte du jour manque.
- Au lancement de l'app: `GET /api/daily-summary/today` joue le rôle
  d'assurance idempotente et planifie un run si le contexte ou les
  qualifications manquent.
- Au début d'un nouveau jour Europe/Paris: forcer un nouveau contexte du jour.
- À chaque nouvelle ingestion: vérifier toutes les news du jour, qualifier les
  manquantes, puis recalculer le résumé depuis toutes les news qualifiées.
- Le tick par défaut reste `FILFINANCE_DAILY_SUMMARY_TICK_SECS=120`.

## Sources Contexte

`FILFINANCE_DAILY_SUMMARY_SOURCE_URLS` est une allowlist stricte. Le workflow ne
fait pas de recherche libre. Les sources par défaut incluent:

- `https://www.reuters.com/markets/`;
- `https://www.ecb.europa.eu/press/html/index.en.html`;
- `https://www.investing.com/economic-calendar/`;
- `https://www.forexfactory.com/calendar`.

Le contexte doit couvrir les facteurs économiques, financiers et géopolitiques
du jour, ainsi que les annonces attendues quand elles apparaissent dans ces
sources.

## Règles Produit

- `daily_context_text` est obligatoire pour tout résumé `audio_ready`.
- `important_news_summary_text` est obligatoire; il peut indiquer qu'aucune
  nouvelle importante n'a encore été qualifiée.
- `summary_text` reste compatible et concatène contexte + résumé des news.
- Le hash de contenu inclut la version de contrat, le jour, les sources, les
  qualifications du jour et la voix Piper effective.
- Un ancien audio sans contexte ne doit jamais être préféré à un résumé IA
  valide pour le hash courant.

## Definition Of Done

- Le bouton du frontend lit toujours un résumé journalier non vide quand un HLS
  est prêt.
- Une journée sans news importante produit quand même un audio avec contexte du
  jour.
- Une news non qualifiée déclenche uniquement la qualification manquante.
- Un changement de qualifications ou de sources invalide le résumé courant.
- Telegram reste idempotent et ne part que pour les nouvelles news importantes.
- Les tests confirment que le live reste sans appel n8n.

## Tickets

- [NEWS-189](../tickets/NEWS-189-audit-cache-fallback-resume-journalier.md)
- [NEWS-190](../tickets/NEWS-190-versionner-fallback-resume-journalier.md)
- [NEWS-191](../tickets/NEWS-191-rebuild-resume-ia-news-importantes-existantes.md)
- [NEWS-192](../tickets/NEWS-192-selection-resume-courant-priorite-ai.md)
- [NEWS-193](../tickets/NEWS-193-contexte-marche-sources-allowlist-resume-ia.md)
- [NEWS-194](../tickets/NEWS-194-observabilite-tests-resume-ia-journalier.md)
