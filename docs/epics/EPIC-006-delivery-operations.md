# EPIC-006 - Delivery et operations

## Objectif

Encadrer la livraison et l'exploitation du service pour passer d'un module dev a un service relancable et verificable.

## Valeur livree

Une base d'exploitation claire: image Docker, conventions d'environnement, runbook, scenarios E2E et checklist de release.

## Statut

A finir. Le deploiement courant fonctionne, mais le runbook transversal et les criteres de go/no-go doivent etre finalises apres `EPIC-024` et `EPIC-005`.

## Contexte courant et dependances

Le projet contient deja un `Dockerfile` minimal et un `.env.example`, mais aucune documentation d'exploitation ni criteres de go/no-go. Cet EPIC depend de tous les precedents afin de figer la forme finale du service.

## Definition of Done

- l'image et les conventions de lancement sont documentees
- les prerequis locaux et production sont explicites
- des scenarios E2E permettent de valider le flux complet
- une checklist release existe et couvre les regressions critiques

## Risques et points durs

- Dockerfile correct mais runbook incomplet
- divergence entre dev local et execution conteneurisee
- absence de scenarios de verification du flux complet
- release sans checklist donc regressions silencieuses

## Liste des tickets

- `NEWS-021` Docker/profils d'environnement et startup checks
- `NEWS-022` Runbook local/prod et matrice de configuration
- `NEWS-023` Scenarios d'acceptation end-to-end
- `NEWS-024` Checklist release et criteres de non-regression

## Details des tickets

### `NEWS-021` Docker/profils d'environnement et startup checks

**Objectif**  
Fiabiliser le demarrage du service en local et en conteneur.

**Scope**  
Finaliser l'image Docker, les variables requises, les volumes utiles, et les verifications de demarrage qui doivent faire echouer le service si le runtime est invalide.

**Hors scope**  
Kubernetes, Helm ou orchestration multi-instance.

**Approche technique**  
Verifier le `Dockerfile`, documenter les ports et chemins attendus, ajouter des startup checks simples sur la config, le stockage et les prerequis externes critiques. Distinguer ce qui doit hard-fail du simple warning.

**Criteres d'acceptation**

- l'image se lance avec la config minimale documentee
- les erreurs de demarrage critiques stoppent le process
- les chemins de stockage et les variables obligatoires sont explicites

**Tests attendus**

- build de l'image
- lancement avec config minimale
- echec attendu sur config critique absente

**Dependances**

- `NEWS-001`
- `NEWS-004`
- `NEWS-018`

### `NEWS-022` Runbook local/prod et matrice de configuration

**Objectif**  
Donner un mode d'emploi operable du service.

**Scope**  
Documenter l'installation locale, le lancement, les variables par environnement, les checks post-demarrage, les commandes de debug et les incidents frequents.

**Hors scope**  
Documentation commerciale ou publique grand public.

**Approche technique**  
Ajouter un runbook oriente exploitation dans `docs/` avec sections local, pre-prod, prod, debug rapide, et tableau de configuration. Referencer la roadmap et les EPICs seulement si utile au suivi du projet.

**Criteres d'acceptation**

- un dev peut lancer le service en suivant le runbook
- un incident courant dispose d'une section de diagnostic minimum
- la matrice de configuration distingue obligatoire, optionnel et recommande

**Tests attendus**

- relecture croisee du runbook a partir d'un environnement vierge
- verification que chaque variable de `.env.example` est documentee

**Dependances**

- `NEWS-021`

### `NEWS-023` Scenarios d'acceptation end-to-end

**Objectif**  
Formaliser les scenarii minimaux de validation du service complet.

**Scope**  
Definir des scenarii E2E couvrant: ingestion source, persistence, diffusion WebSocket, relay Telegram, et mode degrade.

**Hors scope**  
Suite de tests bout en bout executee en environnement reel a chaque commit.

**Approche technique**  
Documenter des scenarii reproductibles, avec donnees attendues, preconditions, commandes et resultat attendu. Prioriser des scenarii derives des vrais tickets critiques plutot qu'une longue liste de cas accessoires.

**Criteres d'acceptation**

- chaque flux critique a au moins un scenario E2E
- les etapes de validation sont actionnables sans interpretation
- un scenario degrade existe pour une source indisponible ou un downstream casse

**Tests attendus**

- scenario nominal `Financial Juice -> SQLite -> WS`
- scenario `Financial Juice -> Telegram`
- scenario `Financial Juice -> audio_jobs -> HLS clip`
- scenario degrade sans perte du service

**Dependances**

- `NEWS-008`
- `NEWS-012`
- `NEWS-021`

### `NEWS-024` Checklist release et criteres de non-regression

**Objectif**  
Encadrer le passage a une version livrable.

**Scope**  
Definir une checklist avant release, les verifications obligatoires, les artefacts attendus, et les regressions bloquantes.

**Hors scope**  
Pipeline CI/CD complet automatise.

**Approche technique**  
Creer une checklist courte mais stricte: build, tests de parsing, smoke test du service, verification Docker, verif des endpoints, et validation du flux temps reel. Marquer comme bloquantes les regressions sur ingestion, dedup, persistence ou diffusion.

**Criteres d'acceptation**

- la checklist est exploitable comme go/no-go
- les regressions bloquantes sont identifiees noir sur blanc
- les preconditions de release et les artefacts attendus sont explicites

**Tests attendus**

- revue de la checklist sur une release candidate
- verification qu'elle couvre bien les flux critiques de la roadmap

**Dependances**

- `NEWS-023`
