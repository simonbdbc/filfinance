# TOPIC-001 - Faut-il ouvrir un bloc d'EPICs pour traduire localement et streamer l'audio FR des news ?

## Statut

Decision prise. Le sujet est promu en priorite absolue, mais il est maintenant precede par un EPIC transverse de commentaires Rust pedagogiques, puis decoupe en trois EPICs lies: traduction locale `OPUS-MT`, TTS serveur pilotee par Rust, puis streaming audio binaire via WebSocket.

## Etat reel au 23 avril 2026

La cible finale retenue n'est pas encore totalement livree.

Ce qui est deja en place:

- le service persiste et diffuse un flux texte EN+FR unifie
- Telegram et le WebSocket user-facing consomment le meme objet traduit
- un process de test E2E est declenchable depuis le client et suivi via un flux WebSocket de debug
- la traduction locale `OPUS-MT tc-big en-fr` est branchee en vrai dans le service
- une TTS serveur Rust est maintenant branchee via `Piper`
- le service expose maintenant `/ws/audio` pour pousser des chunks audio WAV en memoire
- un client autonome `filfinance` lit deja ce flux sur le web via `Web Audio API` et `AudioWorklet`
- le client charge l'historique journalier sans lecture automatique, puis ne lit automatiquement que les vraies news live nouvelles
- la file audio locale ne sert maintenant qu'a sequencer les news live arrivees pendant une lecture deja en cours
- le client peut maintenant rejouer les traductions audio live encore en attente apres reconnexion

Ce qui n'est pas encore livre:

- le bootstrap local des voix Piper et la validation live du provider nominal
- la validation navigateur/mobile de la couche audio cible

Le projet est donc aujourd'hui dans un etat intermediaire volontaire:

- la validation fonctionnelle EN+FR, E2E et audio binaire existe deja
- la couche audio finale demande encore du durcissement
- `EPIC-008` est maintenant clos avec benchmark local et observabilite runtime de la traduction
- `EPIC-009` est maintenant clos avec benchmark local et observabilite runtime TTS
- la cible produit active restante reste maintenant `EPIC-007`

## Question

Le service `filfinance-service` doit-il ajouter un bloc d'EPICs dedies a une traduction locale EN vers FR et a une lecture audio serveur des news, en s'appuyant sur `OPUS-MT tc-big en-fr`, une TTS pilotee par Rust et un streaming audio binaire sur WebSocket ?

## Decision retenue

Le choix est acte: ce sujet passe avant les autres developpements restants.

La cible finale doit toutefois rester cadre par les memes contraintes techniques. En pratique:

- traiter d'abord `EPIC-010`, puis le bloc `EPIC-008` puis `EPIC-009` puis `EPIC-007`
- limiter le MVP aux `headline` et exclure d'emblee les evenements `macro_calendar`
- produire localement la traduction EN vers FR via `OPUS-MT tc-big en-fr`
- produire la TTS cote serveur, pilotee par le service Rust
- pousser en temps reel des metadata EN+FR et un flux audio binaire via WebSocket
- ne pas generer, stocker ni servir de fichiers audio persistants
- conserver le texte source d'origine a cote de la traduction francaise pour limiter les risques d'interpretation
- deleguer au client uniquement le buffering et la lecture via `Web Audio API` et `AudioWorklet`
- considerer l'historique charge comme une base de dedup visuelle, pas comme une source d'autoplay
- ajouter un process de test E2E declenchable depuis le client Web, qui traverse exactement le meme pipeline que Telegram et le WebSocket user-facing
- reserver le suivi detaille de ce process de test a un flux WebSocket de debug dedie

## Pourquoi le sujet est pertinent

- le service possede deja un modele canonique `NewsEvent`, une persistence SQLite, un broadcast WebSocket et un downstream Telegram
- l'architecture actuelle accepte deja le principe de consommateurs secondaires non bloquants
- la traduction locale supprime le cout variable et la dependance a un quota externe de traduction
- une TTS serveur supprime la forte variabilite des voix navigateur
- un flux francais lu automatiquement cote client cree une vraie valeur produit pour un usage mobile, multitache ou pre-market
- le besoin reste suffisamment transverse pour justifier plusieurs EPICs, car il touche le contrat de sortie, la persistence du texte derive, la TTS, le WebSocket et le comportement du client

