# EPIC-002 - Financial Juice end-to-end

## Objectif

Livrer la premiere source reelle du service, de l'acces distant jusqu'a l'evenement normalise injecte dans le pipeline.

## Valeur livree

Un flux `Financial Juice` exploitable en continu, dedupe et teste, servant de reference pour les autres integrations.

## Statut

Depasse par `EPIC-011` au 6 mai 2026.

## Contexte courant et dependances

Le premier collecteur `FinancialJuice` du projet etait base sur le RSS puis un fallback HTML. Ce lot a ete retire du backend apres stabilisation de `EPIC-011`: session web authentifiee, `GetPreviousNews`, live SignalR et calendrier separe. Cet EPIC reste documente comme historique produit, mais il n'est plus une capacite runtime du service Rust.

## Etat reel au 6 mai 2026

Ce qui reste pertinent comme historique:

- strategie `RSS first` initiale
- fallback HTML initial
- mapping historique des items vers `NewsEvent`
- normalisation historique de `published_at`
- dedup et idempotence historique via `signature`
- tests de parsing et de regression qui existaient pour ce palier

Ce qui a change:

- le RSS n'est plus present dans le backend
- le fallback HTML n'est plus une capacite runtime
- la vraie acquisition `FinancialJuice` vit maintenant dans `EPIC-011`

Reste a surveiller:

- robustesse sur changements futurs de structure amont
- observation en production lors de degradations source

## Definition of Done

- strategie d'acces priorisant le RSS documentee et implemente
- fallback defini si le RSS ne fournit pas les informations attendues
- mapping vers `NewsEvent` stable
- dedup specifique a la source prouve sur fixtures
- tests de parsing et de non-regression en place

Cette Definition of Done de l'EPIC historique est conservee pour lecture, mais elle n'est plus l'objectif nominal du backend actuel.

## Risques et points durs

- changements de structure RSS/HTML
- titres dupliques avec URL differentes
- evenements republies avec timestamps fluctuant
- blocages intermittents cote source

## Liste des tickets

- `NEWS-005` Strategie d'acces source `Financial Juice` RSS puis fallback
- `NEWS-006` Parseur `Financial Juice` et mapping vers `NewsEvent`
- `NEWS-007` Dedup/idempotence source-specifique `Financial Juice`
- `NEWS-008` Fixtures et tests source `Financial Juice`

## Details des tickets

### `NEWS-005` Strategie d'acces source `Financial Juice` RSS puis fallback

**Objectif**  
Definir le chemin d'acces primaire et le chemin de repli pour `Financial Juice`.

**Scope**  
Documenter l'URL prioritaire RSS, les headers minimaux, le timeout, la frequence de polling, et le fallback HTML ou secondaire si le RSS est indisponible ou incomplet.

**Hors scope**  
Contourner un anti-bot agressif via navigateur headless.

**Approche technique**  
Conserver une strategie "RSS first". Ajouter un runner qui essaye d'abord le RSS et ne passe en fallback que sur condition explicite: statut HTTP non valable, body vide, parsing impossible ou schema incomplet. Le fallback doit rester encapsule dans la source `Financial Juice` et ne pas polluer le contrat global.

**Criteres d'acceptation**

- le chemin prioritaire et le fallback sont codes et documentes
- les erreurs reseau et parsing indiquent le chemin utilise
- l'intervalle de polling est configurable sans recompilation
- le fallback n'est pas active silencieusement sans trace

**Tests attendus**

- test du chemin RSS nominal
- test de bascule vers fallback
- test d'absence de fallback sur erreur non eligibile

**Dependances**

- `NEWS-001`
- `NEWS-002`

### `NEWS-006` Parseur `Financial Juice` et mapping vers `NewsEvent`

**Objectif**  
Transformer la reponse brute `Financial Juice` en evenements normalises.

**Scope**  
Parser les items utiles, extraire titre, URL, date de publication, texte complementaire et metadata source, puis construire un `NewsEvent`.

**Hors scope**  
Enrichissement semantique ou classification en categories avancees.

**Approche technique**  
Utiliser `quick-xml` pour le RSS et un parseur HTML de secours si necessaire. Normaliser les champs en UTC, nettoyer les titres, et ranger les specificites `Financial Juice` dans une zone metadata source. Les items partiels doivent etre rejetes explicitement, pas completes de facon implicite.

**Criteres d'acceptation**

- chaque item parse produit un `NewsEvent` valide
- les champs obligatoires sont remplis ou l'item est rejete avec raison
- le nettoyage des titres et URLs est stable
- les dates de publication sont normalisees en format unique

**Tests attendus**

- parsing d'un flux nominal avec plusieurs items
- rejet d'un item incomplet
- verification de la normalisation de date et de texte

**Dependances**

- `NEWS-003`
- `NEWS-005`

### `NEWS-007` Dedup/idempotence source-specifique `Financial Juice`

**Objectif**  
Eviter les doublons propres a `Financial Juice`.

**Scope**  
Definir la signature metier source-specifique, les regles de comparaison et la politique de re-publication si un item revient inchangé.

**Hors scope**  
Dedup cross-source garde comme principe general si une deuxieme source est ajoutee plus tard.

**Approche technique**  
Completer la signature globale avec une logique `Financial Juice` basee sur titre nettoye, URL canonique et horodatage normalise si present. Separer clairement dedup memoire court terme et idempotence durable via `signature` unique en base.

**Criteres d'acceptation**

- un meme item repolle n'est emis qu'une fois
- deux items differents mais proches ne sont pas fusionnes a tort
- la regle de signature est documentee et testee

**Tests attendus**

- double polling du meme flux
- collision negative entre deux titres tres proches
- insertion ignoree sur `signature` deja presente

**Dependances**

- `NEWS-004`
- `NEWS-006`

### `NEWS-008` Fixtures et tests source `Financial Juice`

**Objectif**  
Installer un filet de securite de non-regression sur la source prioritaire.

**Scope**  
Creer des fixtures representative du RSS nominal, d'un flux degrade, d'un fallback HTML et des cas limites de duplication.

**Hors scope**  
Tests live contre la source distante en CI.

**Approche technique**  
Stocker des fixtures locales versionnees, brancher des tests unitaires et d'integration de parsing, et documenter la procedure d'ajout d'une nouvelle fixture lors d'une regression de production.

**Criteres d'acceptation**

- les cas nominal, degrade et fallback sont couverts
- les tests ne dependent pas du reseau
- une regression de parsing est reproductible a partir d'une fixture versionnee

**Tests attendus**

- execution du parseur sur chaque fixture
- verification de snapshot logique sur les champs normalises
- test de dedup sur fixtures repetees

**Dependances**

- `NEWS-005`
- `NEWS-006`
- `NEWS-007`
