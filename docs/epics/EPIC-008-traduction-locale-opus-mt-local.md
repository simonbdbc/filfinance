# EPIC-008 - Traduction locale EN vers FR via OPUS-MT local

## Objectif

Produire localement un texte francais stable, rapide et predictible a partir des news anglaises, en s'appuyant sur un checkpoint `OPUS-MT` local, sans dependre d'un service de traduction distant. Dans l'etat courant du projet, le checkpoint retenu est `OPUS-MT tc-big en-fr`.

## Valeur livree

Le service derive un flux EN+FR coherent sans cout variable par caractere, sans quota externe de traduction, et avec un niveau de controle plus fort sur la latence et la disponibilite.

## Statut

Fait au 23 avril 2026.

Extension Docker/qualite validee le 3 mai 2026.

## Contexte courant et dependances

Cet EPIC ouvre le nouveau bloc prioritaire du projet. Il alimente ensuite `EPIC-009` pour la TTS et `EPIC-007` pour le streaming audio. Il reutilise le socle deja present: runtime `tokio`, persistence SQLite, pipeline d'ingestion et downstream Telegram.

## Etat reel au 23 avril 2026

Ce qui est deja livre dans le code:

- abstraction de provider de traduction cote service
- modes `heuristic_fr`, `local_command` et `local_command_session`
- branchement reel `OPUS-MT` via helper Python `CTranslate2`
- mode session long-vivant avec warmup
- possibilite de changer de checkpoint local sans toucher le backend Rust, via la commande locale
- workspace local actuellement bascule sur `OPUS-MT tc-big en-fr` avec `beam-size=4`
- protection des tokens finance sensibles
- persistence `news_translations`
- diffusion du meme item derive EN+FR vers Telegram et `/ws/news`
- helper local, requirements et smoke test documentes
- benchmark local documente et scriptable
- endpoint runtime `/api/runtime/translation` pour l'observabilite de la traduction
- enveloppe de cout MVP documentee avec chiffres reels du workspace
- garde-fous finance verifies par tests sur tokens sensibles

Extensions planifiees apres validation live Docker:

- `NEWS-126`: runtime Docker OPUS-MT pour remplacer `heuristic_fr` en deploiement serveur
- `NEWS-129`: profils traduction `fast`, `balanced`, `quality`

## Definition of Done

- un runtime local `OPUS-MT tc-big en-fr` est integre et charge proprement par le service
- le texte source EN reste la source de verite et la traduction FR est persistee a part
- les memes items traduits alimentent Telegram, le flux texte WebSocket et la future TTS
- les chiffres, tickers, devises, pourcentages, heures et symboles sensibles sont preserves
- les erreurs de traduction sont visibles et non bloquantes
- l'enveloppe de cout local est documentee: RAM, CPU ou GPU, cold start et throughput

## Risques et points durs

- taille du modele et temps de chargement a froid
- pression CPU ou GPU si le volume augmente
- preservation imparfaite de la terminologie finance sans garde-fous metier
- packaging du modele et strategie de distribution locale
- faux sentiment de gratuit si le cout local en machine n'est pas explicite

## Liste des tickets

- `NEWS-032` Cadrage modele, licence, packaging et enveloppe de cout
- `NEWS-033` Runtime local OPUS-MT et cycle de vie du modele
- `NEWS-034` Worker de traduction FR et persistence des variantes texte
- `NEWS-035` Garde-fous finance, benchmark et observabilite
- `NEWS-126` Runtime Docker OPUS-MT traduction
- `NEWS-129` Profils traduction OPUS-MT

Etat reel des tickets:

- `NEWS-032` fait
- `NEWS-033` fait
- `NEWS-034` fait
- `NEWS-035` fait
- `NEWS-126` a faire
- `NEWS-129` a faire

## Details des tickets

### `NEWS-032` Cadrage modele, licence, packaging et enveloppe de cout

**Objectif**  
Arbitrer le checkpoint `OPUS-MT` retenu, son mode de distribution et son cout d'exploitation local.

**Scope**  
Comparer le ou les checkpoints `en-fr` retenus, documenter la licence, definir le mode de livraison du modele, le plan de mise a jour et l'enveloppe machine cible du MVP.

**Hors scope**  
Implementation du worker de traduction.

**Approche technique**  
Valider un modele de traduction local `en-fr`, documenter le mode d'execution retenu par le service Rust, les besoins disque et memoire, ainsi que la strategie de versionning du modele.

**Criteres d'acceptation**

- le checkpoint `OPUS-MT tc-big en-fr` retenu est documente
- la licence et les contraintes de redistribution sont explicites
- la strategie de packaging, cache ou preload est tranchee
- une enveloppe de cout local cible est fixee

**Tests attendus**

- revue technique du choix de modele
- validation documentaire du packaging et de la licence

**Dependances**

- aucune

### `NEWS-033` Runtime local OPUS-MT et cycle de vie du modele

**Objectif**  
Integrer le runtime local qui execute `OPUS-MT tc-big en-fr` de maniere stable.

