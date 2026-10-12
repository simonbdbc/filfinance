# EPIC-007 - Streaming audio FR temps reel en memoire via WebSocket

## Objectif

Diffuser, a partir des news deja traduites en francais, un flux audio serveur streamable en memoire vers les clients, sans fichiers temporaires ni artefacts audio persistants.

## Valeur livree

Un client Web ou mobile peut ecouter automatiquement les dernieres news traduites, avec une voix homogene et un transport temps reel sans telechargement ni gestion locale de clips.

## Statut

En cours avance au 23 avril 2026.

## Contexte courant et dependances

Cet EPIC reste le but produit prioritaire du projet, mais il s'execute apres deux prerequis directs:

- `EPIC-008` pour produire localement le texte FR via `OPUS-MT tc-big en-fr`
- `EPIC-009` pour produire un flux TTS cote serveur pilote par Rust
- `EPIC-012` pour brancher `Piper` comme provider TTS nominal du service

Il s'appuie sur le socle deja visible dans le repo: runtime `tokio`, API `axum`, persistence SQLite, broadcast WebSocket, pipeline d'ingestion et downstream Telegram.

## Etat reel au 23 avril 2026

Ce qui est deja livre:

- contrat texte EN+FR stable sur `/ws/news`
- replay des traductions recentes, heartbeat et gestion du lag
- flux de debug `/ws/test-runs`
- declenchement d'un run de test E2E depuis le client
- injection du test dans le pipeline canonique du service
- traces de run corrigees par `test_run_id`
- client autonome `filfinance` avec affichage EN+FR, file locale, historique et lecture web via `Web Audio API`
- contrat `audio_start` / chunks binaires / `audio_end` en place sur `/ws/audio`
- worker TTS serveur branche et diffuse le flux audio binaire en memoire
- moteur `AudioWorklet` branche dans le client web avec fallback `BufferSource`
- etats UI audio enrichis: voix active, moteur de lecture et taille du buffer courant
- rejet de l'autoplay pour l'historique charge au demarrage: seule une vraie news live nouvelle peut entrer en lecture automatique
- rejeu cible des seules traductions audio live encore en attente lors d'une reconnexion du client
- historique journalier EN+FR separe entre feed reel et `test_run`, avec rejeu audio cote client
- invariant FIFO audio live durci cote client: admission monotone par timestamp, replay item-aware et file binaire triee

Ce qui n'est pas encore livre:

- tests navigateur et validation mobile de la couche audio cible

Lecture d'avancement des tickets:

- `NEWS-043` largement livre
- `NEWS-044` largement livre sur la partie texte et Telegram
- `NEWS-045` largement livre sur la trace texte/debug
- `NEWS-040` et `NEWS-041` sont largement livres en MVP
- `NEWS-042` est maintenant largement livre avec `Web Audio API`, `AudioWorklet`, resynchronisation au reconnect et replay audio depuis l'historique journalier
- `NEWS-079`, `NEWS-080` et `NEWS-081` sont livres cote client; il reste un smoke navigateur/mobile a faire

## Definition of Done

- le service emet un contrat texte EN+FR stable pour les metadata user-facing et le debug
- le service ouvre un flux WebSocket audio binaire capable de pousser des chunks audio en memoire sans fichiers temporaires
- un item derive du meme pipeline que Telegram peut declencher une generation TTS puis un streaming audio cote client
- un client peut bufferiser les chunks en memoire, jouer un item a la fois, puis enchainer automatiquement les items suivants
- la file audio live est monotone: une news plus ancienne que l'item en lecture, ou plus ancienne que le dernier item recu quand rien ne joue, n'entre jamais dans la file audio
- l'historique visible reste exhaustif cote client et trie du plus recent au plus ancien, meme quand une news est rejetee de la file audio
- les erreurs de streaming audio, de TTS ou de buffer ne bloquent ni ingestion, ni persistence, ni Telegram
- un bouton de test cote client peut declencher un run E2E qui traverse exactement le meme pipeline que la production
- un flux WebSocket de trace permet de suivre chaque etape du run de test, y compris l'etape TTS et l'emission audio

## Risques et points durs

- instabilite du transport WebSocket binaire en cas de client lent ou de reseau mobile fluctuant
- croissance memoire si le client ou le serveur bufferise plus vite qu'il ne lit
- choix du format audio et du niveau de chunking pour limiter la latence sans casser la lecture
- complexite du player Web Audio API et de l'`AudioWorklet`
- risque de faux positif si le process de test contourne la TTS ou le transport audio reel
- pression CPU/GPU si plusieurs clients demandent des flux simultanes

## Liste des tickets

