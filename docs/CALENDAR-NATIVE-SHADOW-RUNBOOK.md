# Runbook — calendrier Rust natif

> Ce fichier remplace le runbook d’observation shadow. Son chemin historique est
> conservé temporairement afin de ne pas casser les liens internes. Le calendrier
> n8n/shadow n’est plus une procédure opérationnelle.

## Statut et règle d’exécution

Livraison Dots préparée le 7 septembre : `dots-studio/dots-3-note-preview:free`,
endpoint AtlasCloud gratuit annonçant `response_format` et `structured_outputs`.
L'ingestion revient à JSON Schema strict pour tous les modèles ; l'exception
JSON simple MiniMax est supprimée. Logs détaillés masqués, limites et validations
Rust sont inchangés. Migration et déploiement via CI uniquement, DEV gelé.
Aucune nouvelle admission calendrier n'est autorisée par ce changement.

L'admission CI `16352353222` du 7 septembre à 17:56:08Z a échoué après 3,1 s :
OpenRouter HTTP/code fournisseur 404, modèle MiniMax M3 gratuit indisponible.
Huit flux examinés, ForexFactory et Eurostat couvrants, aucun item/résumé/audio/
alerte, aucun travail actif. Admin/WebSocket et HLS Bunny validés par le smoke
`16352364629`, live et traductions actifs, six conteneurs sains. Le catalogue
public MiniMax ne proposait plus aucun endpoint ; aucun fallback payant effectué.

