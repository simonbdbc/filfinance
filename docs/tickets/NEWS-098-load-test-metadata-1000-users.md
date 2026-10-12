# NEWS-098 - Load test metadata 1000 users

## Objectif

Fournir le runbook et l'outil de mesure pour valider le fan-out metadata Durable Objects avec 1000 clients WebSocket.

Le test cible uniquement le control plane:

- `news_card`;
- WebSocket `/ws/live`;
- endpoint de publication `/publish/live`;
- aucune donnee audio binaire.

## Implementation

Fichiers:

- `filfinance-edge/scripts/load-test-live.js`;
- `filfinance-edge/package.json`;
- `filfinance-edge/README.md`.

Le script:

1. ouvre `N` clients WebSocket;
2. publie `M` messages `news_card`;
3. mesure les livraisons recues par tous les clients;
4. calcule latences `p50`, `p95`, `p99`, `max`;
5. retourne un code non nul si une connexion echoue ou si une livraison manque.

## Variables

```bash
FILFINANCE_EDGE_WS_URL="wss://live.filfinance.fr/ws/live"
FILFINANCE_EDGE_LOAD_TEST_TICKET="<ticket listener court>"
FILFINANCE_EDGE_PUBLISH_URL="https://live.filfinance.fr/publish/live"
FILFINANCE_EDGE_PUBLISH_SECRET="<publish-secret>"
FILFINANCE_EDGE_LOAD_TEST_CLIENTS=1000
FILFINANCE_EDGE_LOAD_TEST_MESSAGES=10
FILFINANCE_EDGE_LOAD_TEST_CONNECT_BATCH_SIZE=50
FILFINANCE_EDGE_LOAD_TEST_CONNECT_DELAY_MS=50
FILFINANCE_EDGE_LOAD_TEST_PUBLISH_INTERVAL_MS=250
FILFINANCE_EDGE_LOAD_TEST_TIMEOUT_MS=30000
```

Le ticket peut etre obtenu via:

```bash
curl -H "Authorization: Bearer <supabase-access-token>" \
  https://api.filfinance.fr/api/edge/live-ticket
```

## Commande

```bash
cd filfinance-edge
FILFINANCE_EDGE_WS_URL="wss://live.filfinance.fr/ws/live" \
FILFINANCE_EDGE_LOAD_TEST_TICKET="<edge-live-ticket>" \
FILFINANCE_EDGE_PUBLISH_URL="https://live.filfinance.fr/publish/live" \
FILFINANCE_EDGE_PUBLISH_SECRET="<publish-secret>" \
FILFINANCE_EDGE_LOAD_TEST_CLIENTS=1000 \
npm run load-test:live
```

## Rapport attendu

Le script imprime un JSON:

```json
{
  "clients_requested": 1000,
  "clients_connected": 1000,
  "connection_failures": 0,
  "messages_published": 10,
  "expected_deliveries": 10000,
  "received_deliveries": 10000,
  "missed_deliveries": 0,
  "latency_ms": {
    "p50": 12.3,
    "p95": 48.1,
    "p99": 83.7,
    "max": 120.4
  }
}
```

## Criteres d'acceptation

- 1000 clients connectes;
- 0 connexion echouee;
- 0 livraison manquante;
- latence p95 et p99 archivees dans le ticket de validation;
- le backend Rust ne maintient aucune socket metadata client pendant ce test.

## Statut validation

Le script est versionne et verifie par `npm run check`.

Le run 1000 clients doit etre execute sur l'environnement Cloudflare cible avec un ticket edge valide et un secret de publication hors Git.
