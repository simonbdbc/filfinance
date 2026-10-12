# Roadmap Fil Finance

## Vision cible

Le workspace `filfinance` porte maintenant le produit `Fil Finance`, avec une app publiee sous le nom technique `filfinance`.

`Fil Finance` doit devenir un ensemble coherent service plus client, capable de:

- scrapper ou consommer plusieurs sources en continu avec une latence faible et stable
- normaliser les evenements dans un modele unique
- persister l'historique pour relecture et debug
- diffuser le flux en WebSocket sans bloquer l'ingestion
- exposer un flux texte EN+FR temps reel exploitable par les clients et par le debug
- documenter le code Rust comme un support d'apprentissage progressif pour des devs venant surtout de `JavaScript`, `PHP` et `Java`
- traduire localement les news EN vers FR via `OPUS-MT tc-big en-fr`
- produire une narration TTS cote serveur pilotee par Rust
- diffuser cette narration sous forme de clips audio HLS finalises et cacheables
- permettre a un client de lire le live audio sous forme de clips HLS finalises par news, declenches par evenement temps reel
- exposer le niveau source `FinancialJuice` des news, notamment `active active-critical`, jusqu'au client, avec libelle utilisateur `Élevée`
- factoriser le client Expo en architecture `feature-based` avec composants réutilisables gluestack v5, UniWind et Tailwind CSS 4
- presenter un branding coherent `Fil Finance` cote client, docs, manifests et identifiants runtime
- proteger l'application avec Better Auth et PostgreSQL via le service `filfinance-auth`
- utiliser PostgreSQL pour les profils, roles et preferences, et SQLite pour le pipeline news/audio
- scaler la diffusion live via Cloudflare Durable Objects pour le control plane et Cloudflare CDN pour les clips audio HLS
- versionner le workspace sur GitLab, deployer le frontend sur Cloudflare Pages et le backend sur le serveur personnel Traefik via CI/CD
- relayer les evenements pertinents sur Telegram
- permettre un scenario de test declenche depuis le client Web et suivi pas a pas via un flux WebSocket de debug
- survivre aux erreurs transitoires, aux changements de DOM et aux ralentissements amont

## Etat actuel

Refonte locale des 16–17 septembre 2026 :
[EPIC-039 — Refonte PWA avec gluestack v5](./epics/EPIC-039-refonte-pwa-gluestack.md)
est **terminé localement** (NEWS-227 à NEWS-237). Le socle UniWind / Tailwind CSS 4 remplace NativeWind ; toutes les
routes utilisent les thèmes et composants partagés. Le [référentiel PWA](./design/pwa/README.md)
conserve les cinq maquettes validées. Le [guide technique](./guides/pwa-ui-stack.md)
décrit les tokens, la navigation, le thème local et la reproduction des contrôles.
Les résultats sont des preuves locales, sans commit, push ni déploiement.

Livraison locale des pages légales du 12 septembre 2026 : quatre documents
publics clair/sombre, liens dans l’authentification et le profil PWA, notice
avant inscription et contrôle strict des cinq informations de l’éditeur.
Les publications landing et PWA attendent la validation légale, sans changement
des déclencheurs manuels. Voir le [manifeste Fil Finance](./guides/legal-landing-app/MANIFESTE.md)
et la [recette locale](./guides/legal-landing-app/RECETTE-FILFINANCE.md) :
30 tests landing, 12 tests légaux PWA, 32 rendus documentaires, 16 rendus PWA
avec API simulée et 12 audits Lighthouse réussis. Aucun déploiement ; la recette
native et la vérification de l’environnement publié restent distinctes.

Livraison locale landing du 12 septembre 2026 :
[EPIC-038](./epics/EPIC-038-refonte-visuelle-landing-themes-clair-sombre.md)
est terminé, avec NEWS-222 à NEWS-226. Les quatre services, les bandes audio
pleine largeur et les deux thèmes sont implémentés. Validation : 15 tests,
80 contrôles navigateur et 12 audits Lighthouse à 100 dans les quatre
catégories. Voir la [recette et les captures](./design/landing/recette/2026-09-12/README.md).
Preuves locales uniquement, aucun déploiement.

Historique — jalon documentaire initial du 12 septembre 2026 :
[EPIC-038 — Refonte visuelle et thèmes clair/sombre](./epics/EPIC-038-refonte-visuelle-landing-themes-clair-sombre.md)
est planifié. Les [palettes, textes et deux maquettes retenues](./design/landing/README.md)
préparent une landing à quatre services, sans section gamma, claire à la
première visite avec choix clair/sombre mémorisé. NEWS-222 est terminé après
vérification documentaire ; NEWS-223 à NEWS-226 restent à faire. Ce jalon ne constitue ni
une implémentation frontend ni une validation de déploiement.

Derniere mise a jour calendrier (EPIC-037): 7 septembre 2026.

Livraison suivante : `dots-studio/dots-3-note-preview:free` confirmé, endpoint
AtlasCloud gratuit et JSON Schema strict annoncés. Suppression de l'exception
JSON simple ; migration via CI uniquement, sans relance calendrier ni action DEV.
Le run du 7 septembre à 17:56Z a confirmé un HTTP/code fournisseur 404 : MiniMax
M3 free indisponible, catalogue sans endpoint. Aucun produit créé ; les smokes
admin/WebSocket/HLS et la continuité live ont été vérifiés après ce rejet.

Historique de la release #363 (`1972d5a`) : modèle `minimax/minimax-m3:free` et logs serveur
OpenRouter avec code numérique, message et corps d'erreur borné, credentials
masqués. Mode `json_object` explicitement confirmé pour ce modèle exact (le
fournisseur n'annonce pas JSON Schema strict), validations Rust inchangées et
aucun fallback silencieux. 370 tests Rust passants, récupération native et fmt
validés ; livraison prévue via CI uniquement. La relance unique de 16:27Z a
confirmé `open_router_embedded_error`, sans données produit ni impact live.
Pas de nouvelle admission calendrier autorisée par cette livraison.

Release PROD observee : #359 sur `8aaeb24`, 18 jobs verts et smokes valides.
Rollback : `prod-before-calendar-source-cleanup-20260907` sur `1b670c4`.
Les references #350 ci-dessous documentent la livraison initiale Rust-only,
pas le dernier SHA deploye. L'observation interne prolongee de #353 reste
distincte et non validee ; rollback disponible sur `808a177`.

La couverture ForexFactory et le schema strict sont livres par #355.
La release #359 livre le retrait complet des sources legacy dans
Rust, la PWA et le script d'observation ; diagnostic OpenRouter distinguant
classe d'erreur et statut HTTP sans contenu sensible. La regle reste ForexFactory JSON
couvrante obligatoire, sept flux facultatifs toujours examines et evenements
complementaires autorises. Les jours hors export de semaine courante restent
en erreur explicite, sans substitution par une autre source. Diagnostics types
atomiques exposes dans l'admin/WebSocket (classe et statut HTTP OpenRouter sans
contenu), JSON Schema OpenRouter strict et
controle de fin de generation, sans retry ajoute ni changement du resume.
Une sonde isolee opt-in permet de comparer les contrats sans donnee produit ;
la cause historique du rejet n'est pas attribuee sans reproduction. Une seule
admission produit du 7 septembre via CI a ete observee : ForexFactory JSON et
Eurostat couvrantes, puis `open_router_invalid_response`, sans item/resume/audio.
Le live continue. Aucune nouvelle admission apres cet echec. Le correctif
suivant precise la phase du rejet et la famille des retries epuises, sans changer
modele, limites, validations ou contenu journalise.
Validation locale : 364 tests Rust passants, 0 echec, 3 ignores ; fmt et
recuperation valides. La livraison passe exclusivement par le pipeline GitLab
sur main : aucun transfert de sources ni mise a jour manuelle du serveur.
La sonde serveur reste non executee ; son integration CI est differee.