## Pourquoi ce n'est pas un simple ticket

Ce sujet n'est pas un "petit plus d'affichage".

Il ajoute au moins six dimensions distinctes:

- choix, packaging et cout du modele de traduction local
- politique de traduction et de fidelite du message
- orchestration asynchrone d'un pipeline de traduction supplementaire
- orchestration d'une TTS serveur
- formalisation d'un contrat WebSocket texte plus audio binaire
- gestion cote client d'un buffer audio et d'une lecture automatique

## Points de vigilance pendant l'EPIC

### 1. Droit de redistribution

Il faut verifier si les sources actuelles autorisent:

- la traduction du titre et du contenu
- la rediffusion du texte traduit au sein du produit

L'absence de fichiers audio persistants reduit le scope legal, mais ne supprime pas le besoin de clarifier le droit de traduction et de republication du texte derive.

### 2. Nature exacte du "flux audio"

Il faut choisir explicitement le modele produit:

- le serveur pousse du texte EN+FR de debug et un flux audio binaire
- le serveur lance la TTS a partir du texte FR derive
- le client bufferise les chunks audio en memoire et les joue
- le client ne lit pas l'historique au chargement; il ne lit automatiquement que les news live nouvelles
- la lecture est cote client, mais la voix et la synthese sont cote serveur

Pour ce MVP, c'est l'option la plus coherente avec la contrainte produit "pas de fichiers a telecharger" et avec la volonte d'avoir une voix homogene.

Point important de pilotage:

- la cible retenue reste ce modele serveur TTS plus audio binaire
- le slice actuellement livre dans le repo reste encore un MVP web a durcir surtout sur la validation multi-navigateurs et mobile

### 3. Exigence de fidelite

Une traduction libre peut introduire des contresens critiques sur des news de marche. Il faut donc arbitrer entre:

- traduction quasi litterale du titre et des champs connus
- breve narration normalisee en francais
- resume ou reformulation

La recommandation MVP est de rester tres proche du texte source, sans analyse ni editorialisation.

### 4. Contraintes navigateur et lecture

Le navigateur ne choisit plus la voix, mais il reste responsable du buffering et de la lecture.

Il faut accepter des variations sur:

- le support `Web Audio API`
- le comportement du buffer en cas de reseau instable
- la gestion des coupures et reconnexions WebSocket
- l'autoplay effectif selon le contexte de la page

Le MVP doit donc viser une compatibilite fonctionnelle et un buffer stable, pas un player multimedia complet.

### 5. Valeur reelle du contenu disponible

Aujourd'hui, certaines sources peuvent ne fournir qu'un titre et peu de corps de texte. Si la lecture audio se limite a lire un titre traduit, la valeur percue peut etre faible. Cet EPIC a plus de sens si:

- les titres portent deja une information exploitable
- ou le service dispose d'un texte additionnel exploitable et autorise

### 6. Cout et latence

Le choix local supprime les couts variables des services externes, mais deplace le cout sur la machine et l'orchestration. Il reste donc:

- le cout local de traduction `OPUS-MT`: RAM, CPU ou GPU, cold start
- le cout local de TTS: CPU ou GPU, warmup, debit simultane
- la latence bout en bout entre la news, la traduction, la TTS et le premier chunk audio
- la politique de drop, retry et backpressure en cas de burst

## Positionnement retenu dans la roadmap

La priorisation a ete tranchee:

- `EPIC-008`, `EPIC-009` puis `EPIC-007` passent avant les autres phases restantes
- `EPIC-010` passe avant ce bloc pour imposer la norme de commentaires Rust pedagogiques
- `NEWS-046` est fait et `NEWS-047` fixe deja l'ordre de couverture a appliquer ensuite au code Rust
- sa numerotation est historique et ne reflete pas son ordre d'execution
- les autres EPICs reprennent ensuite en s'alignant sur le contrat FR temps reel retenu et sur la regle documentaire Rust

La raison reste la meme: une news mal parsee, dupliquee, en retard ou mal normalisee devient encore plus visible une fois traduite et lue automatiquement cote client.

## Principes d'architecture recommandes

### 1. Ne rien changer au hot path d'ingestion

