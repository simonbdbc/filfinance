# NEWS-235 — Refonte admin : supervision, sources, blacklist et glossaire

## Statut

**Terminé localement — 17 septembre 2026.** gluestack v5 / UniWind / Tailwind CSS 4.

EPIC : [EPIC-039](../epics/EPIC-039-refonte-pwa-gluestack.md).
Référence : [charte PWA et inventaire des routes](../design/pwa/README.md).

## Objectif métier

Unifier la supervision et les outils de configuration sans réduire les possibilités d’administration ni brouiller les états des sources.

## Périmètre

- Migrer /admin, /admin/config-sources, /admin/flux-status, /admin/blacklist et /admin/glossary.
- Décliner la planche Administration avec composants partagés pour tableaux, statuts, filtres, formulaires et actions existantes.
- Garder toutes les destinations admin accessibles depuis le menu autorisé, pas seulement les entrées illustrées.

## Hors périmètre

Modification des permissions, nouvelles API ou métriques, nouvelles actions serveur, données fictives de sources/compteurs, refonte des historiques et logs confiée à NEWS-236.

## Approche technique

- Établir une matrice des actions réellement disponibles pour ces cinq routes avant de migrer leurs compositions, puis vérifier la parité.
- Réutiliser les hooks et validations de configuration, blacklist et glossaire ; conserver confirmations et retours d’erreur existants.
- Une source volontairement désactivée utilise un statut neutre avec texte, pas le point rouge de l’image ; succès et erreur reposent sur l’état réel.
- Remplacer les variantes de composants admin par le catalogue partagé sans imports de fonctionnalités dans shared/ui.
- Maintenir les gardes de rôle à l’accès direct aux routes, même lorsque les liens ne sont pas visibles.

## Critères d’acceptation

- Les cinq routes et toutes leurs actions actuelles sont utilisables par l’admin, avec les mêmes droits et validations.
- Utilisateur non admin et visiteur sans session n’accèdent pas aux outils ; les erreurs réseau ne sont pas confondues avec un refus d’accès.
- Statuts actifs/désactivés/en erreur distincts par texte et icône, compteurs fondés sur les données réelles.
- Aucune palette ou famille de composants admin concurrente introduite ; lecteur connecté inchangé.

## Tests attendus

- Matrice visiteur/auditeur/modérateur/admin, accès par URL, lecture/configuration et validations d’erreur des cinq routes.
- Sources actives, désactivées volontairement, erreurs et données indisponibles ; blacklist/glossaire vide et éditions selon les fonctions actuelles.
- Deux thèmes aux cinq largeurs, clavier et audio persistant ; check:pwa dont qa:blacklist et architecture, build:pwa.

## Dépendances

- [NEWS-230](./NEWS-230-navigation-audio-persistant-pwa.md).

Les commentaires du code livré restent en français. Les preuves de recette locale restent distinctes de tout déploiement.

## Livraison et preuves

Supervision, sources, flux, blacklist et glossaire migrés vers les compositions Workspace ; modérateurs et admin conservent leurs droits.

[Recette locale, commandes, résultats et captures](../design/pwa/recette/2026-09-16/README.md).
Aucun commit, push, déploiement ou livraison native.
