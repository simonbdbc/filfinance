# NEWS-093 - Contrat metadata live edge

## Statut

Implementation livree le 3 mai 2026.

## Objectif

Figer le contrat du futur control plane live porte par Cloudflare Durable Objects avant de creer le Worker.

Le canal edge doit transporter uniquement des metadata:

- news cards;
- items traduits EN+FR;
- catch-up court des derniers items traduits pour l'historique journalier;
- etat `now_playing`;
- etat du flux audio HLS;
- evenements audio live sans binaire;
- notifications systeme;
- heartbeat et lag.

L'audio binaire reste explicitement hors contrat.

## Scope

- Ajouter les types Rust du contrat edge pour le futur publisher backend.
- Ajouter les types TypeScript partages pour le futur Worker et le client.
- Documenter les messages autorises.
- Verrouiller par tests Rust que le contrat expose des metadata et pas de chunks audio.

## Hors scope

- Worker Cloudflare.
- Durable Object de fan-out.
- Endpoint backend de publication.
- Ticket d'auth edge.
- Player HLS client.

## Messages retenus

Tous les messages possedent un champ `type` stable en `snake_case`.

Messages de connexion:

- `hello`: version du protocole, `server_time`, snapshot minimal;
- `heartbeat`: `server_time`;
- `lagged`: `server_time`, nombre d'evenements sautes.

Messages metier:

- `news_card`: `event_id`, `server_time`, carte compacte;
- `translated_news_item`: `event_id`, `server_time`, `LiveNewsItem` existant;
- `now_playing`: `event_id`, `server_time`, metadata de lecture courante ou `null`;
- `audio_state`: `event_id`, `server_time`, etat public HLS;
- `system_notice`: `event_id`, `server_time`, severite, message, code/detail optionnels.

Extension `EPIC-021`:

- `audio_ready`: playlist finale d'un clip HLS disponible et lisible depuis le debut;
- `audio_failed`: generation audio impossible ou timeout;
- `audio_finalized`: finalisation optionnelle du clip, utile pour observabilite.

## Decisions

- `event_id` est opaque pour le client. Le backend publisher devra le produire de facon monotone ou au moins strictement ordonnable.
- `translated_news_item` reutilise `LiveNewsItem`, car ce contrat porte deja les champs stables du flux actuel.
- `audio_state` annonce seulement des URLs de playlists et un statut. Il ne transporte jamais d'octets audio.
- Le snapshot initial contient le dernier `event_id`, `now_playing`, `audio_state`, les derniers `translated_news_item` et les derniers evenements audio.
- Les derniers `translated_news_item` du snapshot servent uniquement a restaurer l'historique visible apres reconnexion. Ils ne declenchent jamais de lecture audio automatique.
- Les evenements `audio_ready`, `audio_failed` et `audio_finalized` restent separes du refresh historique.

## Implementation

- Backend Rust:
  - `EdgeLiveMessage`;
  - `EdgeNewsCard`;
  - `EdgeNowPlaying`;
  - `EdgeAudioState`;
  - `EdgeLiveSnapshot`;
  - payloads dedies par type de message.
- Client TypeScript:
  - `EdgeLiveWsMessage`;
  - types metadata associes dans `src/shared/api/contracts.ts`.

## Criteres d'acceptation

- Le contrat ne contient aucun chunk audio.
- Chaque message metier possede `event_id` et `server_time`.
- Les messages techniques possedent `server_time`.
- Les payloads reutilisent `LiveNewsItem` quand l'item traduit complet est diffuse.
- Les types sont disponibles cote Rust et cote client.
- Le snapshot de reconnexion peut rejouer les derniers `translated_news_item` sans passer par un evenement audio.

## Validation locale

- `cargo test edge_live`: OK.
- `npm run typecheck`: OK.