Le socle produit, le backend, l'audio et le parcours Better Auth sont livres dans
`main` au commit `8e6b5e4`. La migration monorepo est maintenant commitee sur
`dev` : `9164900` deplace l'ancien `filfinance-client` vers `apps/pwa`, ajoute
la landing Astro dans `apps/landing` et finalise le pipeline de release a deux
frontends. Le bootstrap dev de ce pipeline a ensuite ete durci par `1e9a49d`.

Les validations locales de ce jalon sont vertes : QA/typecheck PWA, typecheck,
build et validation de production de la landing, 22 tests Better Auth et 269
tests Rust. Les documents de reference (`README`, architecture, environnement,
GitLab et auth) sont alignees sur cette organisation. Les validations externes
restent distinctes : provisionnement Cloudflare, DNS/domaines, secrets CI,
origines Better Auth et smokes sur les environnements deployes.

Le workspace est maintenant organise ainsi:

- `filfinance-service` pour le backend Rust Fil Finance
- `apps/landing` pour la landing Astro publique
- `apps/pwa` pour l'app Expo/React `filfinance`
- `docs/` pour la documentation partagee service plus client

Etat reel du backend Fil Finance:

- runtime `tokio` et API `axum` en place
- endpoints `/health`, `/api/sources`, `/api/news`, `/api/news/{id}`, `/ws/news` et `/ws/test-runs`
- endpoint `/api/translations` pour l'historique traduit EN+FR separe entre feed reel et `test_run`, avec textes complets en HTTP
- endpoint `/api/translations/{id}` pour charger le detail complet d'une traduction unique
- persistence SQLite `WAL` avec migrations idempotentes, `news_events` et `news_translations`
- tri historique et replay recent bases sur `published_at` avec fallback explicite
- collecteur `FinancialJuice` primaire via session web auth, `GetPreviousNews` et live SignalR
- champ `Level` FinancialJuice normalise en `news_level`, persiste sur `news_events` et expose aux contrats HTTP/WebSocket
- collecteur `FinancialJuice` RSS legacy supprime du backend; le chemin nominal est session web auth + API interne + SignalR
- blocage `FinancialJuice` par `Cloudflare Managed Challenge` diagnostique: le backend detecte le cas, backoff proprement, et `EPIC-025` formalise la strategie MVP navigateur separe puis acces officiel v2
- ancien calendrier `FinancialJuice` legacy retire; `EPIC-037` a construit sur
  `main` le chemin calendrier Rust canonique autour de `calendar_day_*`. Le test
  de recuperation Rust unique a ete valide localement et les artefacts calendrier
  n8n sont retires du depot et de la CI, avec les commits pousses sur `main`.
  La pipeline web #350 sur `808a177` termine ses 17 jobs en succes, dont la
  recuperation au premier essai (budget live interne 1 s), les deploiements
  backend/landing/PWA/edge et les smokes PROD. OVH execute ce SHA Rust-only,
  sans conteneur n8n ; rollback `prod-before-calendar-rust-20260905` (`67b48bb`),
  volumes historiques preserves. Les comptes smoke PROD sont crees, scopes et
  testes ; DEV reste inchange. Audits calendrier HTTP/WebSocket admin et lecture
  HLS Bunny valides. Les refus 403 des deux sources restent un etat calendrier
  degrade accepte. Observation de 30 minutes validee de 09:16:21 a 09:46:24 UTC :
  30 echantillons sains, aucune anomalie critique, persistance live en progression.
- ancien collecteur macro externe retire du scope produit et du code runtime
- pipeline async avec dedup, diffusion WebSocket, relay Telegram et traces de run E2E
- flux user-facing unifie EN+FR: Telegram et WebSocket consomment le meme objet traduit
- traduction locale `OPUS-MT tc-big en-fr` branchee en vrai via helper Python `CTranslate2`, mode session long-vivant, warmup, benchmark, endpoint runtime et fallback heuristique
- glossaire metier `EN -> FR affiche -> FR vocal` pilotable, avec motifs `<number>` et `<string>` sans ponctuation terminale, seed `ECB's <string>` -> `<string> de la BCE`, support de `<string>:` -> `<string> :`, et action admin `Update glossaire` pour retraduire les news `financial_juice` presentes dans la table retenue J/J-1
- worker TTS serveur Rust branche via provider `piper_local_session`, warmup, normalisation de narration, benchmark, endpoint runtime et publication de clips HLS finalises via `audio_ready`
- process de test E2E declenchable depuis le client via `/ws/test-runs`, avec injection dans le pipeline canonique et trace pas a pas

Etat reel de l'app `filfinance`:

- client Expo/React autonome, separe du service et de `deltalytix-main`
- connexion au flux user-facing `/ws/news` et au flux de debug `/ws/test-runs`
- feed utilisateur `Fil d'actualités` en francais: titre toujours visible, contenu FR replie par defaut, chevron discret, aucun texte anglais source
- historique admin `Historique actualités` avec blocs anglais source, francais traduit, TTS et badge utilisateur en deuxieme ligne a gauche: `active -> Élevée`, `news-general -> Général` ou `none -> Faible`; les badges affichent le terme source et le libelle utilisateur applique
- admin blacklist avec portee `Titre`, `Contenu` ou `Titre + contenu`, audit `reject_news`/`strip_content` et bouton `Update Blacklist` pour appliquer les regles actives aux news `financial_juice` deja stockees dans la table retenue J/J-1 sans broadcast live
- calendrier `Calendrier hebdo` avec bloc `FILTRES` unifie, jours puis bouton filtres, filtres partages entre prochains evenements et evenements fusionnes, type/localisation/importance synchronises
- historique admin calendrier `/admin/calendar-history` sans bloc prochains evenements, avec details complets, audios et alertes par evenement
- lecture web des clips HLS finalises annonces par `audio_ready`, sans WebSocket audio binaire dedie
- historique charge sans autoplay, utilise comme base de dedup visuelle pour le live
- lecture automatique reservee aux vraies news live nouvelles
- queue audio pilotee par les evenements `audio_ready` et replay journalier HLS separe
- invariant FIFO audio durci: une news plus ancienne que la reference audio courante reste visible dans l'historique mais n'entre pas dans la file audio
- pas de fallback audio natif: le MVP audio cible reste le client web
- bouton de test E2E et panneau de trace
- historique journalier separe entre feed reel et `test_run`
- badges utilisateur historiques `active -> Élevée`, `news-general -> Général` et `none -> Faible` derives de `news_level`
- file de lecture reservee aux news live arrivees pendant une lecture deja en cours, avec admission monotone par timestamp et tri du plus ancien au plus recent
- branding client et metadata app alignes sur `Fil Finance` / `filfinance`
- authentification Better Auth active dans le code, profils et preferences dans PostgreSQL
- side menu mobile-app avec theme ancre en bas, page profil, page tests role-based `moderator/admin`, page admin `Supervision admin`
- theme clair/sombre avec palette sombre type Codex, switch en bas de sidebar et synchronisation `preferences.theme`

