# NEWS-130 - Pools workers traduction et TTS

## Objectif

Augmenter le debit de traduction et de TTS avec des pools mesures, sans saturer la machine serveur.

## Priorite

Priorite 5.

Ce ticket doit etre precede par `NEWS-127`, sinon le dimensionnement sera fait a l'aveugle.

## Variables cible

```env
NEWS_TRANSLATION_WORKERS=1
NEWS_TTS_WORKERS=1
```

Valeurs de test:

- commencer a `2`;
- benchmarker `3`;
- ne tester `4` que si CPU/RAM et latence restent stables.

## Regle de dimensionnement

Ne pas depasser les coeurs physiques avec le produit:

```text
workers * inter_threads * intra_threads
```

Pour OPUS-MT/CTranslate2, preferer plusieurs sessions avec peu de threads par session plutot qu'une seule session sur-threadée, seulement si les mesures le confirment.

## Scope

- Ajouter un pool de sessions OPUS-MT long-vivantes.
- Ajouter un pool de sessions Piper long-vivantes.
- Appliquer une strategie de dispatch simple et observable.
- Exposer la taille de queue et le nombre de workers actifs.
- Benchmarker les combinaisons `1/2/3/4`.

## Critere d'acceptation

- Aucun item n'est perdu quand plusieurs workers sont actifs.
- L'ordre user-facing reste coherent.
- Le debit augmente sur burst sans degradation majeure de latence p95.
- La RAM reste compatible avec le serveur personnel.
- Une configuration `1 worker` conserve le comportement historique.

## Hors scope

- GPU.
- Micro-batching.
- Refonte du pipeline d'ingestion.
