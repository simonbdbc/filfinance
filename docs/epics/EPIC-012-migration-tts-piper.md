# EPIC-012 - Migration TTS serveur vers Piper

## Objectif

Remplacer le provider TTS `macos_say` par un provider local `Piper`, pilote depuis `filfinance-service` via une session Python long-vivante.

## Valeur livree

Le service ne depend plus d'une voix systeme macOS. La voix FR serveur devient portable au niveau du projet, versionnee par scripts/bootstrap d'assets et compatible avec le contrat `/ws/audio` deja en production.

## Statut

En cours avance au 23 avril 2026.

Extension qualite/perf validee le 3 mai 2026, puis reprise diction v2 ouverte le 2 juin 2026.

## Etat reel au 23 avril 2026

Ce qui est deja livre dans le repo:

- provider Rust `piper_local_session` branche dans le worker TTS
- helper Python `scripts/piper_local_session.py` en protocole JSONL long-vivant
- suppression du provider `macos_say` dans le code Rust, les tests et la configuration active
- catalogue local des voix Piper francaises via `data/models/piper/<voice>/`
- flattening multi-speaker cote produit avec des IDs `Language-Voice-Quality-Speaker`
- voix par defaut fixee a `fr_FR-upmc-medium-jessica`
- conservation du contrat `/ws/audio` et du selecteur de voix cote client
- script de bootstrap local `scripts/bootstrap_piper_local.sh`
- benchmark local `scripts/benchmark_tts_local.py` recable sur Piper
- runtime Docker Piper package par `NEWS-125`

Ce qui reste hors repo-code et doit encore etre execute localement:

- bootstrap reel de `.venv-piper`
- telechargement de `fr_FR-upmc-medium`
- validation audio live bout en bout sur la machine de dev
- normalisation TTS finance dediee au `narration_text`
- pools Piper mesures si la queue TTS devient le goulot

## Tickets

- `NEWS-057` fait cote code
- `NEWS-058` fait cote code
- `NEWS-059` fait cote code
- `NEWS-060` fait cote scripts et observabilite
- `NEWS-061` en cours sur la documentation et le runbook
- `NEWS-125` fait cote repo
- `NEWS-128` livre v1, a reprendre par `NEWS-195`
- `NEWS-130` a faire
- `NEWS-195` en cours: diction finance v2 avec la meme voix `fr_FR-upmc-medium-jessica`

## Points de cadrage

- `Piper` est pilote par un helper Python embarque, pas par un sidecar HTTP
- seules les voix FR detectees localement sont exposees au client
- les assets de voix restent hors git et vivent dans `data/models/piper`
- `macos_say` n'est plus un fallback nominal du projet

## Tickets extension qualite/perf

### [`NEWS-128` Normalisation TTS finance](../tickets/NEWS-128-normalisation-tts-finance.md)

Objectif: ameliorer la comprehension audio sans modifier le texte affiche.

Statut: v1 livree mais insuffisante; la reprise produit est portee par [`NEWS-195`](../tickets/NEWS-195-reprise-diction-tts-finance-v2.md).

Scope:

- normaliser acronymes, horaires, devises, `bps`, pourcentages et tickers
- appliquer les transformations au `narration_text`
- ajouter des tests unitaires Rust sur fixtures finance

Criteres d'acceptation:

- le texte affiche reste stable
- le texte parle est plus naturel
- les chiffres critiques restent exacts

### [`NEWS-195` Reprise diction TTS finance v2](../tickets/NEWS-195-reprise-diction-tts-finance-v2.md)

Objectif: reprendre la diction sans changer la voix Piper.

Scope:

- conserver `NEWS_TTS_VOICE=fr_FR-upmc-medium-jessica`
- renforcer les fixtures macro
- appliquer `spoken_term` du glossaire uniquement au texte envoye a Piper

Criteres d'acceptation:

- `FR affiche` et `FR vocal` restent separes
- les formes longues vocales ne polluent pas le client
- la diction est moins mecanique sur les fixtures finance

### [`NEWS-130` Pools workers traduction et TTS](../tickets/NEWS-130-pools-workers-traduction-tts.md)

Objectif: augmenter le debit avec des sessions long-vivantes mesurees.

Scope:

- `NEWS_TTS_WORKERS`
- pool Piper
- observabilite de queue et workers actifs
- benchmark des valeurs `1`, `2`, `3`, `4`

Criteres d'acceptation:

- le mode `1 worker` garde le comportement historique
- les workers supplementaires augmentent le debit sans saturer CPU/RAM
