# NEWS-143 - Restauration du catch-up historique journalier edge

## Statut

Implementation livree le 4 mai 2026.

## Probleme

Avant `EPIC-018`, le fonctionnement valide etait simple:

- l'historique journalier est charge depuis `/api/translations`;
- le flux `/ws/news` diffuse `translated_news_item`;
- le backend peut rejouer des traductions recentes a la connexion;
- le client ajoute les `translated_news_item` au bon historique visible, sans autoplay.

Avec la bascule edge, le Durable Object conserve un snapshot `now_playing`, `audio_state` et `audio_events`, mais ne conserve pas encore un historique court de `translated_news_item`. Une reconnexion edge peut donc rater les dernieres news traduites, alors que l'ancien fallback backend savait les rejouer.

Le contournement consistant a rafraichir l'historique depuis `audio_ready` est invalide: l'audio est un data plane de lecture, pas la source de verite de l'historique.

## Principe cible

L'historique journalier reste un contrat texte/persistence:

```text
SQLite news_translations
  -> GET /api/translations
  -> base historique journalier

translated_news_item
  -> WebSocket backend ou edge
  -> mise a jour temps reel du meme historique
```

L'audio reste un contrat separe:

```text
audio_ready
  -> queue audio globale
  -> lecture d'un clip HLS finalise par news
```

`audio_ready`, `audio_failed` et `audio_finalized` ne doivent jamais declencher de refresh historique.

## Scope

- Ajouter au snapshot edge un historique court borne de messages `translated_news_item`: OK.
- Rejouer ce snapshot cote client avec le meme handler que les messages live: OK.
- Dedupliquer par id de traduction et conserver le tri metier existant: OK.
- Garder le fallback backend `/ws/news` comme reference de comportement pendant la transition: OK.
- Documenter explicitement la separation entre historique visible et queue audio: OK.

## Hors scope

- Modifier la generation TTS.
- Modifier les clips HLS `EPIC-021`.
- Rejouer automatiquement de l'audio depuis l'historique.
- Lier le rafraichissement historique a un evenement audio.

## Criteres d'acceptation

- Une reconnexion au WebSocket edge rattrape les derniers `translated_news_item`: OK cote snapshot edge.
- L'historique journalier feed reel et `test_run` se met a jour via `translated_news_item`: OK cote client, meme handler que le live.
- L'historique initial reste charge par `/api/translations`: OK, chemin conserve.
- `audio_ready` alimente uniquement la queue audio: OK, aucun changement du dispatch audio.
- Aucun appel `/api/translations` n'est declenche par `audio_ready`: OK.
- Une news rejetee de la queue audio reste visible dans l'historique: OK par separation des handlers.
- Le comportement est couvert par tests edge/client avant promotion: partiel, test edge et typecheck client livres; smoke produit a executer au deploiement.

## Validation

- `filfinance-edge`: `npm run check` OK, avec test du snapshot borne de `translated_news_item`.
- `filfinance-client`: `npm run typecheck` OK.
- `filfinance-service`: `cargo fmt --check` OK.
- `filfinance-service`: `cargo test edge_live` OK.
- Smoke produit restant: nouvelle news FinancialJuice visible dans l'historique journalier apres reconnexion edge, sans action audio.

## References

- [EPIC-018 - Metadata live edge Durable Objects](../epics/EPIC-018-metadata-live-edge-durable-objects.md)
- [NEWS-093 - Contrat metadata live edge](./NEWS-093-contrat-metadata-live-edge.md)
- [NEWS-096 - Client WebSocket edge metadata](./NEWS-096-client-websocket-edge-metadata.md)
