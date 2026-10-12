# NEWS-158 - Systeme securite Worker Edge

## Objectif

Durcir la securite des routes Cloudflare Worker et Durable Objects:

- `/ws/live` pour les clients;
- `/publish/live` pour le backend Rust.

## Scope

- Conserver le ticket live court pour `/ws/live`.
- Verifier expiration, signature, scope et claims minimaux cote Worker.
- Garder le ticket edge obtenu via backend authentifie.
- Durcir `/publish/live`:
  - signature HMAC du body avec timestamp;
  - aucun secret de publication expose au frontend.
- Ajouter protection anti-rejeu pour la publication:
  - timestamp obligatoire;
  - fenetre de validite courte;
  - signature du body + timestamp.
- Exiger un secret de publication fort, au moins 32 caracteres.
- Verifier que le frontend ne connait jamais le secret de publication.
- Documenter les variables et secrets Worker.

## Hors scope

- Securite HLS.
- Auth Supabase directe dans le Worker.
- Persistance longue de donnees metier dans Durable Objects.

## Contrat `/ws/live`

```text
GET /ws/live?ticket=<edge-live-ticket>
```

Le ticket contient seulement:

- `sub`;
- `role` ou `scope`;
- `iat`;
- `exp`.

## Contrat `/publish/live` cible

Headers:

```text
X-FilFinance-Timestamp: 2026-05-09T09:12:00Z
X-FilFinance-Signature: sha256=<base64url-hmac>
```

Signature:

```text
HMAC_SHA256(secret, timestamp + "." + raw_body)
```

## Criteres d'acceptation

- `/ws/live` refuse un ticket absent, expire, mal signe ou scope invalide.
- `/publish/live` refuse une publication sans secret/signature valide.
- Une signature rejouee hors fenetre est refusee.
- Une panne Worker ne bloque pas l'ingestion ni la persistence backend.
- Les erreurs Worker sont loguees sans exposer les secrets.

## Tests attendus

- Tests unitaires Worker ticket live.
- Tests unitaires Worker publish HMAC.
- Test de publication backend vers Worker.
- Test WebSocket live avec ticket valide et invalide.

## Validation locale

- `npm run check` dans `filfinance-edge`
- `cargo test`

## Dependances

- `EPIC-026`
- `EPIC-018`
- `NEWS-156`
