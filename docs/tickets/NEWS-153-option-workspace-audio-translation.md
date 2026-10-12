# NEWS-153 - Option non prioritaire workspace audio et translation

## Objectif

Documenter une evolution possible vers un Cargo workspace pour isoler les dependances lourdes `audio` et `translation`.

Cette evolution n'est pas prioritaire tant que le backend reste un binaire unique et que les frontieres internes restent stables dans le crate actuel.

## Motivation

Les features `audio` et `translation` concentrent des dependances et contraintes runtime plus lourdes que le reste du backend:

- Piper et decouverte de voix
- HLS et encodage `ffmpeg`
- provider local OPUS-MT
- sessions long-vivantes de commande locale
- observabilite runtime specifique

## Architecture cible possible

```text
filfinance-service/
  Cargo.toml
  crates/
    api/
      src/main.rs
      src/app_state.rs
      src/router.rs
      src/workers.rs
    core/
      src/errors.rs
      src/time.rs
      src/text.rs
      src/pagination.rs
    news/
      src/domain/
      src/sqlite/
    translation/
      src/domain/
      src/provider/
      src/providers/
      src/runtime/
      src/service/
    audio/
      src/domain/
      src/tts/
      src/hls/
      src/service/
```

## Etapes recommandees

1. Stabiliser les contrats entre `news`, `translation`, `audio` et `live`.
2. Extraire un crate `core` minimal sans dependances lourdes.
3. Extraire `translation` avec une facade publique petite.
4. Extraire `audio` avec une facade publique petite.
5. Garder `api` responsable du wiring Axum, `AppState`, config et workers.
6. Migrer les tests unitaires proches dans chaque crate.
7. Garder les tests d'integration bout-en-bout dans `api`.

## Avantages

- Frontieres plus fortes que `pub(crate)`.
- Dependances lourdes isolees.
- CI plus ciblable par domaine.
- Preparation a une separation future de workers.

## Consequences

- Plus de `Cargo.toml`.
- Plus de types doivent devenir publics.
- Risque de cycles si les contrats ne sont pas stabilises avant extraction.
- Refactor plus couteux qu'une organisation en modules.

## Decision actuelle

Ne pas lancer maintenant.

Le single crate feature-first reste l'architecture nominale jusqu'a ce qu'une pression concrete apparaisse: temps de compilation, CI ciblee, separation de workers, ou ownership technique distincte.