Le pipeline actuel doit continuer a faire foi:

`collecte -> dedup -> persistence -> broadcast`

La traduction, la TTS et la lecture audio doivent etre alimentees depuis des downstreams secondaires, comme Telegram aujourd'hui.

### 2. Isoler la traduction dans un worker dedie

Le modele cible est:

`broadcast interne ou lecture SQLite -> queue traduction -> traduction FR -> persistence texte derive`

Ce worker doit avoir:

- sa propre queue bornee
- sa propre politique de retry
- sa propre politique de drop en cas de burst
- des logs et metriques distincts

### 3. Isoler la TTS dans un worker dedie

Le modele cible est:

`traduction FR persistente -> queue TTS -> generation audio en memoire -> flux streamable`

Ce worker doit avoir:

- sa propre queue bornee
- sa propre politique de timeout
- sa propre politique de drop en cas de surcharge
- des logs et metriques distincts

### 4. Stocker le texte derive a part

Il vaut mieux introduire une table dediee de type `news_translations` plutot que d'alourdir `news_events`.

Cette table pourrait porter au minimum:

- `news_event_id`
- `language`
- `status`
- `published_at`
- `translated_title`
- `translated_body`
- `narration_text`
- `error_message`
- `created_at`
- `updated_at`

### 5. Garder la trace du texte source

Le client doit pouvoir rattacher la lecture a l'evenement source pour afficher:

- titre original
- traduction francaise
- source
- horodatage
- lien vers la news

### 6. Exposer un contrat WebSocket texte plus audio

Le MVP ne doit pas inventer un produit "radio live continu". Il peut exposer:

- un flux texte user-facing EN+FR pour l'affichage et le debug fonctionnel
- un flux WebSocket audio binaire pour le transport des chunks TTS
- un flux WebSocket de debug pour les traces techniques
- eventuellement un endpoint HTTP de debug pour relire les derniers items traduits

### 7. Laisser le client seulement bufferiser et jouer

Le client doit gerer:

- un buffer local des chunks audio
- une file locale des items a jouer
- `Web Audio API`
- un `AudioWorklet`
- le passage automatique au message suivant a la fin de la lecture
- un fallback visuel si le flux audio ne peut pas etre lu correctement

## Scope retenu

- headlines traduites en francais
- generation d'un texte de narration court et stable
- persistence minimale des variantes texte francaises
- traduction locale via `OPUS-MT tc-big en-fr`
- TTS cote serveur pilotee par Rust
- emission WebSocket temps reel du texte EN+FR et du flux audio binaire
- lecture automatique cote client via `Web Audio API` et `AudioWorklet`
- observabilite minimale des erreurs de traduction, TTS et streaming

## Hors scope retenu

- stockage ou diffusion de fichiers audio persistants
- radio live continue
- WebRTC, HLS ou Icecast
- traduction multilingue
- lecture des evenements `macro_calendar`
- personnalisation avancee des voix
- auth utilisateur et gestion d'abonnements
- resume analytique ou commentaire de marche

## Sortie de cadrage

Le sujet n'est plus une proposition theorique. Il est maintenant porte par:

- la roadmap officielle dans [ROADMAP.md](./ROADMAP.md)
- l'EPIC de traduction locale [EPIC-008](./epics/EPIC-008-traduction-locale-opus-mt-local.md)
- l'EPIC TTS serveur [EPIC-009](./epics/EPIC-009-tts-serveur-rust.md)
- l'EPIC de streaming audio [EPIC-007](./epics/EPIC-007-streaming-audio-fr-temps-reel-websocket.md)

Le detail de reference des tickets est donc maintenant:

- `NEWS-032` a `NEWS-035` pour `OPUS-MT tc-big en-fr`
- `NEWS-036` a `NEWS-039` pour la TTS serveur pilotee par Rust
- `NEWS-040` a `NEWS-045` pour le streaming audio WebSocket et le process de test E2E

En lecture d'avancement:

- `EPIC-008` est maintenant clos
- `EPIC-009` est clos comme socle TTS Rust, et `EPIC-012` porte maintenant la migration produit vers `Piper`
- `EPIC-007` est deja avance sur le flux texte, le debug, `/ws/audio` et le client web