- `NEWS-040` Contrat de streaming audio et correlation avec le flux texte
- `NEWS-041` Streaming audio binaire en memoire cote backend
- `NEWS-042` Client Web Audio API et AudioWorklet
- `NEWS-043` Declenchement E2E de test depuis le client web
- `NEWS-044` Injection backend de test via le pipeline canonique
- `NEWS-045` Trace d'execution EN+FR, TTS et audio via WebSocket debug
- `NEWS-079` Contrat de file audio monotone par timestamp
- `NEWS-080` Admission client FIFO et watermark audio live
- `NEWS-081` Replays, clips binaires et tests de non-regression FIFO

## Details des tickets

### `NEWS-040` Contrat de streaming audio et correlation avec le flux texte

**Objectif**  
Definir le contrat WebSocket global qui relie metadata EN+FR, controle de session audio et chunks binaires.

**Scope**  
Formaliser les messages de controle `audio_start`, `audio_chunk`, `audio_end`, `audio_error`, leur correlation avec `news_event_id` et `translation_id`, ainsi que la coexistence avec le flux texte EN+FR et le flux de debug.

**Hors scope**  
Implementation du moteur TTS ou du player frontend.

**Approche technique**  
Conserver un flux texte user-facing pour la visibilite EN+FR, et ajouter un transport audio binaire dedie ou multiplexe sur WebSocket avec correlation stricte, heartbeat, politique de backpressure et semantique explicite de fermeture/reprise.

**Criteres d'acceptation**

- le contrat texte EN+FR et le contrat audio binaire sont documentes
- chaque session audio peut etre rattachee sans ambiguite a l'evenement source et a sa traduction
- la politique de lag, skip, close et retry est explicite
- la separation entre flux user-facing et flux debug est claire

**Tests attendus**

- revue technique du contrat
- test de serialisation des messages de controle

**Dependances**

- `EPIC-008`
- `EPIC-009`

### `NEWS-041` Streaming audio binaire en memoire cote backend

**Objectif**  
Streamer l'audio genere cote serveur directement depuis la memoire vers les clients sans creation de fichiers audio.

**Scope**  
Construire le worker qui recoit le texte FR pret a lire, lance le TTS, decoupe la sortie en chunks streamables, pousse ces chunks sur WebSocket, et applique une politique explicite de queue, timeout et drop.

**Hors scope**  
Implementation du player frontend.

**Approche technique**  
Reutiliser `tokio`, `axum` et un canal borne dedie au hot path audio. Le worker TTS ne doit pas bloquer l'ingestion. L'audio est produit et pousse en memoire, avec envelope de controle, correlation stricte et nettoyage immediat des buffers en fin de stream.

**Criteres d'acceptation**

- un evenement traduit eligible peut declencher un stream audio sans fichier temporaire
- un client lent ou deconnecte ne bloque pas les autres consommateurs ni l'ingestion
- les erreurs TTS et transport sont visibles, correlees et non bloquantes
- la memoire allouee par stream reste bornee et liberee en fin de traitement

**Tests attendus**

- test de stream audio sur un item unique
- test de coupure client en cours de stream
- verification de la liberation memoire et de la politique de drop

**Dependances**

- `NEWS-040`
- `EPIC-009`

### `NEWS-042` Client Web Audio API et AudioWorklet

**Objectif**  
Lire le flux audio binaire cote client de maniere stable et automatique, sans fichiers locaux.

**Scope**  
Mettre en place le buffer audio en memoire, l'`AudioWorklet`, la file de lecture, l'enchainement automatique clip par clip, la correlation avec le flux EN+FR, et les etats UI minimaux de connexion/lecture.

**Hors scope**  
Lecteur offline, mixage avance, ou effets audio.

**Approche technique**  
Utiliser `Web Audio API` et un `AudioWorklet` pour lisser le buffer de chunks recus sur WebSocket. Le client charge d'abord l'historique journalier sans lecture automatique, puis n'autorise l'autoplay que pour une vraie news live qui n'etait pas deja connue localement. La file d'items audio ne sert qu'a sequencer les news live arrivees pendant qu'une lecture est deja en cours, et le client recale son etat sur les messages de controle `audio_start` et `audio_end`.

**Criteres d'acceptation**

- le client peut lire un flux audio binaire sans telechargement de fichier
- le chargement de l'historique ne declenche aucune lecture automatique
- un item deja present dans l'historique local n'est jamais relu automatiquement
- un seul item est lu a la fois
- un item live suivant peut etre mis en file sans casser l'item en cours
- un item live plus ancien que la reference audio courante est visible dans l'historique mais rejete de la file audio
- les etats de connexion, buffer et erreur sont visibles et utiles au debug

