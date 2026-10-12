# EPIC-018 - Metadata live edge Durable Objects

## Objectif

Sortir le fan-out WebSocket metadata du backend Rust et le porter sur Cloudflare Durable Objects.

Le backend Rust ne doit plus maintenir une socket metadata par client final. Il publie une seule fois chaque evenement live vers l'edge, puis Durable Objects diffuse aux clients connectes.

## Valeur livree

Cette refonte transforme le WebSocket en control plane scalable:

- metadata live
- news cards
- historique journalier par catch-up des items traduits recents
- `now_playing`
- etat du flux audio
- evenements audio `audio_ready`, `audio_failed` et `audio_finalized`
- notifications systeme
- heartbeat leger

L'audio binaire est exclu de ce canal. Le data plane audio est porte par HLS/CDN: d'abord via le palier `EPIC-019`, puis par les clips HLS finalises par news de `EPIC-021`.

## Statut

Cadrage ajoute le 2 mai 2026.

`NEWS-093` est implemente le 3 mai 2026: le contrat metadata live edge existe cote Rust et cote TypeScript, sans audio binaire.

`NEWS-094` est implemente le 3 mai 2026: le package `filfinance-edge` contient le Worker Cloudflare, le Durable Object live, les routes `GET /ws/live` et `POST /publish/live`, ainsi que les tests unitaires du protocole de publication.

`NEWS-095` est implemente le 3 mai 2026: le backend Rust peut publier les traductions live pretes vers `POST /publish/live` avec secret, timeout court et retries bornes. Le publisher reste desactive par defaut jusqu'a la bascule client `NEWS-096`.

`NEWS-096` est implemente le 3 mai 2026: le client peut consommer `EXPO_PUBLIC_FILFINANCE_EDGE_WS_URL` pour les metadata live, tout en gardant le backend `/ws/news` comme fallback.

`NEWS-097` est implemente le 3 mai 2026: le backend genere un ticket edge court via `/api/edge/live-ticket`, le client l'ajoute a `/ws/live` et le Worker verifie signature, expiration et scope localement.

`NEWS-098` est prepare le 3 mai 2026: le script `filfinance-edge/scripts/load-test-live.js` mesure le fan-out metadata 1000 clients et produit un rapport JSON p50/p95/p99. Le run 1000 clients reste a executer sur l'environnement Cloudflare cible.

Extension decidee le 4 mai 2026: `EPIC-021` utilise le meme control plane pour annoncer les clips audio HLS finalises par news avec `audio_ready`, `audio_failed` et eventuellement `audio_finalized`, toujours sans audio binaire dans Durable Objects.

Correction de cadrage decidee le 4 mai 2026: l'historique journalier reprend le fonctionnement valide sur la branche backup `backup/main-prod-before-epic18-2026-05-03`. Il est pilote par `/api/translations` et par les messages `translated_news_item`, jamais par les evenements audio.

`NEWS-143` est implemente le 4 mai 2026: le Durable Object garde un historique court borne de `translated_news_item` dans le snapshot, et le client le rejoue via le meme handler que le live sans toucher a la queue audio.

## Architecture cible

```text
Backend Rust
  -> POST /publish/live
  -> Cloudflare Worker
  -> Durable Object "filfinance-live"
  -> WebSocket clients Pages
```

Le client Expo Web heberge sur Cloudflare Pages ouvre un WebSocket edge:

```text
wss://edge.filfinance.com/ws/live
```

## Decisions

- Durable Objects porte le fan-out metadata.
- Le backend Rust reste source de verite metier et publie une seule copie de chaque evenement.
- Le WebSocket edge ne transporte jamais les chunks audio.
- Le flux `/ws/audio` actuel doit etre retire du chemin nominal par le bloc audio HLS.
- Le live audio nominal de `EPIC-021` est declenche par evenement `audio_ready`, pas par polling ni par flux HLS permanent.
- Le rafraichissement de l'historique journalier ne depend jamais de `audio_ready`, `audio_failed` ou `audio_finalized`.
- Le snapshot de reconnexion edge doit contenir un historique court borne des derniers `translated_news_item`, separe de l'historique court des `audio_events`.
- Le flux `/ws/news` backend peut rester en fallback temporaire jusqu'a validation du canal edge.
- Le canal debug moderator doit rester separe du canal live public.

## Scope

Dans cet EPIC:

- definir le contrat de messages metadata live
- creer le Worker edge et le Durable Object live
- brancher un endpoint publish serveur protege
- publier depuis le backend Rust vers l'edge
- brancher le client sur le WebSocket edge
- preparer l'auth edge par ticket court
- tester le fan-out 1000 users metadata
- etendre le contrat avec les evenements audio de `EPIC-021` sans transporter d'octets audio
- restaurer le catch-up des items traduits recents cote edge pour garder l'historique journalier coherent

Hors scope:

- data plane audio HLS
- replay audio du jour
- migration Supabase profils
- deploiement GitLab/Cloudflare complet

## Tickets de dev

### [`NEWS-093` Contrat metadata live edge](../tickets/NEWS-093-contrat-metadata-live-edge.md)

Objectif: figer les messages du control plane edge.

Messages attendus:

- `hello`
- `heartbeat`
- `news_card`
- `translated_news_item`
- `now_playing`
- `audio_state`
- `audio_ready`
- `audio_failed`
- `audio_finalized`
- `system_notice`
- `lagged`

Criteres d'acceptation:

- le contrat ne contient aucun chunk audio: OK
- chaque message possede un `type`, un `event_id` si applicable et un timestamp serveur: OK
- les payloads reutilisent les types metier existants quand ils sont stables: OK

