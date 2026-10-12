# NEWS-217 - Contrats Rust calendrier et parite figee

> Statut : implemente le 25 aout 2026. Le standard pedagogique Rust
> [`RUST-COMMENTING-STANDARD`](../RUST-COMMENTING-STANDARD.md) et la
> [`RUST-REVIEW-CHECKLIST`](../RUST-REVIEW-CHECKLIST.md) s'appliquent a tout le
> scope Rust de ce ticket ; le ticket ne peut pas etre clos sans commentaires
> coherents et controles Rust explicites.

## Objectif

Extraire les contrats et validations aujourd'hui repartis entre les workflows n8n
et le backend en types Rust purs, avant tout appel reseau natif.

## Scope

- Types `CalendarSourceSnapshot`, `CalendarIngestionDraft`, `CalendarItemDraft`,
  `CalendarSummaryDraft` et erreurs de validation.
- Normalisation, deduplication, francais visible, `final_importance`, IDs `high`
  et coherence summary.
- Fixtures versionnees et golden tests hors reseau.

## Criteres d'acceptation

- Les fixtures valides produisent exactement le payload canonique attendu.
- Les fixtures vides, anglaises, incoherentes ou sans jour cible sont rejetees.
- Aucun changement runtime ni ecriture SQLite native n'est introduit.

## Dependances

- [`EPIC-037`](../epics/EPIC-037-orchestrateur-rust-natif-calendrier.md)
- [`EPIC-032`](../epics/EPIC-032-refonte-calendrier-du-jour-n8n-audio-piper.md)

## Implementation

Scope Rust :

- `filfinance-service/src/features/calendar_day/contracts.rs` contient les six
  contrats cibles, les enums auxiliaires et les fonctions pures ;
- `filfinance-service/src/features/calendar_day.rs` declare seulement ce
  sous-module, sans l'appeler depuis le runtime legacy ;
- `filfinance-service/tests/fixtures/calendar_day/` contient les snapshots,
  reponses IA et goldens versionnes.

Le contrat fige les deux sources autorisees, le jour cible ouvrable, les formats
JSON toleres par n8n, les alias historiques, les champs visibles francais, la
regle `high`, le clamp de confiance, la categorie, les provenances et les
doublons exacts. Le jour d'un evenement est compare dans le fuseau produit
Europe/Paris. Les erreurs et rapports sont rediges. Les tailles de reponse,
items, sources par item, texte source et bloc `raw` sont bornees.

`CalendarParityReport::compare_ingestion_strict` couvre uniquement l'egalite
deterministe des fixtures. Le comparateur d'invariants shadow live reste
explicitement dans `NEWS-218`, car une variation textuelle LLM ne doit pas etre
une divergence critique a elle seule.

## Verification

- `cargo test calendar_day::contracts::tests` : 20 tests passes, 0 echec ;
- `cargo test --manifest-path filfinance-service/Cargo.toml` : 289 tests
  passes, 0 echec ;
- `cargo check` : passe ; un warning `unused_imports` preexistant subsiste dans
  `src/features/glossary/pipeline.rs` ;
- `cargo fmt --all -- --check` : passe ;
- `node scripts/check-n8n-calendar-language-guard.cjs` : passe.

La verification couvre source valide Investing/ForexFactory, jour absent ou
week-end, golden ingestion/resume, doublons, JSON IA vide/invalide/fenced/repare,
alias, coercitions et formes de provenance n8n, anglais visible, jour produit
Europe/Paris, bornes a la limite et limite + 1, hash stable, coherence
compte/IDs/texte `high` et rapport de parite redige.

## Non-changements runtime

Aucun appel reseau natif, flag shadow, ecriture SQLite, audio, alerte ou nouveau
contrat HTTP/WebSocket n'est introduit. Les workflows n8n restent inchanges et
actifs comme baseline.

## Risque reporte a la phase 2

Le texte d'un `CalendarSourceSnapshot` est volontairement prive, non serialise
et redige dans `Debug`. `NEWS-218` devra donc ajouter un hand-off transitoire en
memoire qui remet exactement le meme snapshot a n8n et au shadow natif, sans
affaiblir cette redaction et sans persister le texte source.
