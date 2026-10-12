# NEWS-047 - Audit de couverture des commentaires Rust

## Objectif

Produire un etat des lieux exploitable du code Rust existant dans
`filfinance-service`, afin de piloter la couverture pedagogique imposee par
`EPIC-010`.

Ce document a d'abord servi a lancer `NEWS-048` et `NEWS-049`. Il est maintenant
maintenu comme inventaire vivant de la dette documentaire Rust.

## Statut

Re-audite le 12 mai 2026 dans le cadre de `NEWS-168`.

L'audit historique du 22 avril 2026 portait sur 16 fichiers Rust et 7031 lignes.
Il reste utile pour comprendre la cloture initiale d'EPIC-010, mais il ne decrit
plus l'arborescence actuelle. Le service a ete fortement modularise depuis.

## Perimetre audite

Scope audite au 12 mai 2026:

- tous les fichiers `*.rs` sous `filfinance-service/src`
- les tests inline portes par `#[test]` ou `#[tokio::test]`
- les modules recents ajoutes apres la cloture initiale d'EPIC-010

Il n'existe toujours pas, a cette date, de dossier `tests/` dedie. Les tests Rust
restent embarques directement dans les modules source.

## Methode

L'audit combine cinq angles:

1. inventaire exhaustif des fichiers Rust existants
2. volume par famille fonctionnelle
3. presence brute de commentaires `//`, `//!` et `///` comme indicateur faible
4. presence de tests Rust inline
5. lecture ciblee des modules recents les plus risques pour un developpeur venant
   surtout de `JavaScript`, `PHP` et `Java`

Point important:

- un ratio de commentaires n'est pas un critere de done
- un fichier peut avoir beaucoup de commentaires et rester pedagogiquement faible
- un fichier peut avoir peu de lignes et etre critique si le pattern Rust est dense
- le standard applicable reste `docs/RUST-COMMENTING-STANDARD.md`

## Synthese executive

- 524 fichiers Rust audites
- 32252 lignes Rust au total
- 4410 lignes de commentaires `//`, `//!` ou `///` detectees
- ratio brut d'environ `13.67%`
- 17 fichiers contiennent au moins un test Rust inline
- 167 tests Rust sont detectes par attribut `#[test]` ou `#[tokio::test]`
- baseline initial: 1 fichier de plus de 20 lignes n'avait aucun commentaire:
  `features/logs/tests.rs`

Conclusion:

- le standard EPIC-010 reste en vigueur
- la cloture initiale de `NEWS-048` et `NEWS-049` reste historiquement valide
- l'arborescence actuelle a derive au-dela du scope couvert en avril 2026
- le rattrapage doit etre cible sur les modules recents a fort risque, pas lance
  comme une mega-campagne sur les 524 fichiers

## Inventaire par famille

| Famille | Fichiers | Lignes | Commentaires | Fichiers avec tests | Tests detectes | Etat de couverture |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| Socle, config, shared, router, workers | 31 | 3051 | 670 | 1 | 5 | Partiellement conforme; plusieurs fichiers historiques sont bien expliques, mais `config/sources.rs`, `config/runtime.rs` et routes restent a reprendre au fil des changements. |
| Auth et profils | 43 | 2324 | 419 | 2 | 18 | Globalement lisible, mais les tests et certains adaptateurs Postgres doivent rester surveilles a chaque evolution auth. |
| Ingestion | 89 | 6122 | 805 | 1 | 28 | Zone prioritaire: runtime, monitor, collecteurs FinancialJuice, X, SignalR et tests ont beaucoup evolue depuis l'audit initial. |
| News, calendrier, logs, health | 85 | 7140 | 721 | 6 | 46 | Zone prioritaire: `health/http.rs`, `logs/tests.rs`, `news/status.rs` et certains chemins SQLite ont une couverture pedagogique faible. |
| Traduction | 66 | 2969 | 359 | 2 | 14 | Couverture correcte sur les concepts centraux, mais les providers/session et les tests restent a maintenir strictement. |
| Audio, TTS, HLS | 131 | 6178 | 797 | 2 | 33 | Zone prioritaire: HLS, tickets, origin, worker TTS, generation progressive et tests audio sont critiques et recents. |
| Live, edge, WebSocket | 64 | 3229 | 518 | 2 | 19 | Zone prioritaire: edge publisher, tickets, DTO edge, worker et streams live recents doivent etre consolides. |
| Notifications | 13 | 897 | 89 | 1 | 4 | Risque moyen; Telegram est stable mais les tests restent peu pedagogiques par endroits. |
| Autres modules racine | 2 | 342 | 32 | 0 | 0 | Risque faible. |

