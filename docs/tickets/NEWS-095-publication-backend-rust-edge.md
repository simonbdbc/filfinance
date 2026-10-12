# NEWS-095 - Publication backend Rust vers edge

## Objectif

Brancher le backend Rust sur le Worker Cloudflare `filfinance-edge` pour publier une seule copie des metadata live vers Durable Objects.

Le pipeline actuel reste source de verite:

- ingestion FinancialJuice;
- persistence SQLite;
- traduction FR;
- diffusion interne backend.

Le publisher edge est un downstream optionnel. Une panne edge ne doit pas bloquer l'ingestion, la traduction, Telegram, le WebSocket backend de fallback ou la TTS actuelle.

## Implementation

Fichiers principaux:

- `filfinance-service/src/config.rs`
  - ajout de `EdgeConfig`;
  - variables `NEWS_EDGE_PUBLISH_*`;
  - validation stricte uniquement quand `NEWS_EDGE_PUBLISH_ENABLED=true`.
- `filfinance-service/src/edge.rs`
  - worker abonne a `translated_broadcast_tx`;
  - filtrage des traductions `language=target_language` et `status=ready`;
  - reconstruction du `LiveNewsItem` avec l'event source pour conserver `news_level`, URL et timestamp;
  - publication HTTP `POST /publish/live` signee avec `X-FilFinance-Timestamp` et `X-FilFinance-Signature` depuis `NEWS-158`;
  - timeout court et retries bornes.
- `filfinance-service/src/main.rs`
  - demarrage du worker edge si active;
  - shutdown gracieux borne.

## Variables

Dans `filfinance-service/.env` pour le local et `.env.server` pour le serveur:

```env
NEWS_EDGE_PUBLISH_ENABLED=false
NEWS_EDGE_PUBLISH_URL=https://filfinance-live-edge.your-cloudflare-account.workers.dev/publish/live
NEWS_EDGE_PUBLISH_SECRET=your-edge-publish-secret
NEWS_EDGE_PUBLISH_TIMEOUT_MS=1500
NEWS_EDGE_PUBLISH_RETRY_ATTEMPTS=2
NEWS_EDGE_PUBLISH_RETRY_BACKOFF_MS=250
```

Activation:

1. Deployer `filfinance-edge`.
2. Creer le secret Worker `FILFINANCE_EDGE_PUBLISH_SECRET`.
3. Mettre la meme valeur dans `NEWS_EDGE_PUBLISH_SECRET` cote backend.
4. Renseigner l'URL publique `/publish/live`.
5. Passer `NEWS_EDGE_PUBLISH_ENABLED=true`.

## Contrat publie

Le backend publie le message edge:

```json
{
  "type": "translated_news_item",
  "payload": {
    "event_id": "translated_news_item:00000000000000000042",
    "server_time": "2026-05-03T08:00:00Z",
    "item": {
      "id": 42
    }
  }
}
```

`event_id` est derive de l'identifiant SQLite de traduction avec padding fixe pour rester lisible et ordonnable.

Le payload reutilise `LiveNewsItem`; il ne contient aucun chunk audio.

## Comportement d'erreur

- HTTP `2xx`: publication acceptee.
- HTTP `408`, `429`, `5xx` ou erreur reseau: retry borne.
- HTTP `4xx` hors `408/429`: erreur permanente loguee, pas de retry.
- apres epuisement des retries: log `edge metadata publish failed`, puis le pipeline continue.

## Validation

Commandes:

```bash
cd filfinance-service
cargo fmt --check
cargo test edge
cargo check
```

Critères d'acceptation:

- une panne edge ne bloque pas l'ingestion FinancialJuice;
- un evenement live traduit pret est publie une seule fois vers l'edge;
- les logs distinguent les echecs edge des erreurs pipeline;
- le publisher reste desactive par defaut pour ne pas modifier le runtime actuel avant `NEWS-096`.

## Suite

- `NEWS-096`: brancher le client sur le WebSocket edge metadata.
- `NEWS-097`: remplacer l'acces public temporaire du WebSocket edge par un ticket de session court.