**Tests attendus**

- test manuel sur navigateur Chromium
- verification de l'enchainement de plusieurs items consecutifs
- verification du comportement en cas de fermeture/reconnexion WebSocket

**Dependances**

- `NEWS-040`
- `NEWS-041`

### `NEWS-079` Contrat de file audio monotone par timestamp

**Objectif**  
Formaliser la regle produit de la file audio live pour eviter qu'une news arrivee tardivement perturbe la lecture temps reel.

**Scope**  
Definir le timestamp metier utilise par le client audio, les conditions d'admission dans la file, le tri FIFO et la separation stricte entre file audio et historique visible.

**Regle produit**

- le timestamp audio de reference est `item.timestamp || item.created_at`
- si un item est en lecture, toute news dont le timestamp est anterieur a celui de l'item en lecture est rejetee de la file audio
- si aucun item n'est en lecture, toute news dont le timestamp est anterieur au dernier item recu est rejetee de la file audio
- si le timestamp est posterieur ou egal a la reference audio, l'item peut entrer dans la file audio
- les items admis dans la file audio sont tries du plus ancien au plus recent
- les egalites de timestamp sont stabilisees par `id` croissant
- toutes les news, y compris celles rejetees de la file audio, restent ajoutees a l'historique visible et triees du plus recent au plus ancien

**Hors scope**  
Changer l'ordre de l'historique visible, filtrer les news cote serveur, ou rejouer automatiquement l'historique charge au demarrage.

**Approche technique**  
Documenter un comparateur commun `(timestamp, id)`, une watermark audio live et une fonction d'admission pure reutilisable par la file texte, les replays et la file binaire.

**Criteres d'acceptation**

- le contrat d'admission audio est documente
- le contrat precise explicitement que l'historique visible n'est jamais filtre par cette regle
- les cas "item en lecture" et "aucun item en lecture" sont differencies

**Tests attendus**

- tests unitaires du comparateur timestamp/id
- tests unitaires de la fonction d'admission audio

**Dependances**

- `NEWS-042`

### `NEWS-080` Admission client FIFO et watermark audio live

**Objectif**  
Empecher l'entree dans la file audio de toute news plus ancienne que la reference audio courante.

**Scope**  
Ajouter dans le client une watermark audio monotone, appliquer la regle avant `markAwaitingAudio`, avant `setQueue`, et trier la file admise du plus ancien au plus recent.

**Hors scope**  
Modifier la persistence backend ou masquer les news de l'historique.

**Approche technique**  
Introduire un helper client pur, par exemple `shouldAdmitAudioItem(candidate, currentItem, lastReceivedItem)`, base sur `item.timestamp || item.created_at`. Quand `currentItem` existe, il est la reference. Sinon, `lastItem` est la reference. La file est inseree via le comparateur FIFO et tronquee sans inverser l'ordre.

**Criteres d'acceptation**

- une news plus ancienne que l'item en lecture ne declenche ni `markAwaitingAudio`, ni insertion dans `queue`
- une news plus ancienne que le dernier item recu, quand rien ne joue, ne declenche ni attente audio ni insertion dans `queue`
- une news posterieure est inseree dans la file audio en ordre timestamp ascendant
- l'historique `history`, `dailyFeedHistory` et `dailyTestRunHistory` continue a recevoir toutes les news et reste trie du plus recent au plus ancien

**Tests attendus**

- test unitaire "current item 10:05, candidate 10:03 => rejet audio"
- test unitaire "pas de lecture, dernier recu 10:05, candidate 10:03 => rejet audio"
- test unitaire "candidate 10:06 => admission et tri FIFO"

**Dependances**

- `NEWS-079`

### `NEWS-081` Replays, clips binaires et tests de non-regression FIFO

**Objectif**  
Garantir que les replays audio et la file de clips binaires respectent le meme ordre FIFO par timestamp que la file live.

**Scope**  
Construire les demandes de replay a partir des `TranslatedNewsItem` quand le timestamp est necessaire, filtrer les items plus anciens via la watermark audio, puis convertir en IDs seulement apres tri. Inserer les clips binaires decoded dans une file triee par timestamp de leur item associe au lieu d'un simple append par arrivee reseau.

**Hors scope**  
Reordonner le serveur par lui-meme; le serveur peut continuer a respecter l'ordre demande par le client.

**Approche technique**  
Remplacer les chemins qui manipulent uniquement des `translation_id` par un chemin item-aware quand l'ordre audio compte. `resolveItemForPlayback` devient obligatoire avant admission d'un clip binaire non deja connu. Un clip dont l'item associe est plus ancien que la reference audio est ignore pour la lecture, mais la news reste disponible dans l'historique.

