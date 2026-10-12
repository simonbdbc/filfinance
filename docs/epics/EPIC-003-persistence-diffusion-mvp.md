# EPIC-003 - Persistence et diffusion MVP

## Objectif

Transformer le flux collecte en sorties consommables par les clients: historique, WebSocket temps reel et Telegram.

## Valeur livree

Une premiere version exploitable du service du point de vue produit: les evenements entrent, sont stockes, consultables et diffuses.

## Statut

Largement livre au 22 avril 2026.

## Contexte courant et dependances

Le projet ne se limite plus a un hub WebSocket minimal. Il expose deja un historique HTTP, un flux WebSocket user-facing EN+FR, un flux WebSocket de debug et un relay Telegram qui consomme le meme objet derive. Cet EPIC depend du socle `EPIC-001` et de la livraison d'une premiere source reelle via `EPIC-002`.

## Etat reel au 22 avril 2026

Ce qui est deja livre:

- repository SQLite stable pour les news et les traductions
- pagination simple sur l'historique HTTP
- tri des news et des traductions sur `published_at`
- `GET /api/news`, `GET /api/news/{id}` et `GET /api/sources`
- `/ws/news` avec `hello`, `heartbeat`, `translated_news_item` et `lagged`
- replay des dernieres traductions prêtes a la connexion
- worker Telegram avec retry borne et throttling
- alignement Telegram plus WebSocket sur le meme flux EN+FR

Reste a durcir hors de cet EPIC:

- la partie audio binaire de la diffusion
- une observabilite plus complete
- les scenarios delivery et exploitation

## Definition of Done

- les evenements normalises sont persistés et relus via un repository stable
- le WebSocket diffuse un contrat clair avec comportement defini en cas de lag
- le worker Telegram ne casse pas le hot path
- l'API HTTP permet une lecture MVP de l'historique

## Risques et points durs

- coupler trop fortement la persistence au schema de sortie
- blocage du pipeline par les consommateurs lents
- format Telegram fragile
- requetes HTTP insuffisantes pour deboguer le flux

## Liste des tickets

- `NEWS-009` Repository news, indexation et requetes d'historique
- `NEWS-010` Contrat WebSocket et diffusion avec backpressure
- `NEWS-011` Worker Telegram, retry, throttling et format des messages
- `NEWS-012` Endpoints de lecture et filtres MVP

## Details des tickets

### `NEWS-009` Repository news, indexation et requetes d'historique

**Objectif**  
Stabiliser la couche de lecture/ecriture vers le stockage.

**Scope**  
Encadrer les methodes repository pour insert, lecture recentes, filtres par source, pagination simple et lecture par identifiant si utile au debug.

**Hors scope**  
Recherche full-text ou analytics historiques avancees.

**Approche technique**  
Isoler l'acces `sqlx` derriere un repository orienté `NewsEvent` persiste. Verifier la coherence entre schema, indexes et requetes. Garder une pagination simple de type `limit + before` ou `limit` strict si aucun besoin de cursor n'est retenu pour le MVP.

**Criteres d'acceptation**

- les methodes repository couvrent les lectures nécessaires au MVP
- les indexes servent les requetes prevues
- les insertions restent idempotentes
- les sorties repository ne dependent pas d'un schema SQL brut cote handler

**Tests attendus**

- insert + lecture des derniers evenements
- filtre par source
- ordre inverse chronologique stable

**Dependances**

- `NEWS-004`
- `NEWS-007`

### `NEWS-010` Contrat WebSocket et diffusion avec backpressure

**Objectif**  
Formaliser le flux temps reel WebSocket.

**Scope**  
Definir la forme des messages, le type d'evenement, le comportement a la connexion, les limites de buffer, et la reaction aux consommateurs lents.

**Hors scope**  
Auth, multiplexage multi-topics ou replay historique automatique a la connexion.

**Approche technique**  
Stabiliser une enveloppe JSON simple pour les `news_item`, documenter si un message `hello` ou `subscribed` est necessaire, et traiter explicitement les cas `lagged` et `closed`. Le hub de broadcast doit rester non bloquant pour l'ingestion.

**Criteres d'acceptation**

- le contrat WebSocket est documente et coherent avec le modele de sortie
- un client lent ne bloque pas les autres consommateurs ni l'ingestion
- les logs de lag ou de fermeture sont explicites

**Tests attendus**

- connexion d'un client et reception d'un `news_item`
- perte de messages sur client lent selon la politique retenue
- serialisation JSON du payload sortant

**Dependances**

- `NEWS-003`
- `NEWS-009`

### `NEWS-011` Worker Telegram, retry, throttling et format des messages

**Objectif**  
Relayer le flux vers Telegram sans fragiliser le service.

**Scope**  
Definir le format de message, les conditions d'envoi, le retry minimum, le throttling, et la gestion des erreurs Telegram.

**Hors scope**  
Commandes Telegram bidirectionnelles ou interface d'administration.

**Approche technique**  
Faire du worker Telegram un consommateur secondaire du flux avec budget d'envoi propre. Utiliser un format texte robuste, un nombre borne de retries, et une politique de skip si Telegram est indisponible. Les erreurs doivent rester observables sans requeue infinie.

**Criteres d'acceptation**

- l'indisponibilite de Telegram n'interrompt pas l'ingestion
- le format de message est stable et lisible
- le nombre de tentatives et la temporisation sont documentes
- un burst d'evenements ne provoque pas de boucle de retry infinie

**Tests attendus**

- envoi nominal a plusieurs `chat_id`
- erreur HTTP Telegram avec retry borne
- verification du format final du message

**Dependances**

- `NEWS-001`
- `NEWS-010`

### `NEWS-012` Endpoints de lecture et filtres MVP

**Objectif**  
Donner une interface HTTP suffisante pour debug et premiers consommateurs.

**Scope**  
Finaliser `GET /api/news` et `GET /api/sources`, avec filtres MVP par source et limite, et eventuellement ajout d'un endpoint detail si necessaire au debug.

**Hors scope**  
API publique versionnee ou authn/authz.

**Approche technique**  
Faire converger les handlers sur la couche repository et sur des DTO de sortie explicites. Documenter les parametres query autorises, les bornes de `limit`, et la forme de `source status`.

**Criteres d'acceptation**

- les endpoints renvoient des payloads stables et documentes
- les filtres MVP couvrent les usages de debug immediats
- les handlers ne contiennent pas de logique SQL inline

**Tests attendus**

- lecture des `n` derniers evenements
- filtre par source inexistante et existante
- verification de bornage sur `limit`

**Dependances**

- `NEWS-009`
- `NEWS-010`
