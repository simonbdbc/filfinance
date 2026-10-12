# NEWS-105 - Depot GitLab et conventions de branches

## Objectif

Preparer le workspace `filfinance` pour un versioning GitLab propre avant la mise en place du pipeline Cloudflare.

## Statut

Livre cote workspace local le 2 mai 2026.

Le depot GitLab distant a ensuite ete cree, le remote ajoute et `main` pousse.

## Scope

- README racine du monorepo
- `.gitignore` racine
- runbook GitLab
- conventions de branches
- workflow solo-dev sans merge request obligatoire
- regles de tags et releases
- regles de secrets GitLab

## Hors scope

- creation distante du projet GitLab sans URL ni droits utilisateur
- pipeline `.gitlab-ci.yml`
- deploiement Cloudflare Pages
- deploiement Workers/Durable Objects

## Approche technique

Le depot cible est le dossier `filfinance`, pas le dossier parent `dev-trade-analyse`.

La branche principale est `main`. Dans le workflow actuel, `main` est la branche de travail et de deploiement.

Les branches restent optionnelles pour les previews Cloudflare ou les changements risqués. Quand elles sont utilisees, elles doivent garder un prefixe explicite:

- `preview/description-courte`
- `feature/NEWS-xxx-description`
- `fix/NEWS-xxx-description`
- `chore/NEWS-xxx-description`
- `release/vX.Y.Z`

Le workflow GitLab n'impose pas de merge request: le projet est developpe par une seule personne. La regle pratique est de pousser sur `main` apres verification locale pour les changements sensibles, puis de laisser GitLab CI et Cloudflare Pages signaler les regressions.

## Criteres d'acceptation

- `.gitignore` racine protege les secrets et artefacts locaux
- README racine explique la structure du monorepo
- le runbook GitLab documente initialisation, workflow solo-dev, branches optionnelles, tags, secrets et environnements
- `filfinance-service`, `filfinance-client` et `docs` restent les dossiers versionnes principaux
- la creation distante GitLab est documentee et le remote cible est connu
- le depot local est initialise sur la branche `main`

## Tests attendus

Validation locale:

```bash
cd filfinance
git status --short
```

Resultat attendu apres `git init`:

- pas de `.env`
- pas de `node_modules`
- pas de `target`
- pas de `.expo`
- pas de base SQLite locale
- pas de modeles locaux volumineux

Validation executee le 2 mai 2026:

- `git init -b main`: OK
- `git status --short --untracked-files=normal`: OK, seuls les dossiers/fichiers versionnables remontent
- `git check-ignore` sur `.env`, `node_modules`, `target`, `.venv-piper`, `__pycache__`, bases SQLite et modeles locaux: OK

## Dependances

- [EPIC-020 - GitLab et pipeline Cloudflare](../epics/EPIC-020-gitlab-pipeline-cloudflare.md)
- [Runbook GitLab](../GITLAB-RUNBOOK.md)
