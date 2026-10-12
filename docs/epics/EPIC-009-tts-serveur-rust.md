# EPIC-009 - TTS serveur pilote par Rust

## Objectif

Introduire une couche TTS cote serveur, pilotee par le service Rust, pour transformer le texte francais derive en audio streamable en memoire.

## Valeur livree

Le service produit une voix FR homogene et maitrisee, independante du navigateur client, et prepare le terrain du streaming audio temps reel sans fichiers.

## Statut

Fait au 23 avril 2026.

## Contexte courant et dependances

Cet EPIC suit directement `EPIC-008`, car il consomme `narration_text` et le contrat de traduction FR. Il fournit ensuite le prerequis technique direct de `EPIC-007`.

## Etat reel au 23 avril 2026

Le bloc TTS serveur MVP est maintenant livre dans le repo:

- un `TtsProvider` Rust existe
- le premier provider livre dans l'historique du repo etait `macos_say`
- le worker TTS consomme le meme flux traduit que Telegram et `/ws/news`
- la sortie WAV est decoupee en chunks et diffusee vers le bus audio interne puis `/ws/audio`
- le warmup, les timeouts et les traces E2E sont branches
- la narration FR est normalisee avant synthese pour rendre paires FX, pourcentages et tickers plus lisibles
- un endpoint runtime `/api/runtime/tts` expose l'etat et les metriques TTS
- un benchmark local reproductible existe pour le provider initial puis pour son remplaçant `Piper`
- le cadrage de cout MVP et la migration vers un provider local versionnable sont documentes

## Definition of Done

- une abstraction `TtsProvider` cote Rust est definie
- un moteur TTS serveur local ou embarquable peut etre pilote depuis le service Rust
- l'audio est genere en memoire pour le client, independamment du provider TTS concret choisi ensuite
- le format audio de sortie et la strategie de chunking sont documentes
- la TTS ne bloque pas l'ingestion ni la diffusion Telegram
- les couts locaux du service TTS sont documentes: CPU ou GPU, RAM, latence et debit

## Risques et points durs

- choix de la voix FR et qualite percue
- cout CPU ou GPU de la synthese
- compromis entre PCM brut, compression, latence et compatibilite navigateur
- gestion des longues phrases et de la ponctuation de marche
- complexite d'une integration Rust propre si le moteur TTS n'est pas nativement Rust

## Liste des tickets

- `NEWS-036` Cadrage moteur TTS, format audio et enveloppe de cout
- `NEWS-037` Abstraction Rust `TtsProvider` et orchestration serveur
- `NEWS-038` Generation audio en memoire et chunking streamable
- `NEWS-039` Normalisation de narration, benchmark et observabilite TTS

## Details des tickets

### `NEWS-036` Cadrage moteur TTS, format audio et enveloppe de cout

**Objectif**  
Choisir le moteur TTS cible du MVP et cadrer son cout local d'exploitation.

**Scope**  
Comparer les moteurs cibles, choisir une voix FR, fixer le format audio de sortie, le taux d'echantillonnage, la granularite des chunks, et documenter l'enveloppe machine cible.

**Hors scope**  
Implementation de l'orchestrateur Rust.

**Approche technique**  
Valider un moteur TTS compatible avec un pilotage serveur robuste depuis Rust, puis documenter les contraintes de licence, de voix, de machine et de latence.

**Criteres d'acceptation**

- le moteur TTS MVP et la voix FR cible sont choisis
- le format audio cible est tranche
- la granularite de chunking est documentee
- l'enveloppe de cout local et de debit vise est explicite

**Tests attendus**

- revue technique du choix moteur
- echantillon de sortie audio valide

**Dependances**

- `EPIC-008`

### `NEWS-037` Abstraction Rust `TtsProvider` et orchestration serveur

**Objectif**  
Encapsuler le moteur TTS dans une couche Rust propre, observable et remplaçable.