### [`NEWS-094` Worker et Durable Object live](../tickets/NEWS-094-worker-durable-object-live.md)

Objectif: creer le Worker Cloudflare et le Durable Object de fan-out.

Scope:

- route `GET /ws/live`
- route serveur `POST /publish/live`
- stockage minimal du dernier `now_playing`
- hibernation WebSocket si compatible avec le modele retenu
- logs d'erreur exploitables

Criteres d'acceptation:

- plusieurs clients recoivent le meme message publie une seule fois: OK cote Worker/DO
- une reconnexion recoit un etat initial minimal: OK
- aucun etat durable volumineux n'est stocke dans le Durable Object: OK, seul le snapshot minimal est persiste

### [`NEWS-095` Publication backend Rust vers edge](../tickets/NEWS-095-publication-backend-rust-edge.md)

Objectif: ajouter un publisher Rust vers Cloudflare Worker.

Scope:

- client HTTP interne
- secret serveur de publication
- retry borne
- timeout court
- metriques/logs en cas d'echec edge

Criteres d'acceptation:

- une panne edge ne bloque pas l'ingestion FinancialJuice: OK, worker downstream isole
- un evenement live est publie une seule fois vers l'edge: OK, publication depuis `translated_broadcast_tx`
- les logs permettent de distinguer panne edge et panne pipeline: OK, erreurs edge loguees comme downstream

### [`NEWS-096` Client WebSocket edge metadata](../tickets/NEWS-096-client-websocket-edge-metadata.md)

Objectif: faire consommer au client le nouveau WebSocket edge.

Scope:

- nouvelle URL publique de WebSocket live
- remplacement progressif du flux `/ws/news` nominal
- conservation d'un fallback de dev si necessaire
- synchronisation des news cards et du `now_playing`

Criteres d'acceptation:

- les cards live continuent d'apparaitre sans connexion directe au backend Rust: OK, via `EXPO_PUBLIC_FILFINANCE_EDGE_WS_URL`
- le client n'ouvre plus de canal audio binaire pour le live: partiel, le canal audio actuel reste actif jusqu'a `EPIC-019`
- les erreurs edge sont visibles dans le resume systeme: OK, `system_notice` warning/error et fallback sont remontes

### [`NEWS-143` Restauration du catch-up historique journalier edge](../tickets/NEWS-143-restauration-catchup-historique-journalier-edge.md)

Objectif: reprendre le comportement valide avant `EPIC-018`: le chargement HTTP `/api/translations` donne la base de l'historique journalier et les messages `translated_news_item` font le rattrapage temps reel.

Scope:

- conserver un historique court borne de `translated_news_item` dans le Durable Object
- inclure ces items dans le snapshot `hello`
- rejouer ces items cote client via le meme handler que le live
- dedupliquer par id de traduction
- documenter que les evenements audio ne rafraichissent jamais l'historique visible

Criteres d'acceptation:

- une reconnexion edge rattrape les dernieres news traduites sans attendre un nouvel evenement live: OK
- l'historique journalier feed reel et `test_run` reste separe: OK, handler client existant conserve
- `audio_ready` ne declenche aucun chargement de `/api/translations`: OK
- le fallback backend `/ws/news` garde le meme comportement que sur la branche backup: OK

### [`NEWS-097` Auth edge session ticket](../tickets/NEWS-097-auth-edge-session-ticket.md)

Objectif: eviter que le Durable Object interroge Supabase/Postgres a chaque connexion.

Scope:

- endpoint backend de generation d'un ticket edge court
- signature serveur
- expiration courte
- role/scope minimal dans le ticket
- verification locale cote Worker

Criteres d'acceptation:

- le ticket expire rapidement: OK, TTL configure et borne a 300s
- un ticket listener suffit au live metadata: OK, endpoint dans les routes listener
- un ticket invalide ferme la socket edge: OK, le Worker refuse l'upgrade
- aucun secret Supabase prive n'est expose au client: OK, seul un ticket HMAC court est transmis

### [`NEWS-098` Load test metadata 1000 users](../tickets/NEWS-098-load-test-metadata-1000-users.md)

Objectif: valider le fan-out metadata avant de retirer le chemin backend direct.

Scope:

- simulateur de 1000 clients WebSocket
- injection de messages live representatifs
- mesure latence p50/p95/p99
- mesure erreurs, reconnexions et lag

Criteres d'acceptation:

- 1000 clients recoivent les messages sans perte observable: pret, a mesurer sur Cloudflare
- le backend Rust ne maintient pas les 1000 sockets metadata: OK par architecture, clients connectes au Worker
- les resultats sont documentes avant promotion production: pret, rapport JSON a archiver apres run

## Definition of Done

- le client nominal lit les metadata live depuis Cloudflare Durable Objects
- le backend Rust publie une seule copie de chaque evenement live metadata
- aucun audio binaire ne passe par Durable Objects
- `audio_ready` peut declencher un clip HLS finalise par news sans polling client
- le snapshot edge permet de rattraper les derniers `translated_news_item` sans coupler historique et audio
- le comportement 1000 clients est mesure
- le fallback backend direct est documente ou retire

## References

- [EPIC-019 - Audio HLS live et replay du jour](./EPIC-019-audio-hls-live-replay-du-jour.md)
- [EPIC-021 - Audio live evenementiel par clips HLS progressifs](./EPIC-021-audio-live-evenementiel-clips-hls-progressifs.md)
- [EPIC-020 - GitLab et pipeline Cloudflare](./EPIC-020-gitlab-pipeline-cloudflare.md)
- [Cloudflare Durable Objects WebSockets](https://developers.cloudflare.com/durable-objects/best-practices/websockets/)