Les priorites effectives sont maintenant:

1. maintenir les executions et observations DEV en pause ; elles ne sont pas
   une gate de `EPIC-037`;
2. maintenir et revoir sur `main` le parcours calendrier Rust : les deux
   collecteurs autorises, IA typee, file SQLite durable, Piper/HLS/alertes et
   protection du live news ;
3. conserver le retrait n8n deja realise dans le depot et la CI, sans purger les
   champs `n8n_*` nullables ni les volumes historiques ;
4. conserver la release #350 validee sur `808a177`, ses 30 minutes d'observation,
   le tag rollback exact `prod-before-calendar-rust-20260905`, les preuves
   redigees et les volumes. #339 reste exclue ; DEV et parite n8n exclus ;
5. reprendre les chantiers produit restants : FinancialJuice, observabilite,
   qualite traduction/TTS et capacite edge.

Le travail produit restant porte ensuite surtout sur:

- la validation navigateur/mobile et le durcissement final de la couche audio
- la remise a niveau Docker de la traduction OPUS-MT, puis le level-up qualite/perf traduction et TTS sans changer la voix `fr_FR-upmc-medium-jessica`
- la validation live de l'inventaire des valeurs `Level` FinancialJuice maintenant exposees en `news_level`
- la strategie d'acces `FinancialJuice` si Cloudflare continue de challenger le backend: collector navigateur isole en MVP v1, puis demande d'acces officiel en v2
- l'audit post-migration branding pour verifier que les anciennes references produit ne reviennent pas
- la bascule dev puis prod Better Auth/PostgreSQL et la validation des smokes HTTP/WebSocket
- la refonte broadcast en cours: Durable Objects pour metadata, HLS/CDN pour clips audio live evenementiels et replay du jour si le serveur personnel devient insuffisant
- la finalisation de `EPIC-028` avec `filfinance-prod` (landing `filfinance.fr`)
  et `filfinance-app-prod` (PWA `app.filfinance.fr`), apres levee normale du
  blocage Cloudflare Pages;
- la topologie DEV en pause : `filfinance-dev` (landing `dev.filfinance.fr`) et
  Bunny Storage (PWA `app.dev.filfinance.fr`), avec
  `FILFINANCE_DEV_PWA_URL=https://app.dev.filfinance.fr`;
- la conservation des projets legacy `fil-finance` et `filfinance-pwa` jusqu'a
  l'activation des quatre domaines et aux smokes explicitement autorises; aucun
  contournement du refus anti-abus Cloudflare.
- le durcissement observabilite, delivery et exploitation
- l'ajout d'un glossaire metier pilotable pour maitriser `EN -> FR -> vocal`
- la correction des cas ou le theme frontend n'est pas applique uniformement
- la restauration du catch-up historique journalier edge, sans coupler historique et audio
- la migration du live audio hors HLS permanent vers des clips HLS finalises par news, afin de supprimer les requetes HLS en idle
- la refonte architecturelle du frontend vers `expo-router` et une structure feature-first inspiree de `maroutine`, sans changement visuel
- le suivi du risque source accepte apres `EPIC-037`, livre et observe via #350 :
  les refus 403 des deux sources empechent actuellement la production calendrier.
  Remplacement implemente et teste localement sur `main` par ForexFactory export JSON, BEA
  JSON/ICS, Fed FOMC/RSS, Eurostat ICS et BCE statistiques. Huit flux bornes,
  contexte RSS distinct des evenements, verification de provenance avant SQLite ;
  358 tests Rust passants et 9 tests Node, huit endpoints HTTP 200 sondes depuis
  le poste local ; cette evolution ne constitue pas encore une nouvelle livraison PROD.
  Tout acces autorise futur reste distinct de la gate Rust deja validee,
  sans contournement, reference de resultat n8n ou campagne multi-jours

## Snapshot historique d'avancement au 4 mai 2026

Les tableaux et priorites ci-dessous conservent l'historique des EPICs. Ils ne
remplacent pas les priorites actuelles ci-dessus.

