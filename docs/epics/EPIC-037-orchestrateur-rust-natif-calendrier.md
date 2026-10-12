# EPIC-037 — Orchestrateur Rust natif du calendrier

## Objectif

Remplacer le chemin calendrier n8n par un orchestrateur Rust natif, durable et
performant, sans interrompre le pipeline live news ni modifier les contrats
publics, admin, WebSocket ou HLS existants.

n8n décrit le processus historique à calquer — collecte, normalisation,
qualification, persistance, résumé et audio — mais ses résultats ne sont pas une
référence de validité métier. La migration est validée par le comportement Rust
déterministe et son test de récupération, non par une parité de contenu n8n.

## Correctif — sources fermees et diagnostics OpenRouter, 7 septembre 2026

Livraison suivante confirmée : `dots-studio/dots-3-note-preview:free`, endpoint
AtlasCloud gratuit annonçant JSON Schema strict. L'exception `json_object` est
supprimée, sans fallback ni changement des validations métier ou du résumé.
Le migrateur CI remplace MiniMax/Nemotron, préserve les modèles custom et DEV,
et reste idempotent. Aucune nouvelle admission n'est incluse dans cette release.
Validation locale : 370 tests Rust passants, trois ignorés, test Dots strict et
récupération native ciblés, fmt, 13 tests Node et tests shell du migrateur verts.

Audit PROD du run `calendar-day-ingestion-2026-09-07-1788803768018434267` :
admission unique CI `16352353222`, 17:56:08Z → 17:56:11Z, HTTP/code fournisseur
404 « This model is unavailable for free ». ForexFactory et Eurostat couvrants,
huit flux audités, zéro item/résumé/audio/alerte, un seul diagnostic terminal,
aucun travail actif. Smoke `16352364629` valide admin/WebSocket/HLS ; live actif.
La disponibilité catalogue n'est pas une garantie de succès d'une génération.

Historique de la release #363 (`1972d5a`) : passer à `minimax/minimax-m3:free` et conserver
code numérique, message et corps d'erreur OpenRouter dans les logs serveur.
La journalisation couvre les statuts HTTP non réussis et les enveloppes rejetées,
avec limites 64 Kio/corps et 4 Kio/message avant échappement, suppression des
credentials reconnus et absence de contenu détaillé dans les artefacts produit.
Les budgets de retry, concurrence, timeout et validations métier restent inchangés.
Le support JSON Schema du modèle demandé n'est pas annoncé : son adoption en mode
`json_object` a été explicitement confirmée le 7 septembre. Cette exception ne
concerne que ce modèle exact ; les autres conservent `json_schema` strict, sans
fallback de modèle ou de contrat. La migration CI du modèle est préparée,
idempotente, ne touche pas la clé API et préserve DEV ainsi que les modèles custom.
Validation locale de cette préparation : 370 tests Rust passants, zéro échec,
trois ignorés, récupération native ciblée et contrat JSON MiniMax testé,
fmt, 13 tests Node et tests shell du migrateur verts. Le test des logs couvre
les vraies réponses HTTP simulées 200/400/429/502, le code/message/corps, le
masquage et l'échappement des retours ligne, et l'absence de log des succès.
Livraison prévue exclusivement par GitLab CI après commit/push sur `main`.
Aucune nouvelle admission calendrier autorisée par ce changement de modèle.

La release #355 (`1b670c4`) a livre la couverture ForexFactory obligatoire et le
schema strict. La release #359 (`8aaeb24`) supprime les variantes HTML et libres du
contrat, leurs liens admin/PWA et leur acceptation dans le script d'observation.
Les donnees SQLite existantes ne sont pas purgees ; seules les provenances
autorisees sont reprises lors d'une reingestion. Validation de cet arbre :
364 tests Rust passants, trois ignores, recuperation native et fmt valides,
neuf tests Node d'observation et typecheck PWA valides.

ForexFactory export JSON doit etre `Covered` pour le jour Paris demande avant
tout appel IA. Les sept autres flux sont toujours examines mais facultatifs ;
leurs evenements complementaires restent autorises si ForexFactory couvre le
jour. Un HTTP 200 sans couverture ne suffit pas. L'absence, le refus ou un
document invalide produit `calendar_native_required_source_unavailable`, sans
nouvelle donnee produit et sans effacer l'historique. La semaine suivante peut
donc rester indisponible avec l'export limite a la semaine courante.

