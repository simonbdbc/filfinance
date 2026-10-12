# NEWS-218 - Client OpenRouter Rust et shadow calendrier

## Statut

**Implemente et deploye en shadow DEV ; statut historique de diagnostic au 28
aout 2026.** Le flag reste desactive par defaut et la campagne reste a `0/5`;
elle ne bloque plus les phases suivantes ni le retrait n8n. Apres le
mismatch `ready`/`not_produced` du jour cible `2026-08-25`, l'observation de
`2026-08-24` a produit un nouveau mismatch ingestion: baseline
`Rejected(invalid_shape)`, natif `Ready` avec 4 items, sans summary correle. Le
probe live est reste vert et cette observation du 24 aout ne contient aucun
`not_produced`; cette absence ponctuelle ne clot pas le risque de recurrence.
Le correctif suivant, SHA `6690560da66353cd5920767e87cf4bcaac7950a5`, a
passe les dix validations du pipeline `2798029944`; le pipeline a ensuite
termine `canceled`, son job manuel d'observation n'ayant pas ete joue. Le
correctif a ete deploye separement en
recreant uniquement le backend et en important/reactivant les deux workflows
calendrier. L'attestation DEV et les probes backend/live sont verts. Aucune
observation n'a ensuite ete lancee : les deux sources HTML autorisees
retournaient `403` pour tous les jours vierges examines. Le flux JSON
officiellement exporte par ForexFactory etait accessible mais limite a la
semaine courante, sans jour admissible; il n'a pas ete utilise comme changement
de baseline implicite. La campagne reste donc a `0/5`.

Au SHA `b97818a99428688a74b3c91c49c717721dd92057`, les trois audits
Lighthouse mobile et les trois audits desktop ont passe la gate performance
`0,80` dans le pipeline `2799273585`. Le pipeline global a neanmoins echoue
avant deploiement sur la gate Pages : Cloudflare refuse
`pwa-dev.filfinance.fr` et l'ancien `app-dev.filfinance.fr` sous la meme
politique `banned domain`. Le projet PWA est deja Direct Upload, sur `main` et
sans Git connecte. Ne pas contourner cette politique; NEWS-218 utilise la voie
ciblee documentee, avec les flags full-stack desactives, jusqu'a resolution par
Cloudflare.

## Objectif

Executer la qualification IA calendrier par Rust en parallele de n8n, sans
modifier les donnees produit ni les audios.

## Scope

- Client OpenRouter Rust avec timeouts, retry borne et logs rediges.
- Flag `FILFINANCE_CALENDAR_NATIVE_SHADOW_ENABLED=false` par defaut.
- Rapport de parite : hashes, compteurs et differences compactes seulement.

## Criteres d'acceptation

- Les mocks OpenRouter satisfont les golden tests de `NEWS-217`.
- Le shadow n'ecrit aucun item, summary, audio ou alerte native.
- Une divergence critique de contrat est mesurable dans la supervision admin.
- Un hand-off progress absent ou refuse reste visible comme issue
  `not_produced` redigee; il reste un diagnostic et non une mesure de gate.

## Dependances

- [`NEWS-217`](./NEWS-217-contrats-rust-calendrier-parite.md)

## Gate Rust pedagogique

Le standard [`RUST-COMMENTING-STANDARD`](../RUST-COMMENTING-STANDARD.md) et la
[`RUST-REVIEW-CHECKLIST`](../RUST-REVIEW-CHECKLIST.md) s'appliquent a tous les
fichiers Rust touches : configuration, `AppState`, bootstrap/workers,
`calendar_day.rs`, contrats, client OpenRouter, prompts, coordinateur et tests.
Le ticket ne peut pas etre clos sans commentaires pedagogiques coherents,
`cargo check` et `cargo test` verts (ou impossibilite explicitement documentee).

## Implementation

- `OpenRouterCalendarClient` reutilise le `reqwest::Client` global. Chaque
  tentative a un timeout, les retries sont limites aux timeouts/erreurs
  transport/HTTP 408, 429 et 5xx, le backoff et `Retry-After` sont plafonnes,
  la reponse est lue par chunks avec une taille maximale et un semaphore borne
  refuse immediatement la saturation.
- La requete shadow reproduit la forme n8n : un unique message `user` contient
  les instructions et le payload JSON deja joints, `temperature` vaut `0.1`,
  et aucun `response_format` n'est envoye. Seule l'ingestion porte
  `max_tokens: 24000`; le summary omet entierement ce champ.
- La file `mpsc` shadow est distincte de `ingest_tx`, de Piper et de HLS. Les
  callbacks utilisent une admission non bloquante; le receiver ne retire aucun
  job tant que la concurrence active est pleine.