| EPIC | Statut reel | Commentaire |
| --- | --- | --- |
| EPIC-010 | Fait, rattrapage cible en cours | Le standard pedagogique Rust est ecrit et la review future est verrouillee; `NEWS-168` a re-audite l'arborescence Rust actuelle et doit rattraper les modules recents a risque. |
| EPIC-008 | Fait, extension glossaire a faire | Le runtime local `OPUS-MT tc-big en-fr`, le worker, la persistence EN+FR, l'observabilite runtime, Docker et les profils qualite/perf sont livres. `NEWS-132/133` ajoutent maintenant le glossaire metier. |
| EPIC-009 | Fait | Le socle Rust TTS, la normalisation de narration et l'observabilite runtime sont livres; la diffusion live cible est maintenant portee par EPIC-021. |
| EPIC-012 | En cours avance | La migration produit vers `Piper` est codee dans le repo et le runtime Docker Piper est package. `NEWS-128` a livre une v1 de diction finance mais elle est reprise par `NEWS-195`, sans changement de voix; `NEWS-130` dimensionne les pools si les mesures le justifient. |
| EPIC-007 | Supersede audio | Le flux texte EN+FR, `/api/translations` et le process de test E2E restent valides. Son transport audio binaire historique est remplace par EPIC-021. |
| EPIC-011 | En cours avance | `FinancialJuice` n'utilise plus le RSS: login `/home`, `GetPreviousNews`, live SignalR et calendrier separe sont en place. Le collecteur RSS legacy a ete supprime du backend. |
| EPIC-013 | Fait, corrections UX a faire | `App.tsx` est un bootstrap fin, le client est segmente en `src/app`, `src/features`, `src/shared`, les contrats/helpers/UI sont extraits, l'ecran principal est migre en `className` NativeWind et `NEWS-066` cloture la non-regression. `NEWS-114` ajoute le side menu/pages role-based; `NEWS-115` a `NEWS-119` livrent le theme clair/sombre et sa synchronisation profil. `NEWS-134/135` ajoutent la page glossaire et corrigent les cas de theme non applique. |
| EPIC-015 | En cours avance | Le champ source FinancialJuice `Level` est capture en `news_level`, persiste, expose dans les payloads et rendu visible cote client. La validation live de l'inventaire des valeurs reste a faire. |
| EPIC-016 | En cours avance | Le branding produit bascule vers `Fil Finance` et l'app vers `filfinance`; les sources/configs principales sont alignees avec compatibilite transitoire. |
| EPIC-014 | Historique, supersede | Le premier palier auth/session/JWT a protege HTTP/WebSocket et livre les profils/preferences minimales. Il est remplace par `EPIC-036`. |
| EPIC-017 | Historique, supersede | Le palier hybride profils externes a ete valide comme transition, puis remplace par le palier Logto + SQLite de `EPIC-036`, lui-meme abandonne au profit de Better Auth + PostgreSQL. |
| EPIC-036 | Remplace | La cible Logto + SQLite est abandonnee au profit de Better Auth + PostgreSQL. |
| EPIC-018 | Demarre, catch-up historique restaure | Le fan-out metadata doit passer par Cloudflare Durable Objects; `NEWS-093` livre le contrat metadata edge, `NEWS-094` livre le Worker/Durable Object live, `NEWS-095` branche le publisher Rust optionnel, `NEWS-096` ajoute le client edge avec fallback backend, `NEWS-097` protege `/ws/live` par ticket court et `NEWS-098` prepare le load test 1000 clients, sans audio binaire. `NEWS-143` restaure le catch-up des `translated_news_item` pour l'historique journalier. `EPIC-021` etend ce control plane avec `audio_ready`, `audio_failed` et `audio_finalized`, toujours sans audio binaire et sans lien avec le refresh historique. |
| EPIC-019 | Transitoire, supersede pour le live | L'audio HLS permanent a livre un origin Rust, des headers CDN, un replay du jour et un player client, mais le modele radio live permanent est abandonne comme cible nominale apres les problemes de request storm et d'idle. `NEWS-136` reste une mitigation court terme; `EPIC-021` devient la cible live. |
| EPIC-021 | Valide | Le live audio cible est livre: `pub/sub` leger + clips HLS finalises par news, publication `audio_ready`, controle audio global, suppression de la radio HLS legacy et validation produit du modele sans polling HLS en idle. |
| EPIC-022 | Nouveau, prototype frontend | La refonte cible l'architecture du client, pas le visuel: adoption `expo-router`, features `components/hooks/services/types/screens/index.ts`, aliases publics et stabilisation du theme clair/sombre actuel. Les tickets utilisent `NEWS-144` a `NEWS-150`, car `NEWS-143` est deja reserve au catch-up historique journalier edge. |
| EPIC-023 | Nouveau, admin runtime | Ajouter des surfaces admin pour gerer le profil de traduction et consulter les logs backend recents. `NEWS-151` couvre le profil de traduction, `NEWS-152` couvre la rotation J/J-1 des logs et le viewer admin. |
| EPIC-024 | A finir | Remplacer la consommation audio directe du broadcast par une vraie queue persistante `audio_jobs`, pour absorber les rafales de news et garantir un statut observable par traduction. |
| EPIC-025 | Mail autorisation a envoyer | Strategie en deux temps: `NEWS-154` documente un collector navigateur `Playwright`/`Chromium` separe pour le MVP v1; `NEWS-155` porte maintenant le mail de demande d'autorisation officielle `FinancialJuice`. |
| EPIC-028 | En cours, release pipeline livre | Production `main` : landing `filfinance.fr`, PWA `app.filfinance.fr`, API et edge. Le pipeline a deux frontends est committe sur `dev`; le provisioning, l'alignement Better Auth et les smokes deploiement restent a valider. |
| EPIC-029 | En cours, dev a deux surfaces | Le bootstrap CI dev landing/PWA/edge est livre; `dev` cible `dev.filfinance.fr` et une PWA definie par `FILFINANCE_DEV_PWA_URL`, sans URL implicite dans le depot. Les ressources externes restent a provisionner et valider. |
| EPIC-034 | Nouveau, mitigation Cloudflare | Retirer le data plane HLS du CDN Cloudflare standard: fermer `/hls/audio/...` par defaut, vider la base HLS Pages et bloquer les tests de charge HLS vers Cloudflare sans opt-in explicite. |
| EPIC-035 | Pilote DEV Bunny | Sous-zone Bunny déléguée, Storage landing/PWA, API/HLS/live via Pull Zones et fan-out Rust/Redis; production Cloudflare inchangée. |
| EPIC-037 | Livre et valide en PROD | #350 sur `808a177` : 17 jobs verts, recuperation sans retry, backend/Bunny/edge et smokes livres. Audits HTTP/WS admin et HLS Bunny valides. Observation 30 minutes reussie, aucun incident critique. Refus sources 403 accepte, aucun n8n OVH, volumes preserves ; rollback exact `prod-before-calendar-rust-20260905`. |
| EPIC-020 | Fait pour le palier actuel | Le projet est versionne sur GitLab, les pipelines GitLab/Cloudflare sont valides, le frontend Cloudflare Pages provisoire recupere les news live, la route Traefik API backend est appliquee et le backend serveur est valide live. Le domaine `filfinance.sbcenter.fr` est abandonne; la prod finale passera par `filfinance.fr` avec le runbook dedie. |
| EPIC-001 | Largement livre | Le socle Rust, la config, le modele principal, les migrations et le cycle de vie global existent et sont utilises. |
| EPIC-002 | Depasse par EPIC-011 | Le premier lot `FinancialJuice` RSS/fallback est conserve seulement comme historique documentaire; le backend ne contient plus ce chemin legacy. |
| EPIC-003 | Largement livre | Historique HTTP, WebSocket, Telegram, replay et contrats de debug sont deja exploites. |
| EPIC-005 | A finir | Heartbeat, lag, traces et logs existent, mais l'observabilite reste incomplete et non consolidee. |
| EPIC-006 | A finir | Le runbook transverse, la livraison et l'exploitation doivent etre finalises. |

## Rattrapages transverses actifs

- `NEWS-168`: re-audit `EPIC-010` sur l'arborescence Rust actuelle fait dans `NEWS-047`; prochaine campagne de commentaires ciblee sur `health`, `logs/tests`, `HLS`, `edge`, `ingestion`, `audio` et `live`.

## Evolutions techniques non prioritaires

- `NEWS-153`: documente un passage possible en Cargo workspace pour isoler les dependances lourdes `audio` et `translation`. Decision actuelle: ne pas lancer tant que le single crate feature-first reste suffisant.
- [`EPIC-033`](./epics/EPIC-033-audio-live-intro-immediate-corps-finalise.md): optimiser plus tard le live audio pour jouer l'intro immediatement, puis attendre et enchainer proprement le corps HLS finalise.