## Fichiers a risque detectes mecaniquement au baseline

Cette liste ne remplace pas la review humaine. Elle signale les fichiers de 80 lignes
ou plus avec le plus faible ratio brut de commentaires.

| Ratio commentaires | Lignes | Commentaires | Tests | Fichier | Priorite |
| ---: | ---: | ---: | ---: | --- | --- |
| 0.00% | 193 | 0 | 5 | `src/features/logs/tests.rs` | Haute - traite vague 1 |
| 1.15% | 349 | 4 | 0 | `src/features/health/http.rs` | Haute - traite vague 1 |
| 1.93% | 259 | 5 | 0 | `src/features/news/sqlite/cleanup.rs` | Moyenne |
| 2.17% | 92 | 2 | 0 | `src/features/ingestion/monitor.rs` | Haute |
| 2.53% | 158 | 4 | 2 | `src/features/news/status.rs` | Moyenne |
| 2.68% | 112 | 3 | 1 | `src/features/translation/service/processing/broadcast.rs` | Moyenne |
| 2.94% | 102 | 3 | 1 | `src/features/live/edge/publisher/request.rs` | Haute |
| 3.52% | 682 | 24 | 28 | `src/features/ingestion/tests.rs` | Haute |
| 3.53% | 85 | 3 | 0 | `src/features/audio/service/stream/prepare.rs` | Haute |
| 3.88% | 206 | 8 | 0 | `src/features/news/sqlite/writes/event/duplicate.rs` | Moyenne |
| 3.95% | 228 | 9 | 6 | `src/features/audio/hls/access.rs` | Haute |
| 4.05% | 321 | 13 | 0 | `src/config/sources.rs` | Moyenne |
| 4.32% | 139 | 6 | 5 | `src/config/runtime.rs` | Moyenne |
| 4.42% | 1719 | 76 | 35 | `src/features/news/tests.rs` | Moyenne |
| 4.65% | 86 | 4 | 0 | `src/features/audio/service/progressive/intro/workflow.rs` | Haute |

## Lecture des risques par famille

### 1. Health et observabilite admin

Fichiers representatifs:

- `features/health/http.rs`
- `features/logs/service.rs`
- `features/logs/websocket.rs`
- `features/logs/tests.rs`

Risque principal:

- les payloads admin agregeent beaucoup de snapshots runtime, de redaction et de
  statuts derives; sans commentaires pedagogiques, un changement peut exposer trop
  d'information ou masquer un etat degrade
- au baseline, `features/logs/tests.rs` n'avait aucun commentaire alors qu'il fixe le contrat J/J-1,
  pagination, tail snapshot et append

Decision:

- traiter ce bloc en premier sous `NEWS-168`

### 2. HLS et data plane audio

Fichiers representatifs:

- `features/audio/hls/access.rs`
- `features/audio/hls/http/*`
- `features/audio/hls/origin/*`
- `features/audio/hls/runtime/*`
- `features/audio/tests.rs`

Risque principal:

- tickets HLS, cache headers, playlists, segments, origin local, metrics et tests de
  charge sont directement lies a la stabilite production et aux validations 500/1000
  auditeurs

Decision:

- traiter ce bloc juste apres health/logs

### 3. Edge, Durable Objects et WebSocket live

Fichiers representatifs:

- `features/live/edge/publisher/request.rs`
- `features/live/edge/ticket/*`
- `features/live/edge/worker/*`
- `features/live/dto/edge/*`
- `features/live/websocket/*`

Risque principal:

- signatures HMAC, tickets courts, contrats DTO et fan-out metadata touchent la securite
  et la charge; les patterns async et les contrats Worker doivent etre explicites

Decision:

- traiter ce bloc avant de reprendre les modules live generaux

### 4. Ingestion

Fichiers representatifs:

