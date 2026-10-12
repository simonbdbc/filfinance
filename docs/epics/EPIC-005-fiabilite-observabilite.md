# EPIC-005 - Fiabilite et observabilite

## Objectif

Durcir le service pour qu'il tienne dans le temps face aux incidents reseau, aux degradations de source et aux erreurs de parsing.

## Valeur livree

Un service observable, capable d'indiquer clairement ce qui se passe et de continuer a tourner proprement en mode degrade.

## Statut

A finir. Le socle de logs, runtime health et traces existe, mais l'observabilite doit etre consolidee apres `EPIC-024` et avant la reprise de `EPIC-026`.

## Contexte courant et dependances

Le service logue deja les erreurs principales, mais sans contrat d'observabilite ni budgets d'execution clairs. Cet EPIC depend des integrations reelles livrees par `EPIC-002`, `EPIC-003` et le pipeline audio valide par `EPIC-021`.

## Definition of Done

- logs structures et exploitables par source et par composant
- budgets de timeout/retry/jitter explicites
- sante source exposee de facon fiable
- payloads invalides conserves hors du hot path et rejouables

## Risques et points durs

- sur-instrumenter le service au point de penaliser le hot path
- masquer les erreurs de parsing dans des warnings trop generiques
- considerer une source "up" alors qu'elle ne fournit plus de donnees utiles
- accumuler des payloads invalides sans politique de retention

## Liste des tickets

- `NEWS-017` Logs structures, traces et correlation IDs
- `NEWS-018` Timeouts, retries, jitter et budgets de polling
- `NEWS-019` Etats de sante source, detection anti-bot et degradations
- `NEWS-020` Quarantaine des payloads invalides et reprise

## Details des tickets

### `NEWS-017` Logs structures, traces et correlation IDs

**Objectif**  
Rendre le service lisible en exploitation.

**Scope**  
Structurer les logs par source, composant, type d'erreur, et ajouter des identifiants de correlation ou de run pour suivre un evenement du fetch a la diffusion.

**Hors scope**  
Mise en place immediate d'une stack complete OpenTelemetry/Jaeger.

**Approche technique**  
Homogeneiser l'usage de `tracing` avec des champs fixes (`source`, `stage`, `signature`, `attempt`, `path`). Ajouter des spans ou contextes sur les cycles de polling et les evenements critiques. Documenter les messages clefs attendus.

**Criteres d'acceptation**

- les logs de fetch, parse, persist et diffuse sont distinguables
- un incident peut etre rattache a une source et a un composant
- les erreurs repetitives ne produisent pas des messages opaques

**Tests attendus**

- tests unitaires sur les champs de contexte ou adaptateurs de log si necessaire
- verification manuelle du format sur scenarios nominal et erreur

**Dependances**

- `NEWS-002`
- `NEWS-003`

### `NEWS-018` Timeouts, retries, jitter et budgets de polling

**Objectif**  
Encadrer le comportement reseau et la charge par source.

**Scope**  
Definir pour chaque source les timeouts HTTP, le nombre de retries, la politique de backoff, le jitter, et la relation entre intervalle de polling et duree max d'un cycle.

**Hors scope**  
Autoscaling ou ordonnanceur distribue.

**Approche technique**  
Introduire une configuration de budget par source et l'appliquer dans le runner commun. Garantir qu'un cycle lent ne cree pas un effet d'empilement infini. Documenter quand une erreur est retriee, quand elle est ignoree, et quand elle change l'etat de sante.

**Criteres d'acceptation**

- chaque source a un budget reseau explicite
- les retries sont bornes
- aucun cycle de polling ne s'empile indefiniment
- le jitter evite le martelage synchrone des sources

**Tests attendus**

- simulation d'un timeout HTTP
- simulation de retries puis abandon
- verification qu'un collecteur lent ne cree pas de recouvrement infini

**Dependances**

- `NEWS-001`
- `NEWS-002`

### `NEWS-019` Etats de sante source, detection anti-bot et degradations

**Objectif**  
Savoir quand une source est reellement saine ou degradee.

**Scope**  
Definir un etat de sante par source, les symptomes de degradation, les heuristiques de detection anti-bot, et les signaux exposes via `GET /api/sources` ou un endpoint dedie.

**Hors scope**  
Dashboard graphique ou alerting externe complet.

**Approche technique**  
Conserver par source les dates du dernier fetch reussi, du dernier parse utile, du dernier evenement emis, le nombre d'erreurs recentes et un statut normalise (`healthy`, `degraded`, `blocked`, `idle`). Considerer comme degrade une source qui repond HTTP 200 mais ne livre plus de payload exploitable.

**Criteres d'acceptation**

- l'etat de sante ne se limite pas au statut HTTP
- la detection de blocage/degradation est basee sur des regles explicites
- `api/sources` ou equivalent expose des informations de sante utiles

**Tests attendus**

- source saine avec emissions recentes
- source qui repond mais sans data utile
- source en erreur repetee ou probablement bloquee

**Dependances**

- `NEWS-012`
- `NEWS-018`

### `NEWS-020` Quarantaine des payloads invalides et reprise

**Objectif**  
Ne pas perdre silencieusement les cas invalides et faciliter le debug.

**Scope**  
Stocker ou journaliser de facon exploitable les payloads bruts qui n'ont pas pu etre parses, avec motif d'erreur et metadonnees minimales, puis definir la strategie de reprise ou de relecture.

**Hors scope**  
Interface graphique d'administration de la quarantaine.

**Approche technique**  
Ajouter une zone de quarantaine en base ou sur disque, avec retention bornee, reference de source, hash du payload, horodatage, et raison de rejet. Prevoir une commande ou une procedure documentee pour rejouer ces payloads dans un parseur local.

**Criteres d'acceptation**

- un payload invalide n'est pas seulement logue puis oublie
- la retention est bornee et documentee
- le motif de rejet permet le debug sans re-fetch obligatoire

**Tests attendus**

- insertion d'un payload invalide en quarantaine
- verification de la retention ou du bornage
- test de relecture locale de payload brut

**Dependances**

- `NEWS-004`
- `NEWS-017`
- `NEWS-019`