## Decision de priorisation historique

La liste suivante etait la priorite de developpement au 4 mai 2026 :

1. `NEWS-137`: figer le contrat `audio_ready` et clip HLS finalise.
2. `NEWS-138`: produire l'origin HLS clip progressif compose intro + corps.
3. `NEWS-139`: finaliser la publication audio vers le control plane et la reconnexion par watermark.
4. `NEWS-140`: valider navigateur le controle audio global et la queue HLS par timestamp metier.
5. `NEWS-141`: valider la suppression runtime de la radio HLS permanente.
6. `NEWS-142`: ajouter observabilite, smokes et validation latence/idle.
7. `NEWS-195`: reprendre la diction TTS finance v2 en conservant `fr_FR-upmc-medium-jessica`.
8. `NEWS-132`: ajouter le CRUD backend du glossaire metier.
9. `NEWS-133`: brancher le glossaire dans le pipeline traduction et TTS.
10. `NEWS-135`: corriger les cas ou le theme frontend n'est pas applique uniformement.
11. `NEWS-134`: ajouter la page operateur du glossaire.
12. `NEWS-130`: ajouter les pools traduction/TTS seulement si les mesures justifient le besoin.
13. `NEWS-144` a `NEWS-150`: refondre l'architecture frontend en prototype `expo-router` sans changer le style FilFinance existant.

Les autres EPICs restent valides mais repassent derriere ce bloc prioritaire immediat:

- `EPIC-005` et `EPIC-006` restent des chantiers de consolidation a finir.
- si un ajustement de socle est necessaire pour debloquer une priorite immediate, il doit etre traite comme prerequis direct de cette priorite et non comme un chantier separe.

## Principes d'architecture

- Garder un service autonome, sans dependance runtime a `deltalytix-main` ou `deltalytix-api-main`.
- Garder aussi une app autonome `filfinance`, dans le dossier `apps/pwa`, distincte du service et de `deltalytix-main`.
- Prioriser `FinancialJuice`; l'integration de deuxieme source macro est sortie du scope.
- Concevoir le pipeline autour d'un modele interne unique `NewsEvent`, puis projeter ce modele vers la base, Telegram, le flux texte EN+FR et le flux audio.
- Exiger qu'un scenario de test E2E passe par le meme pipeline canonique que la production, sans envoi direct special-case vers Telegram, le flux texte user-facing ou le flux audio.
- Produire la traduction FR localement dans le service ou dans un runtime local pilote par le service, sans dependance a un service de traduction distant pour le MVP.
- Produire la TTS cote serveur sans ecriture systematique de fichiers audio sur disque.
- Isoler le hot path d'ingestion des sorties lentes avec des channels bornes, du retry et de la backpressure explicite.
- Garder SQLite comme stockage local du pipeline news/audio et support de debug.
- Utiliser PostgreSQL `app.profiles` comme source de verite des profils, roles et preferences utilisateur.
- Ne pas migrer `news_events` ou `news_translations` vers un stockage externe tant que le hot path local reste plus simple et plus robuste.
- Utiliser Durable Objects uniquement pour le control plane metadata: news cards, `now_playing`, evenements `audio_ready`, etat audio et notifications.
- Utiliser HLS/CDN pour le data plane audio: clips live finis par news et replay du jour si la valeur produit est confirmee.
- Ne pas creer de clips replay unitaires historiques et ne pas conserver d'historique replay multi-jours.
- Garder GitLab comme pipeline maitre pour les gates de qualite avant de pousser vers le serveur personnel.
- Documenter des tests d'acceptation a chaque phase pour eviter un service "qui tourne" mais non fiable.
- Considerer le code Rust comme un support d'apprentissage: les commentaires doivent expliquer le "quoi", le "pourquoi" et la specificite Rust mobilisee.

## Consigne permanente de developpement

Cette consigne s'applique a toute la suite du developpement, quel que soit l'EPIC ou le ticket. Elle est detaillee dans [RUST-COMMENTING-STANDARD](./RUST-COMMENTING-STANDARD.md).

- commenter le code en francais
- commenter tous les ajouts, toutes les modifications et toute logique nouvelle ou modifiee
- pour tout code Rust de `filfinance-service`, commenter chaque ligne de code
- expliquer au fil du code les specificites et mots-cles Rust utilises
- expliquer les choix et bonnes pratiques d'architecture du projet directement dans les modules et fonctions concernes
- appliquer la meme exigence pedagogique aux tests Rust
- ne pas considerer un ticket Rust comme done si la couverture de commentaires pedagogiques n'est pas complete et coherente
- appliquer aussi la checklist [RUST-REVIEW-CHECKLIST](./RUST-REVIEW-CHECKLIST.md) a toute review Rust et a toute cloture de ticket Rust

L'objectif est d'obtenir un codebase Rust qui reste executable, maintenable et formateur pour de futurs developpeurs non experts Rust.

## Phases de livraison

L'ordre des phases prime sur la numerotation des EPICs. `EPIC-010` a ete traite en premier, puis `EPIC-008`, puis `EPIC-009`, puis `EPIC-007` restent devant les EPICs `EPIC-001` a `EPIC-006`.

