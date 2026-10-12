# NEWS-168 - Re-audit EPIC-010 et rattrapage commentaires Rust

## Objectif

Remettre `EPIC-010` au niveau de l'arborescence Rust actuelle.

L'EPIC reste valide comme standard transverse, mais son audit de reference `NEWS-047`
date d'un service beaucoup plus petit. Le backend contient maintenant plusieurs zones
recentes et critiques qui doivent etre recontrolees puis commentees selon le standard
pedagogique Rust.

## Declencheur

Audit ponctuel du 12 mai 2026:

- `NEWS-047` reference encore 16 fichiers Rust et 7031 lignes.
- L'arborescence actuelle contient 524 fichiers Rust et 32252 lignes.
- Les gates `cargo fmt --check`, `cargo check` et `cargo test` passent.
- La conformite documentaire EPIC-010 a derive sur plusieurs modules recents.

## Scope

1. Mettre a jour `NEWS-047` avec l'inventaire actuel:
   - nombre de fichiers Rust;
   - volumes par famille fonctionnelle;
   - presence de tests;
   - niveau de commentaire pedagogique reel;
   - priorisation du rattrapage.
2. Lancer une campagne ciblee sur les zones recentes a plus fort risque:
   - `features/health`;
   - `features/audio/hls`;
   - `features/live/edge`;
   - `features/logs/tests.rs`;
   - `features/ingestion`;
   - `features/audio`;
   - `features/live`.
3. Verifier que les tests Rust touches expliquent le scenario et le risque de regression.
4. Conserver le vocabulaire produit impose par `RUST-COMMENTING-STANDARD`.

## Hors scope

- Reouvrir toute la campagne EPIC-010 comme chantier global.
- Recommenter mecaniquement les 524 fichiers Rust sans priorisation.
- Refactorer les modules metier sans besoin direct.
- Introduire un lint semantique de commentaires.

## Approche technique

### Phase 1 - Re-audit

Reprendre `docs/tickets/NEWS-047-audit-couverture-rust.md` pour le rendre exact sur le code
courant. Le document doit distinguer:

- le perimetre historique couvert par `NEWS-048` et `NEWS-049`;
- les modules crees ou fortement remanies depuis la cloture d'EPIC-010;
- les fichiers conformes, partiellement conformes et non conformes;
- les tests Rust sans commentaire de scenario.

### Phase 2 - Rattrapage priorise

Traiter les zones dans cet ordre:

1. `features/health`: handlers admin, snapshots runtime, redaction et statut degrade.
2. `features/audio/hls`: tickets HLS, playlists, origin, cache headers, metrics et tests.
3. `features/live/edge`: publication, signatures, tickets, DTO edge et worker.
4. `features/logs/tests.rs`: tests admin logs J/J-1, pagination, tail snapshot et append.
5. `features/ingestion`: runtime, collecteurs, SignalR, dedup et tests.
6. `features/audio`: worker TTS, generation progressive, runtime et normalisation.
7. `features/live`: WebSocket backend, DTO, delivery replay, heartbeats et tests.

Chaque lot doit rester petit et se terminer par les gates Rust.

## Criteres d'acceptation

- `NEWS-047` decrit l'arborescence Rust actuelle, pas seulement l'etat du 22 avril 2026.
- Les zones a risque sont classees avec un statut de couverture exploitable.
- Les modules traites ont des commentaires utiles, en francais, qui expliquent:
  - le role metier;
  - la specificite Rust;
  - le choix d'architecture;
  - le risque de maintenance ou de regression.
- Les tests Rust modifies expliquent le scenario et le risque couvert.
- Aucun vocabulaire produit obsolete n'est reintroduit.
- `cargo fmt --check`, `cargo check` et `cargo test` passent apres chaque lot.

## Tests attendus

Pour chaque lot de rattrapage:

```bash
cargo fmt --check
cargo check
cargo test
```

Si un lot touche aussi un contrat client ou un script Node associe, ajouter la verification
specifique du package concerne.

## Dependances

- `NEWS-046`
- `NEWS-047`
- `NEWS-048`
- `NEWS-049`
- `NEWS-050`

## Statut

En cours.

Phase 1 demarree le 12 mai 2026: `NEWS-047` a ete remis a jour avec les volumes
actuels, les familles fonctionnelles et l'ordre de rattrapage cible.

Premiere tranche de la vague 1 livree le 12 mai 2026:

- `features/health/http.rs` commente sur les probes admin, snapshots runtime, redaction
  et helpers de statut.
- `features/logs/tests.rs` commente sur les scenarios J/J-1, pagination, tail snapshot
  et append.
- Gates passees: `cargo fmt --check`, `cargo check`, `cargo test`.