**Criteres d'acceptation**

- `pendingAudioReplayIds` est derive d'items tries du plus ancien au plus recent
- le flux `/ws/audio` recoit les IDs dans l'ordre FIFO attendu
- `queuedBinaryClipsRef` reste triee par timestamp metier, pas par ordre de decodage
- un clip binaire plus ancien que l'item en lecture ou que le dernier item recu n'est pas lu

**Tests attendus**

- test unitaire de construction d'IDs de replay depuis des items hors ordre
- test unitaire d'insertion de clips binaires hors ordre
- smoke manuel avec deux news live arrivees dans le desordre

**Dependances**

- `NEWS-080`

### `NEWS-043` Declenchement E2E de test depuis le client web

**Objectif**  
Permettre a un developpeur ou a un operateur de lancer un scenario complet de verification depuis le client, sans attendre une vraie news.

**Scope**  
Ajouter un bouton de test dans le client, transmettre un signal explicite au backend, afficher l'etat du run en cours, et eviter les doubles declenchements accidentels.

**Hors scope**  
Outil d'administration complet ou orchestration multi-utilisateur.

**Approche technique**  
Le client envoie une commande de test sur le flux de debug ou sur un point d'entree dedie, puis suit le run via le flux de trace et verifie la reception de l'item texte et du flux audio correle.

**Criteres d'acceptation**

- un utilisateur peut lancer un test manuellement depuis le client
- l'UI affiche l'identifiant de run et les etats principaux du scenario
- un double clic ou un spam utilisateur ne declenche pas une explosion de runs concurrents
- le run de test peut etre relie au flux texte et au flux audio recus

**Tests attendus**

- test manuel du bouton depuis le client
- verification de la correlation `test_run_id`
- verification du comportement si un run est deja en cours

**Dependances**

- `NEWS-042`

### `NEWS-044` Injection backend de test via le pipeline canonique

**Objectif**  
Garantir qu'un test valide exactement le meme chemin d'execution que la production, jusqu'au flux audio binaire.

**Scope**  
Introduire un evenement synthetique de test injecte en amont du pipeline canonique, avec persistence, traduction, diffusion Telegram, diffusion texte user-facing et diffusion audio via les memes workers que les vraies news.

**Hors scope**  
Bypass direct du TTS, de Telegram ou du transport WebSocket audio.

**Approche technique**  
Le backend cree un `NewsEvent` synthetique balise comme test, l'injecte au meme niveau que les evenements reels, puis laisse le pipeline normal faire persistence, traduction locale, diffusion Telegram, diffusion texte EN+FR et streaming audio.

**Criteres d'acceptation**

- le run de test ne contourne aucun worker critique
- Telegram recoit la notification de test via le meme downstream que la production
- le flux texte user-facing et le flux audio recoivent tous deux l'item de test via le meme pipeline
- le payload de test transporte l'anglais source, le francais derive et les identifiants de correlation utiles au debug

**Tests attendus**

- test d'injection d'un evenement synthetique jusqu'a la persistence
- verification que Telegram, le flux texte et le flux audio recoivent l'item de test
- verification qu'une erreur downstream reste visible sans casser le run complet

**Dependances**

- `EPIC-008`
- `EPIC-009`
- `NEWS-040`

### `NEWS-045` Trace d'execution EN+FR, TTS et audio via WebSocket debug

**Objectif**  
Donner une visibilite complete sur chaque etape d'un run de test pour localiser precisement un blocage.

**Scope**  
Definir un flux WebSocket de debug reserve au suivi des runs de test, avec journal d'etapes structurees: reception du signal, creation de l'evenement de test, persistence, traduction locale, preparation TTS, debut de stream audio, fin de stream, tentative Telegram, succes ou echec final.

**Hors scope**  
Collecte exhaustive de logs applicatifs generiques hors runs de test.

**Approche technique**  
Emettre des messages structures `test_trace` correles par `test_run_id`, `news_event_id` et `stream_id`, avec etape, statut, horodatage, details minimaux et cause d'echec exploitable.

**Criteres d'acceptation**

- chaque etape critique du run est visible dans le flux de debug
- un blocage de traduction, TTS, Telegram ou streaming audio est identifiable sans inspection serveur supplementaire
- le flux de debug ne pollue pas le flux user-facing
- les traces permettent de confirmer que Telegram et le WebSocket ont bien suivi le meme pipeline

**Tests attendus**

- test de trace complete sur un run nominal
- verification des traces d'erreur sur echec de traduction, TTS ou transport audio
- verification de la correlation entre flux debug, flux texte et flux audio

**Dependances**

- `NEWS-044`