| Phase | EPIC | Resultat attendu | Done de phase | Statut au 4 mai 2026 | Lien |
| --- | --- | --- | --- | --- | --- |
| 0 | EPIC-010 | Standard de commentaires Rust pedagogiques | Regle projet ecrite, code Rust existant couvert pedagogiquement, et garde-fous de review formalises pour tous les futurs tickets Rust | Fait, rattrapage `NEWS-168` en cours | [EPIC-010](./epics/EPIC-010-commentaires-rust-pedagogiques.md) |
| 1 | EPIC-008 | Traduction locale EN vers FR via OPUS-MT | Traduction locale stable, persistente et observable pour les headlines MVP | Fait | [EPIC-008](./epics/EPIC-008-traduction-locale-opus-mt-local.md) |
| 2 | EPIC-009 | TTS serveur pilote par Rust | Narration FR transformable en audio streamable en memoire, sans fichiers | Fait | [EPIC-009](./epics/EPIC-009-tts-serveur-rust.md) |
| 3 | EPIC-012 | Migration TTS serveur vers Piper | Provider TTS nominal `Piper` branche, catalogue FR local et scripts de bootstrap en place | En cours avance | [EPIC-012](./epics/EPIC-012-migration-tts-piper.md) |
| 4 | EPIC-007 | Streaming audio FR temps reel en memoire via WebSocket | Flux texte EN+FR, streaming audio binaire et process de test E2E operationnels en MVP | En cours avance | [EPIC-007](./epics/EPIC-007-streaming-audio-fr-temps-reel-websocket.md) |
| 5 | EPIC-001 | Socle Rust coherent, modele stable, config et persistence pretes | Runtime, config, modele et bootstrap de stockage verrouilles | Largement livre | [EPIC-001](./epics/EPIC-001-socle-rust.md) |
| 6 | EPIC-011 | `FinancialJuice` via session web + API interne + SignalR | Premiere source temps reel fiable sans dependre du RSS rate-limited | En cours avance | [EPIC-011](./epics/EPIC-011-refonte-acquisition-financialjuice.md) |
| 7 | EPIC-003 | Flux persiste et diffuse proprement | Requetes historiques, WebSocket et Telegram utilisables en MVP | Largement livre | [EPIC-003](./epics/EPIC-003-persistence-diffusion-mvp.md) |
| 8 | EPIC-005 | Service robuste et observable | Pannes transitoires, anti-bot, lag et invalides geres proprement | A finir | [EPIC-005](./epics/EPIC-005-fiabilite-observabilite.md) |
| 9 | EPIC-006 | Livraison et exploitation encadrees | Docker, runbook, scenarios E2E et checklist release finalises | A finir | [EPIC-006](./epics/EPIC-006-delivery-operations.md) |
| 11 | EPIC-013 | Refonte frontend feature-based et NativeWind v4 | Client Expo segmente en `app / features / shared`, services extraits et styles factorises via `NativeWind v4` | Fait, extension livree | [EPIC-013](./epics/EPIC-013-refonte-frontend-feature-based-nativewind.md) |
| 12 | EPIC-015 | Niveau FinancialJuice des news | Champ `Level` capture, persiste, expose en `news_level` et exploitable cote client | En cours avance | [EPIC-015](./epics/EPIC-015-financialjuice-news-level.md) |
| 13 | EPIC-016 | Branding Fil Finance | Client, manifests, configs et docs principales alignes sur `Fil Finance` / `filfinance` | En cours avance | [EPIC-016](./epics/EPIC-016-branding-fil-finance.md) |
| 14 | EPIC-014 | Authentification legacy | Session utilisateur, JWT verifies cote Rust pour HTTP/WebSocket, profils/preferences et runbook disponibles | Historique, supersede | EPIC historique |
| 15 | EPIC-017 | Stockage hybride profils legacy | Profils, roles et preferences dans un store externe; pipeline news/audio conserve en SQLite local | Historique, supersede | EPIC historique |
| 36 | EPIC-036 | Ancienne migration auth | Palier historique remplace par Better Auth, JWT RS256 et profils PostgreSQL | Remplace | [EPIC-036](./epics/EPIC-036-migration-logto-sqlite.md) |
| 16 | EPIC-020 | GitLab, Cloudflare Pages et pipeline serveur | Versioning GitLab, validation CI, frontend Cloudflare Pages et backend serveur personnel Traefik | Fait pour palier actuel | [EPIC-020](./epics/EPIC-020-gitlab-pipeline-cloudflare.md) |
| 17 | EPIC-018 | Metadata live edge Durable Objects | Fan-out metadata porte par Cloudflare Durable Objects, sans audio binaire | Demarre | [EPIC-018](./epics/EPIC-018-metadata-live-edge-durable-objects.md) |
| 18 | EPIC-019 | Audio HLS live et replay du jour | Origin HLS et replay du jour livres comme palier transitoire; modele live permanent supersede | Transitoire | [EPIC-019](./epics/EPIC-019-audio-hls-live-replay-du-jour.md) |
| 19 | EPIC-021 | Audio live evenementiel par clips HLS progressifs | Pub/sub leger, clips HLS finalises par news, player global et zero requete HLS en idle | Valide | [EPIC-021](./epics/EPIC-021-audio-live-evenementiel-clips-hls-progressifs.md) |
| 20 | EPIC-022 | Refonte architecture frontend FilFinance | Prototype `expo-router`, features explicites et theme actuel stabilise sans changement visuel | Nouveau | [EPIC-022](./epics/EPIC-022-refonte-architecture-frontend-filfinance.md) |
| 21 | EPIC-025 | Strategie d'acces FinancialJuice | Collector navigateur separe possible pour le MVP v1, puis demande d'acces officiel pour la prod v2 | Nouveau | [EPIC-025](./epics/EPIC-025-financialjuice-access-strategy.md) |
| 22 | EPIC-026 | Securite et charge production | Securite App, Worker Edge, HLS, health minimal et plan de charge Durable Objects/CDN | En attente | [EPIC-026](./epics/EPIC-026-securite-et-charge-production.md) |
| 23 | EPIC-027 | Cache court playlists HLS Worker | Cache Worker 1-2s des playlists avec `hls_ticket` dans la cache key pour preparer 1000 auditeurs | Nouveau | [EPIC-027](./epics/EPIC-027-cache-playlists-hls-worker.md) |
| 24 | EPIC-034 | Retrait HLS du CDN Cloudflare standard | Worker Cloudflare limite au control plane; data plane HLS coupe et garde de charge active | Nouveau | [EPIC-034](./epics/EPIC-034-retrait-hls-cloudflare-cdn-standard.md) |
| 25 | EPIC-035 | Migration HLS Bunny CDN | Pull Zones Bunny dev/prod, resolver `/audio/...`, garde de charge Bunny et custom domains DNS-only | En promotion prod | [EPIC-035](./epics/EPIC-035-migration-hls-dev-bunny-cdn.md) |

## Tableau des EPICs