**Scope**  
Creer l'abstraction `TtsProvider`, la gestion du cycle de vie du moteur, la file de traitement TTS, les timeouts, les erreurs, le warmup et les hooks de trace.

**Hors scope**  
Transport WebSocket audio.

**Approche technique**  
Le service Rust orchestre l'appel au moteur TTS, expose une interface stable au reste du pipeline, et isole les details du moteur pour conserver un coeur de service maitrisable.

**Criteres d'acceptation**

- la TTS est appelee par une abstraction Rust unique
- le warmup et les erreurs d'initialisation sont geres
- la TTS est observable et non bloquante pour le hot path
- le moteur peut etre remplace sans casser le contrat du service

**Tests attendus**

- test de cycle de vie du provider
- test de timeout et d'erreur moteur
- test de plusieurs syntheses consecutives

**Dependances**

- `NEWS-036`

### `NEWS-038` Generation audio en memoire et chunking streamable

**Objectif**  
Produire une sortie audio exploitable par un futur transport temps reel sans passer par des fichiers.

**Scope**  
Transformer `narration_text` en flux audio en memoire, definir les buffers intermediaires, le decoupage en chunks et le contrat technique remis a `EPIC-007`.

**Hors scope**  
Player frontend et contrat WebSocket final.

**Approche technique**  
Le provider TTS renvoie un flux ou un buffer progressif que le service Rust decoupe ensuite en chunks streamables, avec metadata de format, bornes memoire et signaux de fin.

**Criteres d'acceptation**

- une narration FR peut produire une sortie audio en memoire
- la sortie peut etre consommee en chunks sans ecriture disque
- la memoire reste bornee sur des textes courts du MVP
- le contrat technique remis au streaming audio est documente

**Tests attendus**

- test de synthese d'un item unique
- test de chunking et d'ordre de sortie
- verification du nettoyage des artefacts temporaires internes au provider MVP

**Dependances**

- `NEWS-037`

### `NEWS-039` Normalisation de narration, benchmark et observabilite TTS

**Objectif**  
Rendre la TTS robuste sur des news finance et exploitable en production.

**Scope**  
Normaliser la ponctuation et les symboles sensibles, gerer les cas courts ou ambigus, benchmarker la latence et le debit, et exposer les metriques utiles.

**Hors scope**  
Personnalisation avancee de la voix ou multi-langue.

**Approche technique**  
Introduire une preparation de `narration_text` orientee TTS, puis instrumenter la synthese: temps de warmup, temps de generation, taille de queue, taille des buffers, taux d'erreur et saturation machine.

**Criteres d'acceptation**

- la narration FR est normalisee pour une lecture audio intelligible
- un benchmark MVP TTS est documente
- les erreurs TTS sont tracees et correlees
- le cout local du service TTS est explicite

**Tests attendus**

- tests sur phrases finance representatives
- benchmark sur lot borne de narrations
- verification des logs et metriques TTS

**Dependances**

- `NEWS-038`

## Etat reel des tickets

- `NEWS-036` fait
- `NEWS-037` fait
- `NEWS-038` fait
- `NEWS-039` fait

## Cloture

`EPIC-009` est clos avec les livrables suivants:

- provider Rust `TtsProvider` livre et rendu remplacable
- generation WAV serveur et chunking streamable en memoire
- changement de voix a chaud via le runtime partage
- normalisation `narration_text` avant synthese
- observabilite runtime via logs structures et `/api/runtime/tts`
- benchmark local reproductible via `scripts/benchmark_tts_local.py`
- enveloppe MVP documentee: warmup `543.24 ms`, moyenne `538.17 ms`, p95 `696.0 ms`, debit `1.86 narrations/s` sur le lot de reference local du 23 avril 2026

Point de cadrage conserve:

- `EPIC-009` reste le socle TTS Rust du projet
- la migration produit vers `Piper` releve maintenant de `EPIC-012`