- Les nodes n8n `Progress Ingest Prompt Ready` et
  `Progress Summary Prompt Ready` transmettent conditionnellement les objets
  exacts deja utilises dans leurs prompts. Pour le summary, `shadowItems`
  contient chaque item SQLite normalise complet de `allItems`, avec tous ses
  champs et son ID stable, et non une projection reduite. Rust les convertit
  immediatement en `CalendarSourceSnapshot` ou `CalendarSummaryInputItem`;
  aucune seconde collecte, aucun contournement anti-bot et aucune nouvelle
  route ne sont ajoutes.
- Sous `executionOrder: v1`, la branche progress est positionnee au-dessus de
  la branche OpenRouter afin de remettre l'input transitoire avant le callback
  baseline final. Elle reste best-effort avec `continueOnFail`, mais son POST
  HTTP peut retarder chacun des deux workflows jusqu'a son timeout borne de 10
  secondes. La chaine ingestion puis summary peut ainsi cumuler jusqu'a 20
  secondes si les deux POST expirent, sans toucher au pipeline live news. Un
  timeout ou un hand-off indisponible ne met pas n8n en echec; le callback final
  produit alors une issue shadow `not_produced` redigee et mesurable.
- Les deux webhooks n8n comparent leur header au seul secret configure dans
  l'environnement et echouent ferme si celui-ci manque; le body entrant ne peut
  jamais fournir lui-meme la valeur attendue.
- Le coordinateur est indexe par `(stage, run_id)`, borne en capacite et TTL,
  et conserve un tombstone borne apres emission. Les snapshots/titres exacts
  restent transitoires en memoire et sont abandonnes des que la preuve baseline
  est calculee.
- Le hand-off et chaque callback terminal shadow exigent une correlation stricte avec le run Rust encore `pending` :
  `run_id`, `local_day`, kind ingestion/summary et `n8n_execution_id` doivent
  correspondre. Les issues terminales sont aussi detectees dans les artefacts
  SQLite de parite : un replay apres redemarrage reste idempotent et ne relance
  pas OpenRouter pour le meme `(run_id, stage)`.
- Le comparateur `shadow_invariant` mesure sources, couverture,
  deduplication, francais visible, `final_importance`, IDs `high`, statut
  summary et eligibilite theorique audio. Une simple reformulation francaise
  ne devient pas une divergence critique.
- L'ingestion expose elle aussi un statut ferme : `Ready` apres canonicalisation
  et validation, `Rejected` lorsqu'une sortie est refusee avant persistance, et
  `NotProduced` lorsqu'aucune sortie exploitable ou aucun hand-off correle
  n'est disponible. Un rejet porte en plus une cause fermee (`empty`,
  `french_visible`, `invalid_shape`, `invalid_time` ou `other`) afin que deux
  echecs differents ne deviennent pas un faux match, sans conserver le texte fautif.
- Le callback d'erreur ingestion n8n transmet le champ interne type
  `shadowRejectionReason` pour ses trois refus observables : `invalid_shape`
  lorsqu'un contenu est present mais sa forme est invalide, `empty` lorsque
  `items` est vide et `french_visible` lorsque la validation des champs visibles
  echoue. Une erreur fournisseur ou une absence de contenu reste `NotProduced`
  sans cause. Rust convertit les causes connues vers
  `CalendarShadowIngestionRejectionReason`; une valeur future inconnue ne casse
  jamais la baseline et redevient une absence mesurable. Un callback ancien ou
  un echec de couverture source suit la meme compatibilite. Les diagnostics n8n
  ne conservent que compteur, formes et codes fermes, jamais le texte fautif ni
  les noms de cles fournis par le modele. Le diagnostic `invalid_shape` borne
  son vocabulaire a la forme du contenu, l'origine du candidat, la forme racine
  et la cause structurelle.
- Le parseur Rust respecte la priorite de fallback n8n: une premiere fence JSON
  parseable non-calendrier est rendue avant la recherche d'un objet equilibre
  hors fence; une fence calendrier ulterieure conserve toutefois la priorite.
  Il reinitialise egalement le scanner a chaque accolade apres une enveloppe ou
  une quote inachevee, dans une limite de 256 candidats et 8 millions d'etapes
  inspectees. Cet alignement n'accepte aucune racine ni aucun champ
  supplementaire.
- La persistance donne la priorite a `mismatch`, puis a `not_produced`, puis a
  `match`. Deux statuts `NotProduced` identiques ne constituent donc jamais une
  preuve positive de parite, pour l'ingestion comme pour le summary.
- Seuls les rapports contenant SHA-256, compteurs, booleens, enums et codes
  fermes sont ajoutes aux steps/artifacts existants. Un echec shadow ne change
  jamais le statut du run n8n.
- Le code ferme d'un echec worker est conserve dans une step idempotente meme
  lorsqu'une paire terminale a deja produit son mismatch. Le guard durable
  interdit alors seulement un artefact `not_produced` tardif et trompeur.