| EPIC | Theme | Priorite | Depend de | Statut reel | Tickets |
| --- | --- | --- | --- | --- | --- |
| [EPIC-039](./epics/EPIC-039-refonte-pwa-gluestack.md) | Refonte PWA, thèmes et composants gluestack | Livré localement | Charte EPIC-038, Expo, gluestack v5 / UniWind / Tailwind CSS 4 et contrats existants | Terminé localement ; recette et captures archivées | `NEWS-227` à `NEWS-237` |
| [EPIC-038](./epics/EPIC-038-refonte-visuelle-landing-themes-clair-sombre.md) | Refonte visuelle landing, clair/sombre | Livrée localement | Socle Astro et destinations PWA existants | Terminé ; 15 tests, 80 contrôles et 12 audits validés, aucun déploiement | `NEWS-222` à `NEWS-226` |
| EPIC-010 | Commentaires Rust pedagogiques | Priorite absolue | Etat courant du service | Fait, rattrapage cible en cours | `NEWS-046` a `NEWS-050`, `NEWS-168` |
| EPIC-008 | Traduction locale EN vers FR via OPUS-MT | Priorite absolue | Etat courant du service | Fait, extension glossaire | `NEWS-032` a `NEWS-035`, `NEWS-126`, `NEWS-129`, `NEWS-132`, `NEWS-133` |
| EPIC-009 | TTS serveur pilote par Rust | Priorite absolue | EPIC-008 | Fait | `NEWS-036` a `NEWS-039` |
| EPIC-012 | Migration TTS serveur vers Piper | Priorite absolue | EPIC-009 | En cours avance | `NEWS-057` a `NEWS-061`, `NEWS-125`, `NEWS-128`, `NEWS-130`, `NEWS-195` |
| EPIC-007 | Streaming audio FR temps reel en memoire via WebSocket | Priorite absolue | EPIC-008, EPIC-009, EPIC-012 | En cours avance | `NEWS-040` a `NEWS-045`, `NEWS-079` a `NEWS-081` |
| EPIC-001 | Socle Rust | Critique | Aucune | Largement livre | `NEWS-001` a `NEWS-004` |
| EPIC-011 | Refonte acquisition FinancialJuice | Critique | EPIC-001 | En cours avance | `NEWS-051` a `NEWS-056` |
| EPIC-013 | Refonte frontend feature-based et NativeWind v4 | Priorite 3 | EPIC-007, EPIC-012, EPIC-014 | Fait, corrections UX | `NEWS-062` a `NEWS-066`, `NEWS-082`, `NEWS-114` a `NEWS-119`, `NEWS-134`, `NEWS-135` |
| EPIC-015 | Niveau FinancialJuice des news | Haute | EPIC-011, EPIC-003, EPIC-013 | En cours avance | `NEWS-073` a `NEWS-078` |
| EPIC-014 | Authentification legacy | Archive | EPIC-013, EPIC-001 | Supersede par EPIC-036 | `NEWS-067` a `NEWS-072` |
| EPIC-017 | Stockage hybride profils legacy | Archive | EPIC-014 | Supersede par EPIC-036 | `NEWS-088` a `NEWS-092`, `NEWS-120` |
| EPIC-020 | GitLab et pipeline serveur | Priorite 1 | Etat courant du workspace | Fait pour palier actuel | `NEWS-105` a `NEWS-113` |
| EPIC-018 | Metadata live edge Durable Objects | Haute | EPIC-013, EPIC-014, EPIC-017, EPIC-020 | Demarre, extension audio events | `NEWS-093` a `NEWS-098`, support `EPIC-021` |
| EPIC-019 | Audio HLS live et replay du jour | Haute | EPIC-007, EPIC-012, EPIC-018, EPIC-020 | Transitoire, supersede pour le live | `NEWS-099` a `NEWS-104`, `NEWS-125`, `NEWS-127`, `NEWS-131`, `NEWS-136` |
| EPIC-021 | Audio live evenementiel par clips HLS progressifs | Livre | EPIC-012, EPIC-018, EPIC-019, EPIC-020 | Valide | `NEWS-137` a `NEWS-142` |
| EPIC-022 | Refonte architecture frontend FilFinance | Priorite 1 | EPIC-013, EPIC-014, EPIC-021 | Nouveau | `NEWS-144` a `NEWS-150` |
| EPIC-028 | Production landing et PWA | Priorite immediate | EPIC-020, Better Auth | Pipeline livre sur `dev`, provisioning et smoke a valider | `NEWS-176` a `NEWS-178` |
| EPIC-029 | Environnement dev landing et PWA | Priorite immediate | EPIC-028 | Bootstrap CI livre, ressources externes a valider | `NEWS-203` a `NEWS-207` |
| EPIC-025 | Strategie d'acces FinancialJuice | Mail autorisation | EPIC-011, EPIC-005, EPIC-006 | A envoyer | `NEWS-154`, `NEWS-155` |
| EPIC-026 | Securite et charge production | Attente | EPIC-014, EPIC-018, EPIC-021, EPIC-023 | En attente | `NEWS-156` a `NEWS-162` |
| EPIC-027 | Cache court playlists HLS Worker | Priorite charge HLS 1000 | EPIC-026, EPIC-021 | Nouveau | `NEWS-163` a `NEWS-166` |
| EPIC-034 | Retrait HLS Cloudflare standard | Mitigation | EPIC-026, EPIC-027 | Nouveau | `NEWS-199` a `NEWS-202` |
| EPIC-035 | Migration HLS dev Bunny CDN | Dev media | EPIC-034, EPIC-029 | Nouveau | `NEWS-203` a `NEWS-207` |
| EPIC-037 | Orchestrateur calendrier Rust natif | Livre et valide en PROD | EPIC-032, EPIC-024, EPIC-005 | #350 `808a177` entierement verte : recuperation sans retry, budget live 1 s ; backend/Bunny/edge et smokes livres. Audits admin et HLS valides, observation 30 minutes reussie. Sources 403 en erreur redigee acceptee. Aucun n8n OVH, volumes preserves, rollback exact tague ; DEV gele | `NEWS-217` a `NEWS-221` |
| EPIC-003 | Persistence et diffusion MVP | Haute | EPIC-001, EPIC-002 | Largement livre | `NEWS-009` a `NEWS-012` |
| EPIC-005 | Fiabilite et observabilite | Haute | EPIC-002, EPIC-003 | A finir | `NEWS-017` a `NEWS-020` |
| EPIC-006 | Delivery et operations | Haute | Tous les precedents | A finir | `NEWS-021` a `NEWS-024` |

## Dependances inter-EPIC