Les rejets d'ingestion disposent d'un diagnostic type (motif, index, champ,
fin de generation, et code HTTP OpenRouter lorsqu'il existe), atomique avec le dead-letter et protege par lease/fencing.
L'artefact `calendar_native_ingestion_diagnostic` utilise les surfaces admin et
WebSocket existantes, sans texte source, prompt ou reponse IA. Le client
ingestion exige une fin `stop` et un JSON Schema strict avec
`provider.require_parameters=true`. Aucun fallback modele/format ni retry
supplementaire. Le chemin resume et les validations metier restent inchanges.

La sonde isolee ignoree par defaut et sa procedure figurent dans le
[runbook calendrier](../CALENDAR-NATIVE-SHADOW-RUNBOOK.md). Une nouvelle
completion ne prouve pas la cause du rejet historique du 6 septembre.
L'artefact historique `open_router_rejected` ne permet pas d'identifier sa cause :
l'ancienne implementation regroupait les statuts HTTP permanents, une reponse
invalide, une completion vide et les limites de taille. Le correctif les
distingue sans conserver de corps, prompt, URL ni detail fournisseur.

Audit reel #359 : une seule admission du 7 septembre par le job CI garde
`calendar:main:run-once`, run `calendar-day-ingestion-2026-09-07-1788795814232828262`.
Les huit audits sont conserves : ForexFactory JSON et Eurostat couvrent le jour,
les six autres flux repondent HTTP 200 sans entree au jour cible. Aucun refus.
Le run termine en erreur a 15:48:45Z apres deux tentatives de travail durable,
avec `calendar_native_openrouter_failed` / `open_router_invalid_response`.
Zero item et resume ; aucun travail audio/alerte aval. La live news et sa
traduction continuent pendant cette inference. Pas de nouvelle admission.

Le correctif d'observabilite suivant distingue les etapes auparavant confondues :
requete locale, JSON invalide, enveloppe incompatible, erreur embarquee du
fournisseur, modele de reponse invalide et contenu invalide. Une erreur terminale
apres retries conserve aussi sa famille timeout/transport/429/serveur dans
l'artefact atomique admin/WebSocket. Aucun budget ni validation ne change.
Ces cas sont reproduits avec des fixtures synthetiques, sans affirmer lequel
explique le run historique : son corps n'a volontairement pas ete conserve.
Validation de ce complement : 366 tests Rust passants, trois ignores, fmt,
recuperation native et 13 tests Node verts. Le cas erreur fournisseur embarquee
verifie aussi atomicite, fencing, unicite, admin/WebSocket et absence de travaux aval.

Validation locale : fmt et gate de recuperation verts, suite complete 364 tests
passants, 0 echec, 3 ignores (dont la sonde reseau opt-in). Les trois warnings
preexistants hors calendrier restent inchanges. La sonde a ete compilee sur le
VPS dans un conteneur distinct limite a 1 CPU/2 Go (6 min 09 s), sans secret ni
base produit montes pendant la compilation. Elle n'a pas ete executee : l'agent
SSH local s'est reverrouille. Cette voie manuelle est abandonnee : commit et
deploiement passent exclusivement par GitLab CI. La sonde reste non executee,
son integration CI est differee ; aucune collecte ou completion reelle n'est
issue de cette sonde ; le run produit #359 ci-dessus constitue une observation
distincte. La livraison autorisee ne permet pas
d'attribuer une cause certaine au rejet historique.

## Évolution locale — sources officielles, 6 septembre 2026

Décision utilisateur : remplacer les collecteurs HTML Investing/ForexFactory par
cinq fournisseurs, huit endpoints fermés : ForexFactory **export JSON** ; BEA
JSON et ICS ; Federal Reserve calendrier FOMC et RSS monetary/speeches ; Eurostat
ICS « Euro indicators release » (tous thèmes) ; BCE calendrier statistique.
Investing, CNBC et TradingEconomics ne sont pas appelés. Cette évolution est
développée sur `main` ; elle ne change pas le SHA PROD décrit ci-dessous.

- Les huit flux sont tous examinés, séquentiellement, jamais arrêtés à la première
  couverture. Timeout 20 s et corps 1 Mo par requête ; aucune authentification,
  cookie, proxy, redirection suivie ou retry HTTP. Cache mémoire dédié : huit
  entrées maximum, TTL une heure, aucun corps source sur disque.
- Parseurs Rust JSON/ICS/HTML/RSS vers records typés filtrés au jour Europe/Paris.
  Au plus 128 records et 48 Ko normalisés par flux ; le JSON OpenRouter n'est
  jamais tronqué (limite globale 400 Ko avant transport).
- Les RSS sont du **contexte publié**, pas une preuve de calendrier. ForexFactory
  JSON couvrante est obligatoire depuis le correctif du 7 septembre. Les IDs et titres de provenance ainsi
  que les horaires proposés par OpenRouter sont vérifiés contre les records
  collectés en mémoire avant toute écriture SQLite.
- FOMC et ICS sans heure restent sans `eventTime`. Les heures BCE étiquetées CET
  sans preuve d'offset restent dans le contexte, sans alerte horodatée inventée.
- L'export ForexFactory ne garantit que la semaine présente : aucune URL de
  semaine suivante n'est inventée. Le BEA JSON peut dupliquer les publications
  ICS ; le prompt demande leur fusion avec conservation des deux provenances.
- Audits rédigés et métadonnées d'items utilisent les surfaces admin/WebSocket
  existantes. Le contrat Rust n'accepte que les huit identifiants de flux fermés :
  les variantes Investing, ForexFactory HTML et toute provenance libre sont
  rejetées. La sentinelle de refus après échec de persistance est versionnée
  pour ce set de flux.
- SQLite, leases/fencing, Piper/HLS, alertes et isolation du live sont réutilisés.
  Aucune nouvelle gate n8n, aucun déploiement DEV/PROD ni modification de secret.

Validation locale de cette évolution : `cargo fmt --check`, récupération ciblée
(0,18 s sur le diff final), `cargo test --offline --quiet` (358 passants,
0 échec, 2 ignorés), neuf tests Node d'observation et `git diff --check` verts.
La récupération utilise désormais le vrai parseur JSON ForexFactory sur une
fixture synthétique et vérifie les provenances avant SQLite. La sonde réseau
manuelle `official_calendar_feed_access_probe` (ignorée en CI) a obtenu huit
réponses 200 et huit parsings valides depuis le poste local. Pour son jour cible
du 16 septembre, FOMC, Eurostat et BCE sont couverts ; les cinq autres flux
n'ont pas d'entrée ce jour. Aucun appel OpenRouter réel ni audio produit n'a
été exécuté : l'IA et l'audio sont simulés dans les tests. La sonde a initialement
détecté le champ BEA `file_last_updated` ; son support typé et sa non-régression
sont ajoutés. Trois avertissements Rust préexistants hors calendrier subsistent.

Références : [export officiel ForexFactory](https://www.forexfactory.com/calendar),
[BEA JSON](https://apps.bea.gov/API/signup/release_dates.json),
[BEA calendrier](https://www.bea.gov/news/schedule),
[FOMC](https://www.federalreserve.gov/monetarypolicy/fomccalendars.htm),
[RSS Fed](https://www.federalreserve.gov/feeds/feeds.htm),
[souscription Eurostat ICS](https://ec.europa.eu/eurostat/subscribe/ics.format),
[calendrier BCE](https://www.ecb.europa.eu/press/calendars/statscal/html/index.en.html).
L'accès technique n'est pas une autorisation de redistribution commerciale :
les [conditions ForexFactory](https://www.forexfactory.com/notices) restent à
vérifier avant publication des données dans le produit.

## Derniere release PROD observee — #359, 7 septembre 2026

La pipeline [#359](https://gitlab.com/simonbdbc/filfinance/-/pipelines/2827006383)
sur `8aaeb24ea8b412b5613b3e40561c9475568de2c1` a ses 18 jobs verts,
dont la gate de recuperation et l'admission unique (une admission acceptee
n'implique pas la reussite du traitement asynchrone). Backend, Bunny landing,
Bunny PWA, edge et smokes livres. Six conteneurs sains ; SHA confirme sur le VPS.
Coupure HTTP observee de 27,499 s pendant le force-recreate, puis health HTTP 200.
Rollback annote `prod-before-calendar-source-cleanup-20260907` sur `1b670c4`.

## Historique de release — #355, 7 septembre 2026

La pipeline [#355](https://gitlab.com/simonbdbc/filfinance/-/pipelines/2824902650)
sur `1b670c44347975820584ee7e8ddbc89ed58c0a23` est en succes avec ses 17 jobs.
Le SHA a ete confirme dans `/srv/filfinance` sur le VPS PROD ; les six
conteneurs sont sains et `/health` repond `ok` avant la prochaine release.

## Historique de release — #353, 6 septembre 2026

La release [#353](https://gitlab.com/simonbdbc/filfinance/-/pipelines/2824536896)
sur `3db34303727e6d1b493b1a83a0088e3a7d66e2c7` est livree : 17 jobs verts,
backend/Bunny/edge et smokes valides. Coupure API observee environ 26 secondes.
La sonde interne prolongee n'a pas ete achevee lors de cette livraison (agent
SSH local verrouille) ; ne pas lui attribuer l'observation validee de #350.
Rollback : `prod-before-official-calendar-feeds-20260906`, sur `808a177`.
Le rejet ulterieur du run du 7 septembre a ete observe sur ce SHA ; ForexFactory
JSON et Eurostat couvraient le jour, sans cause de canonicalisation conservee.

## Historique de release — #350, 6 septembre 2026

**Release PROD Rust-only livrée et observation de 30 minutes validée.** La pipeline web
[#350](https://gitlab.com/simonbdbc/filfinance/-/pipelines/2823982017), sur
`808a177129134746be6eb08aacfdf57a516326f0`, termine ses 17 jobs en succès le
6 septembre à 09:20 UTC. Les onze validations précèdent tout déploiement ;
`calendar:native:recovery` réussit au premier essai (0,32 s pour le scénario,
budget live interne inchangé à une seconde). La validation locale du même arbre
comprend fmt, récupération au premier essai (0,15 s), 344 tests Rust passants,
0 échec et 1 ignoré, huit tests d'observation Node, les tests shell et diff-check.

Backend, landing/PWA Bunny et Worker edge sont livrés sur ce SHA. L'image backend
`sha256:dd9f4187ac729287f58e005cbcb39ac10eff87f1ef3ce4e1ccb441768bdb1840`
correspond à l'export de build #350 et au conteneur actif. Les sondes ont observé
des réponses API 502 de 09:15:20 à 09:15:46 UTC pendant la recréation contrôlée
(environ 26 s, échantillonnage de 5 s), puis le retour à 200 ; le healthcheck
edge live est resté à 200. Cela ne constitue pas une preuve de zéro déconnexion
WebSocket pendant la coupure acceptée.

Les smokes PROD sont verts : permissions des trois rôles, hello WebSocket live,
routes calendrier public/admin et deux audits de sources identiques dans le
snapshot WebSocket admin, manifeste Bunny et segment existant de 42 864 octets.
Aucun calendrier ni clip n'a été créé pour ce contrôle. Les deux sources ont
refusé la collecte (403) ; le calendrier du 7 septembre reste en erreur rédigée,
sans item ni résumé, conformément au risque accepté. La persistance de news
FinancialJuice reprend après le redémarrage. Les trois comptes smoke ont chacun
zéro session persistée après la CI. DEV reste gelé ; aucun volume n8n n'est purgé.

L'observation serveur s'étend de 09:16:21 à 09:46:24 UTC, après le succès backend
(1 802 s, 30 échantillons, script terminé avec le code 0). Tous les services et
healthchecks sont sains : aucun redémarrage imprévu, erreur backend/audio,
panic, doublon, lease expirée ou travail orphelin détecté. Les news retenues
passent de 116 à 119 entre le premier et le dernier échantillon ; dernière
persistance observée à 09:16:57 UTC. La suite de la fenêtre est calme : le smoke
WebSocket prouve l'authentification/hello, pas un débit continu de publications.
La livraison opérationnelle est validée, avec le calendrier dégradé accepté.
Preuves rédigées hors Git : `backups/calendar-rust-release-20260905/`, notamment
`release-350-evidence.json` et les journaux JSONL d'observation/coupure.
Rollback exact : `prod-before-calendar-rust-20260905` (`67b48bb`), sans
réactivation automatique de n8n ni suppression de données.

### Historique des validations et incidents avant #350

**Gate locale Rust-only validée sur `main`.** Depuis `filfinance-service`,
`cargo fmt --check`,
`cargo test native_calendar_recovery_e2e_is_idempotent_and_keeps_live_news_unblocked -- --nocapture`
et `cargo test` ont réussi le 30 août 2026 ; la suite complète compte 341 tests
passants. Le chemin Rust canonique et le retrait des artefacts n8n sont désormais
commités et poussés sur `main`.

**Gate Rust synchronisée validée en CI ; livraison suspendue au correctif SSH CI.** Sur le
SHA `b84e7993`, la pipeline web #339 a échoué une première fois sur l'attente du
broadcast live à une seconde, puis réussi au retry. Cette pipeline ne sera pas
utilisée pour le backend. Le scénario attend désormais un signal interne au
test, émis après l'initialisation de la boucle d'ingestion, avant d'injecter la
news. Le budget d'une seconde couvre ensuite admission, persistance et
broadcast avec la file calendrier saturée ; il n'est pas augmenté. La cause du
timeout initial reste une hypothèse de scheduling, mais la preuve réussit au
premier essai dans #341 sur `e628d9c` (8,29 s pour le scénario complet ; le budget
live interne reste une seconde). Les onze validations de #341 sont vertes, sans
retry. Aucun changement fonctionnel calendrier n'est inclus.

Le backend #341 échoue avant SSH le 5 septembre à 16:41 UTC : les commentaires
émis sur stdout par `ssh-keyscan` sont transmis à `ssh-keygen`, qui retourne 255.
La reproduction Alpine confirme ce défaut de préparation CI ; aucun conteneur
PROD n'a été recréé. Le filtrage des commentaires et lignes vides est corrigé,
avec un test du bloc CI réel couvrant aussi empreinte incorrecte, absence de clé
et ligne malformée. La vérification stricte de la clé hôte reste obligatoire.
La livraison doit repartir d'une nouvelle pipeline web sur ce correctif.
Les sept scenarios SSH passent localement sous Bash et sous BusyBox ash dans
un conteneur Alpine 3.22 jetable, sans montage des donnees PROD. Apres ce seul
changement CI, fmt, check, recuperation ciblee (0,47 s), suite Rust complete
(344 passants, 1 ignore), gardes CI et parsing YAML sont de nouveau valides.

La pipeline web #344 sur `1d3a26c` confirme ensuite les onze validations au
premier essai, dont la récupération Rust. Le pinning SSH passe, mais le backend
échoue encore avant connexion distante sur quatre guillemets non échappés dans
la commande SSH multilignes (`syntax error: unexpected "("`). Le correctif
échappe uniquement ces guillemets ; le nouveau test
`scripts/test_backend_deploy_shell.py` parse le bloc CI réel puis sa commande
distante, et vérifie avec un faux SSH que Git/Docker ne s'exécutent pas localement.
Le 6 septembre, `cargo fmt --check`, la récupération ciblée au premier essai
(0,17 s), puis `cargo test` (344 passants, 0 échec, 1 ignoré) réussissent de
nouveau sur cet arbre ; le budget live strict reste inchangé.
La landing et la PWA Bunny de #344 sont publiées ; backend, edge, smokes et
observation ne constituent pas encore une release complète validée.

Les trois comptes smoke PROD ont été explicitement autorisés et créés le
6 septembre : listener, moderator et admin, adresses vérifiées. Leurs six
variables GitLab sont protégées, masquées et cachées, limitées à
`production/smoke`, sans modification des variables DEV. Connexion HTTPS,
restauration de session, rôle JWT et déconnexion réussissent pour chaque rôle ;
aucune session ni copie temporaire locale de mot de passe ne subsiste.

Le smoke PROD est complete par des lectures authentifiees des routes calendrier,
du snapshot WebSocket admin et de ses audits rediges, ainsi que d'un manifeste
et d'un segment replay HLS Bunny existants. Aucun run ni clip n'est produit pour
ce controle ; l'absence de replay est signalee. Cette observation utilise les
comptes smoke scopes, sans changer les contrats ou le runtime Rust. La release
doit prendre une nouvelle pipeline sur le SHA incluant ce controle, sans
reprendre les deploiements de #346 sur le SHA intermediaire `7f9655a`.
Le premier test d'observation de #348 a detecte un format de date ICU different
sous Alpine ; le script utilise maintenant `formatToParts`, avec une fixture
independante de l'ordre d'affichage. Aucun deploiement n'a ete lance par ces
pipelines intermediaires ; la prochaine release doit inclure ce correctif.

L'inventaire OVH confirme une image déjà Rust-native sur `67b48bb`, calendrier
activé, six services sains et aucun conteneur n8n sur cet hôte. La provenance
BuildKit et les quinze couches de l'image correspondent au runtime. Le tag exact
`prod-before-calendar-rust-20260905` est créé et poussé sur ce SHA ; le tag du
30 août ne désigne pas le runtime actuel. L'inventaire et les trois workflows
historiques versionnés, exportés hors Git et rédigés, sont conservés dans
`backups/calendar-rust-release-20260905/`. Ce sont les workflows du dépôt avant
retrait, pas un export d'une instance active (aucune sur la cible OVH). Les autres
hôtes historiques ne sont pas couverts par cet inventaire. #339 est annulée,
sans effacer ses résultats, pour libérer la file PROD `oldest_first`.

Les observations DEV, le shadow et les campagnes multi-jours ne font plus partie
de la décision de migration. Ils ne doivent pas être relancés ni utilisés comme
une gate. Les anciennes informations de shadow restent accessibles dans
l’historique Git, mais ne décrivent plus la procédure active.

## Périmètre et invariants

- Le développement se fait directement sur main.
- ForexFactory JSON couvrante est obligatoire pour produire le calendrier ;
  les huit flux officiels ci-dessus sont tous examines pour
  fusionner et dedupliquer leurs evenements. Sans couverture ForexFactory, le run
  termine par une erreur rédigée, sans item, audio ni alerte.
- Les collectes sont strictement séquentielles, dans l'ordre du set de flux,
  au plus 20 secondes par source, sans cookie, proxy, autorisation, redirection
  suivie ni retry HTTP.
- Les statuts 401, 403, 429, 451 et les challenges anti-bot sont des refus
  terminaux pour la source concernée. Aucune protection anti-bot ne doit être
  contournée ; les autres sources autorisées restent néanmoins collectées. L'audit du
  refus est commité avant tout appel IA rejouable, afin que le claim suivant
  mette cette source en quarantaine pour le jour au lieu de la recontacter.
  Si cette persistance échoue après un refus observé, la file est dead-lettered
  fail-closed avec un code rédigé avant toute nouvelle admission : aucun retry
  automatique ne peut refaire la requête source.
- Les pages source et sorties IA restent en mémoire. SQLite conserve, pour les
  audits, seulement le verdict, statut HTTP, hash et taille ; pour un item, elle
  conserve uniquement la provenance sélectionnée (`source`, identifiant externe,
  URL canonique datée et titre brut), jamais le HTML ni le JSON fournisseur.
- Les audits rédigés sont publiés via l'artifact et le WebSocket admin existants,
  sans contrat supplémentaire ni contenu source.
- Une réingestion partiellement couverte fait l'union de la provenance validée
  déjà persistée et de la source nouvellement collectée : `sources_json` et les
  lignes enfants ne peuvent pas être remplacés par `[]` ni perdre l'autre
  collecteur autorisé.
- Tous les contrats entre étapes sont des types Rust. Aucun JSON non typé ne
  devient un contrat interne.
- La migration ne touche ni la file de traduction ni l’outbox FinancialJuice.
- Les champs SQLite historiques n8n_* restent nullables et les migrations déjà
  appliquées sont conservées afin de préserver le contrat admin.

## Architecture cible

    Admission admin / scheduler Rust
      -> CalendarSourceCollector
      -> OpenRouterCalendarClient + prompts typés
      -> validation et canonicalisation Rust
      -> transaction CalendarDayStore / calendar_day_*
      -> file SQLite calendrier durable
           CollectAndIngest -> Summarize
           ItemAudio | AlertAudio | SummaryAudio
      -> workers Piper/HLS existants + calendar_alert_ready

La file durable calendrier est isolée de toute outbox FinancialJuice ou
traduction. Chaque travail porte un état fermé, un compteur de tentatives, une
échéance, une lease, un fencing token et une clé d’idempotence. Les reprises de
leases expirées précèdent toute nouvelle admission au démarrage. Le budget est
strictement de trois claims : une lease expirée ou un retry au budget passe
atomiquement en `dead_letter`, sans nouveau fencing token ni appel fournisseur.

Les notifications de réveil utilisent une file mémoire bornée et try_send :
elles sont best-effort, ne bloquent jamais le chemin appelant et SQLite reste la
source de vérité. La concurrence, les retries et le nombre de travaux ouverts
sont bornés. Le live news ne consomme ni la file calendrier ni son budget de
concurrence.

Les écritures d’ingestion, d’état, de couverture, d’alertes et de travaux aval
doivent être atomiques dans calendar_day_*. Le stream_id est réservé avant
Piper/HLS afin qu’une reprise ne publie pas un second clip. Piper, HLS,
calendar_alert_ready et les broadcasts restent best-effort : aucun downstream
ne peut attendre le pipeline live news.

Toute réparation de manifeste HLS passe elle aussi par le travail durable
`SummaryAudio`, avec lease, fencing et `stream_id` déjà réservé. Aucune route GET
ni vérification automatique ne peut envoyer directement un job audio en mémoire.
L'absence du manifeste `current.m3u8` est toujours une anomalie à réparer, même
si l'immuable existe ; une reprise réutilise les segments du stream réservé pour
recréer les manifests sans publier un second clip.

L’état HLS minimal d’un clip calendrier réservé est écrit sur disque avant son
commit mémoire. Après une coupure au milieu de l’intro ou du corps, Item, Alert
et Summary restaurent donc les mêmes segments et le même stream. Le chemin live
reste mémoire-only et refuse toujours un segment préexistant.

## Phasage

### 1. Contrats Rust et composants IA partagés — NEWS-217 / NEWS-218

**Implémentée et testée.** Les contrats de collecte, canonicalisation,
importance, résumé et erreurs sont figés en Rust. Le client OpenRouter et les
prompts sont des composants partagés du chemin natif : timeout, réponse maximale
à 1 Mo, concurrence IA égale à un, retries bornés et redaction des
secrets/contenus.

À valider par les tests unitaires de contrats, prompts, parsing, langue visible,
réponses invalides, limites et redaction. Aucun résultat n8n n’est un oracle.

### 2. Collecte native autorisée — NEWS-219

**Implémentée et testée.** `CalendarSourceCollector` porte les identifiants
fermés des huit flux officiels ci-dessus, les résultats typés Covered, Unavailable et
Refused, et les audits rédigés. Les appels de production respectent les limites
ci-dessus ; tous les flux sont toujours examinés puis leurs snapshots exploitables
sont transmis ensemble à la qualification.

À valider par des tests des URL fermées, du timeout, de l’absence de redirection
et de retry HTTP, des classifications de refus, des limites de corps et de
l’absence de fuite de contenu source.

### 3. Orchestration SQLite canonique — NEWS-220

**Implémentée et testée.** L’admission admin et le scheduler sont raccordés à
la file SQLite calendrier et le runtime Rust remplace les coordinateurs mémoire
et callbacks n8n. Les transactions durables créent les items, le résumé, les
alertes et les travaux audio idempotents ; elles ne dépendent d’aucune outbox
hors calendrier. Elles enregistrent aussi la provenance métadonnée des items,
les audits de collecte redigés et leurs artifacts admin/WebSocket. Les réparations
HLS réadmettent uniquement le travail `SummaryAudio` durable existant. Les
leases expirées au budget sont terminalisées dans SQLite, l’échec de persistance
d’un audit de refus ferme le jour, et une seconde réparation GET concurrente est
un no-op normal plutôt qu’une erreur publique.

À valider par des tests de leases, fencing tokens, capacité, retries bornés,
réservation stable de stream_id, idempotence et admission non bloquante, y
compris les reprises HLS après écriture de segments et l’isolation du live.

### 4. Test de récupération unique — gate de migration

Le test native_calendar_recovery_e2e_is_idempotent_and_keeps_live_news_unblocked
est la seule gate fonctionnelle de migration. Il doit être exécuté localement
avec :

- une SQLite fichier temporaire et une horloge fixe ;
- collecteurs et OpenRouter simulés, sans réseau, secret réel ni n8n ;
- Piper/HLS déterministes dans un répertoire temporaire ;
- une admission via l’API admin réelle ;
- une interruption injectée après le commit calendrier et avant le handoff audio ;
- un redémarrage sur la même SQLite et le même répertoire HLS ;
- une reprise prioritaire des travaux calendrier, sans item, résumé, stream,
  alerte ou publication dupliqués et sans travail orphelin ;
- une réparation HLS concurrente après disparition du manifeste, qui conserve le
  même stream et ne contourne ni la lease ni le fencing ;
- les routes publiques/admin, le WebSocket admin, les manifests HLS et la
  persistance/broadcast d’une live news durant une saturation ou lenteur
  calendrier.

Cette gate a été **validée localement le 30 août 2026**, puis revalidée avec
la synchronisation de démarrage le 5 septembre 2026 par
`native_calendar_recovery_e2e_is_idempotent_and_keeps_live_news_unblocked`.
Elle a exercé l’admission admin réelle, le commit puis l’interruption, le
redémarrage sur la même SQLite/HLS, la reprise idempotente, les surfaces
calendrier et la persistance/broadcast live news. Elle ne compare aucune sortie
n8n, ne requiert ni cinq jours d’observation ni smoke:dev, et ne crée pas une
seconde gate de parité.

### 5. Retrait n8n et livraison Rust-only — NEWS-221

Le retrait de code est **commité et poussé sur `main`** : workflows,
`compose.n8n.yaml`, routes internes, configuration, variables, scripts,
contrôles CI/runbooks et modules Rust devenus morts sont retirés ;
`filfinance_news_error_alert` est retiré avec son automatisation. La CI exécute
désormais le même test Rust de récupération. Les migrations SQLite déjà
appliquées et les champs historiques `n8n_*` nullables sont conservés.

Préparation effectuée avant la livraison #350 :

1. reconfirmer le runtime face au tag exact déjà poussé
   `prod-before-calendar-rust-20260905` (`67b48bb`) ; le tag du 30 août est exclu ;
2. conserver l'inventaire OVH et les exports historiques rédigés déjà archivés
   hors Git, y compris `filfinance_news_error_alert` ; revérifier l'absence de
   conteneurs n8n sur la cible avant la bascule ;
3. valider localement le correctif de commande SSH, le pousser sur `main`,
   puis créer une nouvelle pipeline web de release avec les verrous PROD/Bunny.
   Le test ciblé doit réussir dès son premier essai dans cette pipeline avant
   tout job de déploiement ; le script retire les conteneurs n8n sans option de
   suppression de volume ;
4. conserver les volumes, la base et les migrations n8n/SQLite historiques pour
   audit et rollback. Aucune purge destructive n’est autorisée sans décision
   distincte.

Le runtime OVH exécute maintenant le SHA #350 Rust-only et sans conteneur n8n.
Les déploiements, smokes et 30 minutes d'observation ont réussi le 6 septembre
2026 ; la livraison opérationnelle est clôturée avec le risque source accepté.
La réexécution du même test Rust-only contrôle la non-régression du même
critère, sans nouvelle gate de parité ni campagne multi-jours.

## Contrôles locaux validés

Depuis filfinance-service :

    cargo fmt --check
    cargo test native_calendar_recovery_e2e_is_idempotent_and_keeps_live_news_unblocked -- --nocapture
    cargo test

Résultats consignés le 30 août 2026 : les trois commandes réussissent ; le test
ciblé réussit et `cargo test` termine avec **341 passants, 0 échec**. Cette preuve
valide le chemin Rust-only local et le retrait n8n du dépôt, pas un déploiement
déjà réalisé.

Revalidation du 5 septembre 2026, sur le correctif de synchronisation basé sur
`b84e7993` : `cargo fmt --check`, `cargo check`, le test ciblé (0,16 s) et la
suite complète réussissent : **344 passants, 0 échec, 1 ignoré** (test Redis
nécessitant une instance jetable). Le premier lancement sous sandbox s'est
arrêté sur `PermissionDenied` lors du bind du WebSocket localhost ; l'exécution
autorisée avec sockets locaux a réussi au premier essai, ainsi que le scénario
dans la suite complète. Aucun timeout live n'a été observé après synchronisation.
Les avertissements Rust préexistants restent hors de ce correctif. La preuve
CI sans retry sur `e628d9c` est acquise dans #341 ; elle a été rejouée avec
succès dans #350 sur le SHA exact de livraison `808a177`, sans changer le test
ni augmenter le budget live.

## Livraison et rollback

La séquence de livraison est : développement complet sur main, tests locaux
validés, retrait complet de n8n du dépôt, revue et push de main, puis
déploiement PROD manuel.
Les contrôles PROD ultérieurs vérifient la livraison et l’absence de régression
des surfaces publiques ; ils ne remplacent pas la gate locale Rust.

Le rollback redéploie le tag de release précédent. Il ne dépend pas du maintien
d’un n8n actif. Restaurer un workflow depuis l’export protégé n’est possible
qu’avec une décision opérateur explicite. Ne pas supprimer les volumes n8n, les
migrations SQLite appliquées ou les champs historiques n8n_* pendant ce
rollback.

## Hors scope

- Modifier les routes publiques/admin, le WebSocket admin ou les manifests HLS.
- Coupler le calendrier à FinancialJuice, à la traduction ou à leurs outboxes.
- Contourner une protection anti-bot ou exposer un secret, prompt, source ou
  réponse IA brute.
- Faire du live news un dépendant du calendrier.
- Réintroduire une campagne shadow/DEV ou une parité n8n comme condition de
  bascule.

## Gate suivante

La gate de migration est clôturée par #350, ses smokes et les 30 minutes
d'observation réussies. Pour les prochaines releases, conserver le même test
Rust de récupération sans retry et son budget live d'une seconde ; aucune
seconde gate de parité n8n n'est ajoutée. Un éventuel accès autorisé aux sources
403 est un chantier distinct : aucun contournement, aucune réactivation n8n ni
purge de volumes. Le rollback de cette livraison reste le tag exact
`prod-before-calendar-rust-20260905` ; ne pas réutiliser #339.
