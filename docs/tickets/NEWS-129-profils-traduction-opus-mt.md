# NEWS-129 - Profils traduction OPUS-MT

## Objectif

Ajouter des profils de traduction explicites pour arbitrer qualite et latence sans modifier manuellement les arguments bas niveau OPUS-MT.

## Priorite

Priorite 4.

Ce ticket doit venir apres l'observabilite minimale pour pouvoir comparer les profils proprement.

## Profils cibles

```env
NEWS_TRANSLATION_PROFILE=fast
NEWS_TRANSLATION_PROFILE=balanced
NEWS_TRANSLATION_PROFILE=quality
```

Mapping initial:

- `fast`: `beam-size=1`
- `balanced`: `beam-size=2`
- `quality`: `beam-size=4`

## Scope

- Ajouter la variable de config `NEWS_TRANSLATION_PROFILE`.
- Refuser `--beam-size` dans `NEWS_TRANSLATION_COMMAND_ARGS` pour garder le profil comme source de verite.
- Resoudre le profil en arguments OPUS-MT effectifs.
- Exposer le profil actif dans `/api/runtime/translation`.
- Benchmarker les trois profils sur le corpus de headlines FinancialJuice.

## Critere d'acceptation

- Le profil `quality` reproduit le comportement actuel `beam-size=4`.
- Le profil `balanced` sert de candidat production par defaut si la qualite reste acceptable.
- Le profil `fast` est disponible pour mode degrade ou machine faible.
- Les resultats benchmark documentent latence moyenne, p95 et exemples de degradation.

## Livraison 2026-05-03

Implementation:

- `NEWS_TRANSLATION_PROFILE` accepte `fast`, `balanced` et `quality`; l'absence de variable garde `quality`.
- Mapping effectif: `fast -> beam-size=1`, `balanced -> beam-size=2`, `quality -> beam-size=4`.
- Les aliases OPUS-MT resolvent le profil avant de lancer le helper; un `--beam-size` encore present dans `NEWS_TRANSLATION_COMMAND_ARGS` est maintenant une erreur de configuration.
- `/api/runtime/translation` expose maintenant le champ `profile`.
- `scripts/benchmark_opus_mt_local.py --all-profiles --json` mesure les trois profils en une passe.

Benchmark local du 3 mai 2026, macOS arm64, CPU, int8, `12` headlines FinancialJuice:

| Profil | Beam | Moyenne | p95 | Debit |
| --- | ---: | ---: | ---: | ---: |
| `fast` | 1 | `296.9 ms` | `761.95 ms` | `3.37 headlines/s` |
| `balanced` | 2 | `322.4 ms` | `413.4 ms` | `3.1 headlines/s` |
| `quality` | 4 | `379.0 ms` | `683.38 ms` | `2.64 headlines/s` |

Exemples observes:

- `quality` reproduit le comportement historique `beam-size=4`.
- `balanced` ameliore la p95 sur ce run mais degrade le premier exemple en produisant `Les taux de signaux de la Reserve federale...`.
- `fast` est le plus rapide en moyenne, mais sa p95 reste instable sur ce corpus court.

Decision provisoire: garder `quality` comme defaut conservateur, tester `balanced` en production seulement apres comparaison sur les vraies headlines live.

## Hors scope

- Pool de sessions.
- Micro-batching.
- Changement de modele.
