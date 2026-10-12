# NEWS-123 - Workflow main backup pour refonte live

## Objectif

Eviter une mise en place lourde `prod/dev` pendant la refonte du flux live.

Decision retenue:

- `main` reste la branche de travail et de deploiement de l'environnement courant;
- l'ancien etat stable de `main` est conserve dans une branche backup et un tag;
- la refonte `EPIC-018` est integree sur `main`;
- le rollback revient sur le backup si l'environnement courant devient instable.

Ce choix est pragmatique pour le contexte solo-dev. Il evite de maintenir deux piles Docker, deux Cloudflare Pages et deux jeux complets de secrets tant que le produit n'est pas en production finale.

## Etat sauvegarde

Etat stable avant refonte live:

```text
commit: 47d49c2
branch: backup/main-prod-before-epic18-2026-05-03
tag: prod-before-epic18-2026-05-03
```

Etat courant refonte live:

```text
commit: 76db3ac
branch: main
```

## Commandes de creation du backup

Depuis `main` avant integration de la refonte:

```bash
git switch main
git pull --ff-only origin main
git branch backup/main-prod-before-epic18-2026-05-03
git tag prod-before-epic18-2026-05-03
git push origin backup/main-prod-before-epic18-2026-05-03
git push origin prod-before-epic18-2026-05-03
```

## Integration de la refonte sur main

```bash
git switch main
git merge --ff-only feat/epic18-live-edge-metadata
git push origin main
```

## Rollback rapide

Rollback local:

```bash
git switch main
git reset --hard backup/main-prod-before-epic18-2026-05-03
```

Rollback distant:

```bash
git push --force-with-lease origin main
```

Puis verifier:

```bash
bash scripts/smoke_deploy.sh
```

## Regles d'exploitation

- Ne pas supprimer la branche backup ni le tag avant validation complete de la refonte live.
- Ne pas reutiliser le backup pour de nouveaux commits.
- Pendant la refonte, l'environnement courant peut etre instable.
- Les changements sensibles doivent rester petits et committes par ticket.
- Avant la production finale `filfinance.fr`, revenir a un process plus strict avec environnement dedie ou promotion controlee.

## Impact

Avantages:

- moins de configuration;
- pas de duplication immediate de Traefik, Docker, Supabase, Cloudflare Pages;
- rollback simple vers l'ancien `main`.

Limites:

- l'environnement courant n'est plus une production stable;
- un push sur `main` peut deployer une refonte incomplete;
- les tests live doivent etre faits avec vigilance.