Historique de la release #363 (`1972d5a`) du 7 septembre : modèle demandé
`minimax/minimax-m3:free` et conservation des erreurs OpenRouter détaillées dans
les logs serveur (code numérique, message, corps borné et credentials masqués).
L'API publique des endpoints MiniMax annonce `response_format`, mais pas
`structured_outputs` ; la fiche officielle précise l'absence d'enforcement
JSON Schema. Le mode `json_object` a été explicitement accepté le 7 septembre
pour ce modèle exact, sans affaiblir les validations métier Rust. Tous les autres
modèles conservent le schéma strict ; aucun fallback silencieux.
Référence : [MiniMax M3 free](https://openrouter.ai/minimax/minimax-m3:free).

La relance unique du run `calendar-day-ingestion-2026-09-07-1788798437600965868`
via le job CI `16351366164` a terminé le 7 septembre à 16:28:21Z avec
`open_router_embedded_error` : marqueur d'erreur dans l'enveloppe fournisseur,
avant validation métier. Sources couvertes : ForexFactory et Eurostat. Aucun
item/résumé/audio/alerte, un seul artefact terminal, aucun travail encore actif.
Le serveur n'a pas été redéployé pour cette admission ; le live est resté actif.

Observation PROD du 7 septembre 2026 : release #359 (`8aaeb24`), 18 jobs CI
verts, coupure controlee mesuree 27,499 s. L'admission unique via CI a echoue
avec `open_router_invalid_response` apres couverture ForexFactory et Eurostat ;
aucun item/resume ni travail aval cree, live et traductions actifs. Ne pas
rejouer le job d'admission apres cet echec. Le succes du job atteste l'admission,
pas la reussite asynchrone de la collecte/IA. La validation du 30 aout ci-dessous
est historique, pas le statut courant du deploiement.

Le diagnostic renforce distingue requete locale, JSON, enveloppe, erreur
fournisseur embarquee, modele et contenu ; apres epuisement des retries il
conserve timeout/transport/rate-limit/serveur. Ces codes fermes ne transportent
aucun message fournisseur. Le motif historique `open_router_invalid_response`
ne permet pas de choisir retroactivement une de ces causes. Ne pas attribuer
l'incident a un schema incompatible ou a un quota sans preuve.

**Statut : gate locale de récupération validée le 30 août 2026.**
`cargo fmt --check`, le test de récupération ciblé et `cargo test` ont réussi
depuis `filfinance-service` ; la suite complète compte 341 tests passants. Le
chemin Rust-only et le retrait des artefacts n8n sont prêts dans l’arbre de
travail, mais aucun push, tag ni déploiement PROD n’a été effectué par cette
validation locale.

Le chemin cible est entièrement Rust :

    admission admin / scheduler
      -> ForexFactory JSON / BEA JSON+ICS / Fed FOMC+RSS / Eurostat ICS / BCE statistiques
      -> OpenRouter typé et validation Rust
      -> transactions calendar_day_*
      -> file SQLite calendrier
      -> Piper/HLS et alertes existants

Le live news est un pipeline indépendant. Il ne doit jamais attendre une
collecte, une inférence, un retry, un audio ou une alerte calendrier.

## Invariants d’exploitation

- ForexFactory export JSON valide et couvrante pour le jour Paris est obligatoire ; les RSS Fed
  sont du contexte publié et ne créent jamais seuls un événement ou une alerte.
- Si ForexFactory ne couvre pas le jour, terminer avant OpenRouter avec
  `calendar_native_required_source_unavailable` et un motif type ; ne creer
  ni item, ni resume, ni audio, ni alerte et conserver l'historique.
- Les sept autres flux restent facultatifs, tous examines. Leurs evenements
  complementaires sont autorises si ForexFactory couvre le jour. Leur echec
  ne retire pas cette couverture. Aucun endpoint alternatif pour les jours
  futurs absents de l'export de semaine courante.
- Examiner les huit flux officiels séquentiellement, avec un timeout
  maximum de 20 secondes et 1 Mo par source. Cache en mémoire uniquement, au
  plus huit corps, TTL une heure. Investing, les pages HTML ForexFactory et
  toute provenance non présente dans le set fermé sont rejetés.
- Les parseurs filtrent au jour Paris. ForexFactory ne promet que la semaine
  courante ; aucun endpoint alternatif n'est essayé pour obtenir la suivante.
  Eurostat utilise l'ICS officiel tous thèmes/catégorie « Euro indicators release ».
- Les dates sans heure FOMC/ICS et les heures BCE à offset incertain restent
  sans horaire d'alerte. OpenRouter doit reprendre les identifiants de records
  exacts ; Rust refuse une provenance ou un horaire inventé avant le commit.
- Ne transmettre ni cookie, ni autorisation, ni proxy, ni redirection suivie ;
  ne faire aucun retry HTTP de source.
- Classer 401, 403, 429, 451 et les challenges anti-bot comme refus terminaux.
  Ne jamais tenter de contourner une protection ou de rejouer une requête
  refusée pour le même run.
- Une fois ce refus persisté, ne jamais recontacter cette source pour le jour,
  y compris via `force=true` ou la vérification hebdomadaire. Si ForexFactory
  ne couvre pas le jour, son run devient terminal. Une nouvelle admission peut
  interroger les sources non refusées ; les sources en quarantaine restent exclues.
- Si l’écriture de cet audit échoue après un refus observé, dead-letterer la
  file avec le code rédigé de refus avant toute nouvelle admission : ne jamais
  transformer ce cas en retry de collecte.
- Garder les pages source, prompts et réponses IA uniquement en mémoire. Ne
  persister dans le produit que des hashes, tailles, états et codes redigés.
  Exception autorisée : les corps des réponses OpenRouter en erreur sont
  conservés dans les logs serveur avec code numérique et message, corps 64 Kio
  et message 4 Kio maximum avant échappement. Credentials reconnus masqués,
  troncature explicite ; aucune réponse réussie ni requête n'est journalisée.
- Limiter l’inférence IA à une requête active, une réponse de 1 Mo maximum et
  des retries bornés.
- Garder la file SQLite calendrier séparée de l’outbox de traduction et de
  toute donnée FinancialJuice.

## File SQLite et reprise

La file calendrier durable contient les intentions CollectAndIngest, Summarize,
ItemAudio, AlertAudio et SummaryAudio. Toute intention comporte :

- une clé d’idempotence ;
- l’état fermé pending, leased, completed ou dead_letter ;
- un compteur de tentatives et une échéance ;
- une lease avec owner, expiration et fencing token ;
- un stream_id réservé, lorsqu’un clip Piper/HLS est concerné.

Le budget est de trois claims au plus : une lease expirée ou un retry au budget
est terminalisé atomiquement. Les clips calendrier conservent un état HLS
minimal à côté de leurs segments ; une reprise Item/Alert/Summary réutilise le
stream réservé sans second clip. Le live news ne lit pas cet état et garde son
refus de collision de segment.

Au démarrage, récupérer les leases expirées avant toute nouvelle admission. Les
réveils mémoire sont bornés et best-effort via try_send ; ils ne remplacent
jamais le polling/recovery SQLite. Une file pleine ou un downstream lent doit
rester visible par une erreur rédigée, sans bloquer l’API admin, le scheduler ou
le live news.

Les écritures d’ingestion doivent être atomiques : état de run, couverture
source rédigée, items, alertes et travaux aval. Le résumé est déclenché après le
commit d’ingestion. Le stream_id est réservé avant Piper/HLS afin qu’une reprise
ne publie jamais un second clip.

## Test de récupération — gate unique validée

Exécuter depuis filfinance-service :

    cargo fmt --check
    cargo test native_calendar_recovery_e2e_is_idempotent_and_keeps_live_news_unblocked -- --nocapture
    cargo test

Le scénario unique attendu utilise une SQLite temporaire, une horloge fixe, des
collecteurs/OpenRouter simulés et des sorties Piper/HLS déterministes. Il ne
requiert ni réseau, ni secret réel, ni n8n.

La preuve doit couvrir dans le même test :

1. admission par l’API admin réelle ;
2. collecte native, qualification et commit SQLite ;
3. création durable des travaux audio et alertes ;
4. interruption juste après le commit calendrier et avant le handoff audio ;
5. redémarrage avec la même SQLite et le même répertoire HLS ;
6. reprise prioritaire des travaux calendrier ;
7. unicité des items, résumé, streams, alertes et publications par clé ;
8. absence de travail orphelin ;
9. routes publiques/admin, WebSocket admin et manifests HLS ;
10. persistance et broadcast d’une live news pendant une saturation ou une
    lenteur du calendrier.

Cette commande et ces assertions sont **validées le 30 août 2026** : le test
ciblé a réussi et la suite `cargo test` compte 341 passants, sans échec. Une
campagne multi-jours, un résultat n8n, un rapport shadow ou un smoke DEV ne les
remplace pas.

## Retrait n8n après gate validée

Le retrait du dépôt et de la CI est réalisé : workflows, `compose.n8n.yaml`,
routes internes, configuration, variables, scripts, runbooks et modules Rust
devenus morts sont retirés ; `filfinance_news_error_alert` est retiré et la CI
Rust-only exécute le même test de récupération. Les routes publiques/admin, le
WebSocket admin et les manifests HLS ne sont pas modifiés.

Avant déploiement, il reste à :

1. poser un tag sur la release actuellement déployée ;
2. inventorier et exporter hors Git les workflows calendrier n8n sans
   credentials, y compris `filfinance_news_error_alert` ;
3. pousser `main`, puis déployer manuellement ; la suppression des conteneurs
   n8n ne doit jamais utiliser une option supprimant les volumes ;
4. conserver les migrations SQLite déjà appliquées, les colonnes historiques
   `n8n_*` nullables, la base et les volumes n8n ; ne purger ni volume ni base
   sans autorisation distincte.

## Déploiement et contrôles de livraison

La séquence autorisée est :

    développement complet sur main
      -> tests locaux validés
      -> retrait n8n du dépôt
      -> revue et push main
      -> déploiement PROD manuel

Les contrôles post-déploiement portent sur la livraison : santé du service,
routes calendrier publiques/admin, WebSocket admin, manifests HLS et live news.
Ils ne sont pas une nouvelle validation du processus métier calendrier.

## Diagnostic isole d'ingestion (opt-in, sans livraison)

Pour l'admission normale apres une release, le job manuel
`calendar:main:run-once` utilise le compte smoke admin existant dans le scope
`production/smoke`. Il exige `FILFINANCE_CALENDAR_RUN_AUTHORIZED=true`,
`FILFINANCE_CALENDAR_RUN_DAY` et `FILFINANCE_CALENDAR_RUN_EXPECTED_ID` (dernier
run d'ingestion en erreur inspecte par l'operateur). Les trois variables doivent
etre renseignees lors de la creation de la pipeline : sans elles, le job est
absent plutot qu'executable en echec. Il n'est disponible qu'apres `smoke:main`,
dans une release PROD protegee explicitement demandee.
Le JWT reste en memoire ; aucun secret n'est affiche. Un seul POST vers
`/api/admin/calendar-day/run` est permis, sans retry. Un dernier run different,
un travail actif ou une source refusee arrete le job avant admission. En cas
de resultat reseau incertain, inspecter le run serveur avant toute autre action.
Le succes du job prouve l'admission uniquement : suivre ensuite l'ingestion,
le resume et l'audio jusqu'a un etat terminal via l'admin et les lectures serveur.

Le code public `calendar_native_ingestion_rejected` reste stable. Consulter
l'artefact `calendar_native_ingestion_diagnostic` du run via l'admin HTTP ou le
WebSocket : motif ferme, index/champ optionnels, fin de generation et code HTTP
OpenRouter lorsqu'il existe. L'artefact
est atomique avec l'erreur terminale et ne contient aucun texte brut. Le client
ingestion utilise `json_schema`, `strict=true` pour tous les modèles.
`provider.require_parameters=true`, `finish_reason=stop` et toutes les validations
Rust restent obligatoires. Le resume conserve son contrat historique.

Toute mise a jour du serveur passe exclusivement par GitLab CI. Aucun transfert
manuel de sources, compilation ou lancement de sonde sur le VPS par SSH.
Le Dockerfile distinct prepare la sonde, mais aucun job CI ne l'execute encore :
son integration est differee et elle ne fait pas partie de cette livraison.

Un futur job autorise devra borner CPU/memoire, utiliser un repertoire de
diagnostic protege (0700), sans modifier les volumes produit, et monter la base
et ses fichiers WAL ensemble en lecture seule.
La sonde ne cree aucun AppState, worker, scheduler ou migration. Elle verifie
les capacites publiques du modele configure, lit les quarantaines, collecte une
fois puis compare deux appels maximum, sequentiels et sans retry, sur les memes
snapshots memoire. Une couverture ForexFactory est exigee avant les appels IA.
La sonde refuse de demarrer ou de continuer si un travail d'ingestion/resume
produit est pending ou leased. L'operateur doit aussi choisir une fenetre hors
scheduler et ne pas admettre de run calendrier pendant la sonde ; ce controle
en lecture seule n'est pas un verrou distribue contre une admission concurrente.

Le marqueur `2026-09-07.started` est cree exclusivement et conserve meme en
echec : **ne pas le supprimer pour retenter**. Tout refus source suspend les
appels IA de cette sonde et toute relance jusqu'a revue operateur et prise en
compte de la quarantaine. Conserver seulement les rapports rediges hors Git.
Ne pas utiliser `--ignored` sans le filtre exact de la sonde ; l'image le fixe.
Les secrets restent sur le serveur et ne doivent jamais etre affiches.

Un diagnostic valide n'est pas une livraison. Apres tests puis deploiement
autorise, relancer une fois le 2026-09-07 via l'API admin normale, jamais via
SQL. En cas d'echec, arreter et lire le nouveau motif ; ne pas relacher les
validations metier ni attribuer une cause historique a une nouvelle completion.

## Rollback de release

Le rollback redéploie le tag de release précédent. Il ne dépend pas d’un n8n
resté actif. L’export protégé des workflows ne peut être restauré qu’avec une
décision opérateur explicite.

Pendant un rollback, conserver les volumes n8n, les migrations SQLite appliquées
et les colonnes n8n_* historiques. Ne jamais purger les données afin de rendre
un rollback plus simple.

## Non-objectifs

- Réactiver le shadow ou une campagne DEV.
- Comparer des sorties n8n à la sortie Rust comme vérité métier.
- Coupler le calendrier à FinancialJuice, à la traduction ou à leurs outboxes.
- Contourner un refus anti-bot, exposer un secret ou conserver du contenu source
  brut dans SQLite. Les logs d'erreur OpenRouter constituent l'exception décrite
  ci-dessus ; leurs messages et corps ne sont pas des artefacts calendrier admin.