**Scope**  
Creer l'abstraction de provider de traduction locale, le chargement du modele, le warmup, la gestion de la concurrence, la reutilisation du modele en memoire et la fermeture propre.

**Hors scope**  
Persistence des traductions et integration TTS.

**Approche technique**  
Piloter un runtime local de traduction adapte a `OPUS-MT`, avec file de traitement bornee, warmup au demarrage ou a la demande, et isolation suffisante pour proteger le hot path d'ingestion.

**Criteres d'acceptation**

- le modele peut etre charge et reutilise sans reload a chaque requete
- un warmup explicite est possible
- les erreurs de chargement ou d'inference sont visibles et degradent proprement
- la concurrence ne provoque pas une explosion de memoire ou de latence

**Tests attendus**

- test de chargement a froid
- test de warmup
- test de plusieurs traductions consecutives avec le meme runtime

**Dependances**

- `NEWS-032`

### `NEWS-034` Worker de traduction FR et persistence des variantes texte

**Objectif**  
Faire entrer la traduction locale dans le pipeline canonique du service.

**Scope**  
Selection des evenements eligibles, worker asynchrone, persistence `news_translations`, idempotence, statuts, fallback et exposition du texte derive aux downstreams.

**Hors scope**  
Generation audio et streaming audio.

**Approche technique**  
Brancher un worker secondaire sur le flux persiste ou diffuse, puis persister `translated_title`, `translated_body` et `narration_text` dans une table dediee par `news_event_id + language`.

**Criteres d'acceptation**

- un evenement eligible cree une variante FR persistente
- un doublon ne regenere pas indefiniment la meme traduction
- Telegram et le flux texte WebSocket consomment le meme objet derive
- une erreur de traduction reste visible sans bloquer l'ingestion

**Tests attendus**

- creation d'une variante FR a partir d'une headline
- idempotence sur un meme `news_event_id`
- echec local avec statut d'erreur persiste

**Dependances**

- `NEWS-033`

### `NEWS-035` Garde-fous finance, benchmark et observabilite

**Objectif**  
Rendre la traduction locale exploitable sur des news finance reelles.

**Scope**  
Ajouter les regles de preservation des entites sensibles, mesurer la latence et le throughput, documenter le cout machine et exposer les metriques utiles au debug.

**Hors scope**  
Fine-tuning complet du modele.

**Approche technique**  
Encadrer le modele par des pre et post-traitements metier, puis instrumenter la traduction: temps de chargement, temps par item, taille de queue, taux d'erreur, saturation CPU ou GPU.

**Criteres d'acceptation**

- les tickers, chiffres et pourcentages critiques sont preserves sur les cas cibles
- un benchmark MVP est documente sur un lot de headlines
- les metriques essentielles de traduction sont exposees
- le cout local de service est explicite et relie au debit vise

**Tests attendus**

- tests sur fixtures finance
- benchmark sur lot borne de headlines
- verification des logs et metriques d'erreur

**Dependances**

- `NEWS-034`

### [`NEWS-126` Runtime Docker OPUS-MT traduction](../tickets/NEWS-126-runtime-docker-opus-mt.md)

**Objectif**
Retrouver en Docker la traduction OPUS-MT reelle au lieu du fallback `heuristic_fr`.

**Scope**
 Runtime Python Docker, dependances `ctranslate2`, `sentencepiece` et `huggingface_hub`, installation du modele CTranslate2 pendant le build, helper OPUS-MT, chemins modele conteneur, smoke post-deploy.

**Criteres d'acceptation**

- le serveur Docker utilise `NEWS_TRANSLATION_PROVIDER=opus_mt_local_session`
- le warmup passe
- le run E2E produit une vraie phrase FR

**Dependances**

- `NEWS-035`
- `NEWS-125`

### [`NEWS-129` Profils traduction OPUS-MT](../tickets/NEWS-129-profils-traduction-opus-mt.md)

**Objectif**
Arbitrer qualite et latence avec des profils explicites.

**Scope**
Profils `fast`, `balanced`, `quality`, mapping `beam-size`, exposition runtime et benchmark.

**Criteres d'acceptation**

- `quality` reproduit `beam-size=4`
- `balanced` teste `beam-size=2`
- `fast` teste `beam-size=1`

**Dependances**

- `NEWS-126`
- `NEWS-127`

## Cloture

`EPIC-008` est clos avec les livrables suivants:

- provider local `OPUS-MT tc-big en-fr` branche en mode session
- fallback heuristique non bloquant
- persistence et diffusion EN+FR unifiees
- protection des tokens finance sensibles
- observabilite runtime via logs structures et `/api/runtime/translation`
- benchmark local reproductible via `scripts/benchmark_opus_mt_local.py`
- enveloppe MVP documentee: modele `450M`, warmup `789.15 ms`, moyenne `416.5 ms`, p95 `612.95 ms`, debit `2.4 headlines/s` sur le lot de reference local du 23 avril 2026
