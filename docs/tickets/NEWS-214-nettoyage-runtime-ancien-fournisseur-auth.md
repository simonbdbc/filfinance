# NEWS-214 - Nettoyage runtime de l'ancien fournisseur auth

> **Archive historique — ne pas executer.** Le palier Logto est supersede par
> Better Auth + PostgreSQL. Utiliser [le runbook Better Auth](../AUTH-RUNBOOK.md).

## Statut

Livre cote repo pour le runtime.

## Objectif

Retirer les dependances, variables et modules runtime de l'ancien fournisseur.

## Acceptance

- Le client n'a plus l'ancien SDK auth dans `package.json`.
- Le backend ne contient plus de module JWT provider-specifique obsolete.
- Les scripts et exemples `.env` utilisent Logto.