- Le defaut applicatif du backoff reste `500 ms`. L'observation DEV suivante a
  fixe explicitement `5000 ms`, avec timeout `60000 ms` et deux retries
  inchanges, afin d'espacer davantage les tentatives apres le timing observe
  sans toucher n8n.

## Configuration

```text
FILFINANCE_CALENDAR_NATIVE_SHADOW_ENABLED=false
FILFINANCE_CALENDAR_NATIVE_SHADOW_REQUEST_TIMEOUT_MS=60000
FILFINANCE_CALENDAR_NATIVE_SHADOW_RETRY_ATTEMPTS=2
FILFINANCE_CALENDAR_NATIVE_SHADOW_RETRY_BACKOFF_MS=500
FILFINANCE_CALENDAR_NATIVE_SHADOW_MAX_CONCURRENCY=1
FILFINANCE_CALENDAR_NATIVE_SHADOW_QUEUE_CAPACITY=4
FILFINANCE_CALENDAR_NATIVE_SHADOW_MAX_RESPONSE_BYTES=1000000
OPENROUTER_API_URL=https://openrouter.ai/api/v1/chat/completions
OPENROUTER_MODEL=nvidia/nemotron-3-super-120b-a12b:free
OPENROUTER_API_KEY=<secret serveur>
```

Les bornes maximales acceptees sont 60 secondes par tentative, trois retries,
5 secondes de backoff, huit requetes actives, 64 jobs en queue et 1 000 000
d'octets par reponse. HTTP est refuse hors loopback; credentials, query et
fragment sont interdits dans l'endpoint. Les formats `Debug` redigent la cle,
les prompts, les snapshots et les reponses IA.

## Absence d'effets de bord

Le worker phase 2 n'appelle jamais `upsert_item`, `ensure_summary_generating`,
`queue_calendar_audio`, les tables audio/alertes, Telegram ou la publication
live. La baseline n8n reste seule responsable des donnees produit et de Piper/
HLS. Les tests comparent explicitement les compteurs des tables items, summaries,
audio et alertes avant/apres un hand-off et un rapport shadow.

## Verification locale

```text
cargo test --manifest-path filfinance-service/Cargo.toml calendar_day
cargo test --manifest-path filfinance-service/Cargo.toml
cargo check --manifest-path filfinance-service/Cargo.toml
cargo fmt --manifest-path filfinance-service/Cargo.toml -- --check
node scripts/check-n8n-calendar-language-guard.cjs
node scripts/check-n8n-calendar-shadow-contract.cjs
```

Les tests couvrent notamment timeout, retry, 4xx permanent, `Retry-After`,
reponse bornee, payload OpenRouter exact, semaphore sature, backpressure de
queue, TTL/eviction, statuts ingestion, callbacks dans les deux ordres,
correlation stricte, idempotence durable, hand-off absent, doublons, redaction,
variations textuelles et absence d'ecriture produit. La garde n8n verifie aussi
que le callback progress transmet l'input summary complet, est place avant
OpenRouter sur le canvas, reste `continueOnFail` et borne chaque attente HTTP a
10 secondes dans les deux workflows.

## Diagnostic historique non bloquant

Activer le flag uniquement dans un environnement d'observation possedant sa
cle OpenRouter, sans changer le chemin produit. Aucun resultat shadow, mismatch
ou `not_produced` ne peut interdire `NEWS-219` : les collecteurs Rust sont
valides par leurs contrats natifs et par le test de recuperation complet de
`EPIC-037`.

Avant le prochain POST unique, apres l'attestation SHA/runtime, une sonde
executee dans le conteneur n8n sonde les deux URLs HTML datees canoniques avec
les headers n8n, `credentials: omit`, sans credentials, cookie, proxy declare
ni redirection suivie, deux requetes consecutives au maximum, timeout de 20
secondes, zero retry et corps borne a 1 Mo. Elle reprend le compactage/detection
de date n8n, exige l'annee cible et une preuve de page calendrier, puis rejette
un challenge synthetique ou contenu non calendrier. Elle exige au moins une
reponse `2xx` couvrant le jour cible avant JWT, appel admin ou POST. Le verdict
est ferme et redige : si aucune source ne fournit cette preuve (deux `403`,
timeout, redirection, corps trop grand ou jour absent), la gate reste fermee
sans creer de run. Le job `probe:dev:calendar-shadow-sources` peut etablir cette
seule eligibilite sans redeploiement ni observation : son resultat ne compte pas
dans `0/5` et ne cree aucun `not_produced`. Le jour doit aussi ne porter ni
historique borne a 20, ni ingestion `manual_admin`. L'adaptation au Weekly
Export ForexFactory requiert un ticket separe avec cache partage, conversion `Europe/Paris`, preservation de
l'URL admin et nouvelle campagne de baseline.