- `EPIC-010` est maintenant clos et continue de s'appliquer comme standard transversal pour tout le reste du travail Rust.
- `NEWS-046` est fait et rend le standard de commentaires Rust opposable aux prochains tickets.
- `NEWS-047` est fait et fixe l'ordre de couverture dans [NEWS-047-audit-couverture-rust](./tickets/NEWS-047-audit-couverture-rust.md).
- `NEWS-048` est fait sur le socle et consolide la norme dans le code reel.
- `NEWS-050` est fait et impose maintenant une checklist de review et une gate de fermeture sur tout futur ticket Rust via [RUST-REVIEW-CHECKLIST](./RUST-REVIEW-CHECKLIST.md).
- `NEWS-168` a remis `NEWS-047` a jour sur l'arborescence Rust actuelle; il doit maintenant rattraper les modules recents a risque sans reouvrir une campagne globale.
- le bloc `EPIC-008` -> `EPIC-009` -> `EPIC-007` reste la priorite fonctionnelle immediate apres `EPIC-010`.
- tout futur ticket Rust doit appliquer le standard [RUST-COMMENTING-STANDARD](./RUST-COMMENTING-STANDARD.md).
- toute future review Rust doit appliquer la checklist [RUST-REVIEW-CHECKLIST](./RUST-REVIEW-CHECKLIST.md).
- `EPIC-001` est le socle commun. Aucun parseur source ni contrat de sortie ne doit contourner ce socle.
- les travaux residuels de `EPIC-001` necessaires au bloc prioritaire doivent etre traites en support direct de ce bloc, pas comme un blocage sequentiel.
- `EPIC-002` livre la premiere source reelle et verrouille le contrat de normalisation pour les news "headline".
- `EPIC-003` depend de `EPIC-002` pour finaliser les formes de payloads sortants sur base de donnees reelles.
- `EPIC-005` ne doit pas inventer de nouveaux contrats metier; il durcit les contrats etablis par `FinancialJuice`, le pipeline EN+FR et le live audio.
- `EPIC-006` transforme l'ensemble en livraison operable, sans changer le comportement metier de base.
- `EPIC-008` fournit le texte FR canonique qui doit etre consomme a l'identique par Telegram, le flux texte et la TTS; `NEWS-126` remet ce provider OPUS-MT dans Docker et `NEWS-129` ajoutera les profils de qualite/perf.
- `EPIC-009` fournit le contrat audio streamable en memoire que `EPIC-007` transporte vers les clients.
- `EPIC-007` assemble les prerequis texte, TTS, transport audio et process de test sans redefinir la traduction ou le moteur TTS.
- `NEWS-079` a `NEWS-081` completent `EPIC-007` avec l'invariant de file audio FIFO: historique exhaustif visible, mais admission audio monotone par timestamp.
- `EPIC-013` est clos sur son refactor initial et son extension: le client porte maintenant le side menu, les pages role-based et le theme clair/sombre synchronise profil.
- `EPIC-015` stabilise le contrat `news_level` issu de `FinancialJuice Level` avant que l'auth ne protege les surfaces HTTP/WebSocket.
- `EPIC-014` ajoute l'identite, la session et l'autorisation sur les surfaces HTTP/WebSocket; il est maintenant valide en live sans imposer dans ce meme lot une migration du stockage metier hors SQLite.
- `EPIC-017` et `EPIC-036` restent des jalons historiques; la cible active est Better Auth + PostgreSQL.
- `EPIC-020` a livre le palier actuel: GitLab reste la source de versioning et de pipeline pour livrer le frontend Cloudflare Pages et le backend serveur personnel Traefik, puis Cloudflare edge si necessaire.
- `EPIC-018` deplace le fan-out metadata vers Cloudflare Durable Objects et garde le backend Rust comme producteur unique d'evenements; le catch-up historique journalier doit rester porte par `translated_news_item`, pas par les evenements audio.
- `EPIC-019` a livre un palier HLS/CDN utile, mais son modele live global permanent est supersede pour le chemin nominal par `EPIC-021`.
- `EPIC-021` transforme le live audio en clips HLS finalises par news, declenches par `audio_ready`; il reutilise le socle HLS/CDN de `EPIC-019` et le control plane de `EPIC-018`, sans maintenir de playlist HLS active en idle.
- `EPIC-022` reprend l'organisation frontend de `maroutine` pour rendre le client maintenable, tout en preservant le style et les themes existants de FilFinance.
- `EPIC-025` garde l'acquisition `FinancialJuice` propre: le MVP peut utiliser un collector navigateur separe si la source exige un vrai navigateur, mais la production v2 doit viser un acces officiel, licencie ou allowliste directement avec `FinancialJuice`.
- `EPIC-026` devient le cadrage production securite/charge: backend Rust comme origin/producteur, Durable Objects pour le fan-out metadata, HLS protege et cache par CDN, badges sources via heartbeat/snapshot, et details sources reserves a l'admin.
- `EPIC-027` optimise le data plane HLS valide par `EPIC-026`: les segments sont deja absorbes par le Worker, les playlists doivent maintenant etre cachees tres court cote Worker pour reduire la pression origin et preparer 1000 auditeurs.
- les EPICs restants de robustesse et delivery doivent s'aligner sur ce contrat EN+FR plus audio sans remettre en cause la priorite de la livraison MVP.

## Definition de Done par phase

### Phase 0

- un standard ecrit formalise la regle de commentaires Rust pedagogiques
- `NEWS-046` est fait et le standard est reference par les docs centrales
- `NEWS-047` est fait et fournit l'audit de couverture ainsi que l'ordre de traitement
- le scope de couverture Rust est planifie explicitement
- chaque ligne de code Rust du scope traite est commentee en francais
- les specificites Rust et les choix d'architecture sont expliques dans le code
- les futurs tickets Rust ne peuvent plus etre fermes sans cette couverture
- la review humaine des tickets Rust s'appuie sur une checklist explicite et centrale

### Phase 1

- les `headline` eligibles peuvent produire une variante texte francaise via `OPUS-MT`
- les traductions sont persistees a part et restent rattachees au texte source
- Telegram et le flux texte WebSocket peuvent consommer le meme item derive EN+FR
- le cout local de traduction est documente
- les erreurs de traduction restent non bloquantes pour l'ingestion et la diffusion existantes

### Phase 2

- `narration_text` FR peut produire une sortie TTS cote serveur
- cette sortie audio est disponible en memoire pour le client, sans dependance a `macos_say` cote serveur
- le format audio, le chunking et le cout local TTS sont documentes
- les erreurs TTS restent non bloquantes pour l'ingestion et la diffusion existantes

### Phase 3

- un contrat WebSocket texte EN+FR stable permet de recevoir les items derives en temps reel
- un control plane temps reel publie `audio_ready` quand un clip HLS finalise est lisible depuis le debut
- un client peut lire automatiquement ces clips et detruire le player quand la queue est vide
- l'historique charge au demarrage ne declenche pas de lecture automatique et sert de dedup locale
- un bouton de test peut declencher un run E2E depuis le client Web
- le run de test suit exactement le meme pipeline que Telegram, le flux texte et `audio_ready`
- un flux WebSocket de debug permet de suivre chaque etape du run de test
- aucune generation ni gestion de fichiers audio persistes n'est introduite
- les erreurs de traduction restent non bloquantes pour l'ingestion et la diffusion existantes

### Phase 4

- variables d'environnement documentees et chargees de facon coherente
- contrat `NewsEvent` et types d'erreur internes unifies
- persistence SQLite initialisee avec migrations et indexes minimaux
- collecteurs branches sur une abstraction commune, meme si les parseurs restent vides

### Phase 5

- `Financial Juice` remonte des evenements reels via RSS, avec fallback documente
- les items sont normalises et dedupes de facon idempotente
- les fixtures et tests couvrent les cas courants et les regressions de parsing

### Phase 6

- lecture historique disponible via repository et endpoint HTTP
- diffusion WebSocket stable avec comportement explicite en cas de lag
- relay Telegram propre avec retry et throttling minimum

### Phase 7

- logs structures et observabilite exploitables en incident
- budgets reseau et retry explicites par source
- detection des degradations et des payloads invalides sans perte silencieuse

### Phase 8

- image Docker et conventions d'environnement documentees
- runbook local et production disponible
- scenarios E2E et checklist release permettent un go/no-go clair

## Standards de documentation des tickets

Tous les tickets des EPICs utilisent le meme contrat:

- ID unique `NEWS-xxx`
- objectif metier
- scope
- hors scope
- approche technique
- criteres d'acceptation
- tests attendus
- dependances

L'objectif est d'obtenir des tickets directement actionnables par un developpeur, sans estimations de charge ni arbitrage restant.

En plus de ce contrat, chaque implementation issue d'un ticket doit respecter la consigne permanente de developpement sur les commentaires de code en francais, et tout code Rust doit suivre [RUST-COMMENTING-STANDARD](./RUST-COMMENTING-STANDARD.md).