- `features/ingestion/runtime.rs`
- `features/ingestion/http.rs`
- `features/ingestion/monitor.rs`
- `features/ingestion/tests.rs`
- `features/ingestion/collectors/financial_juice/**`
- `features/ingestion/collectors/x.rs`

Risque principal:

- c'est le hot path d'entree; SignalR, polling, dedup, fallback navigateur et tests
  doivent rester lisibles pour eviter des regressions silencieuses de source

Decision:

- traiter ce bloc apres les surfaces production health/HLS/edge

### 5. Audio, TTS et generation progressive

Fichiers representatifs:

- `features/audio/service/**`
- `features/audio/tts/**`
- `features/audio/domain.rs`
- `features/audio/tests.rs`

Risque principal:

- worker TTS, watermarks, normalisation finance, progressive clips et process Piper
  combinent ownership Rust, I/O async, process externes et etat partage

Decision:

- traiter ce bloc apres HLS, en gardant les lots petits

### 6. News, stockage et tests massifs

Fichiers representatifs:

- `features/news/tests.rs`
- `features/news/sqlite/**`
- `features/news/status.rs`
- `features/calendar/**`

Risque principal:

- les tests et requetes SQL fixent beaucoup de contrats implicites; le besoin est moins
  urgent que health/HLS/edge, mais il faut eviter que la dette documentaire continue
  de grossir

Decision:

- traiter au fil des tickets qui modifient ces zones, ou en lot separe si la dette bloque
  la review

## Ordre de traitement recommande

L'ordre ci-dessous privilegie la reduction de risque production et la comprehension des
zones recemment modifiees.

### Vague 1 - Health et logs admin

- `features/health/http.rs`
- `features/logs/tests.rs`
- au besoin `features/logs/service.rs` et `features/logs/websocket.rs`

Objectif:

- rendre explicites les payloads admin, la redaction, le statut degrade et les contrats
  de lecture/tail des logs

### Vague 2 - HLS et tickets audio

- `features/audio/hls/access.rs`
- `features/audio/hls/http/*`
- `features/audio/hls/origin/*` par petits lots
- tests HLS dans `features/audio/tests.rs`

Objectif:

- documenter la securite d'acces HLS, les decisions de cache et le contrat playlist/segment

### Vague 3 - Edge metadata

- `features/live/edge/publisher/request.rs`
- `features/live/edge/ticket/*`
- `features/live/edge/worker/*`
- `features/live/dto/edge/*`

Objectif:

- documenter signatures, tickets, retries, contrats Worker et payloads publics

### Vague 4 - Ingestion

- `features/ingestion/runtime.rs`
- `features/ingestion/monitor.rs`
- `features/ingestion/tests.rs`
- `features/ingestion/collectors/financial_juice/**`

Objectif:

- rendre lisibles les decisions de source, dedup, fallback navigateur, polling et SignalR

### Vague 5 - Audio service et live backend

- `features/audio/service/**`
- `features/audio/tts/**`
- `features/live/websocket/**`
- `features/live/tests.rs`

Objectif:

- couvrir les workers, watermarks, normalisation, WebSocket backend et heartbeats

## Recommandations d'execution

- eviter une mega-PR de commentaires sur les 32252 lignes
- viser des lots coherents de 1 a 3 fichiers maximum
- commenter aussi les tests modifies dans la meme vague
- preferer des commentaires utiles qui expliquent la contrainte Rust ou produit
- refuser les commentaires qui paraphrasent simplement le code
- verifier a chaque vague:

```bash
cargo fmt --check
cargo check
cargo test
```

## Statut de NEWS-047

Ticket historiquement fait pour la cloture initiale d'EPIC-010.

Depuis le re-audit du 12 mai 2026, ce document sert aussi de point de depart a `NEWS-168`.

Etat de rattrapage au 12 mai 2026:

- vague 1 initiale livree pour `features/health/http.rs` et `features/logs/tests.rs`
- gates Rust passees apres cette tranche: `cargo fmt --check`, `cargo check`, `cargo test`

Le prochain etat acceptable est:

- vague 2 `HLS` commentee et testee
- vague 3 `edge` commentee et testee
- backlog restant confirme ou redecoupe selon les prochains tickets Rust
