# NEWS-236 — Refonte admin : historiques, calendrier, résumé audio et logs

## Statut

**Terminé localement — 17 septembre 2026.** gluestack v5 / UniWind / Tailwind CSS 4.

EPIC : [EPIC-039](../epics/EPIC-039-refonte-pwa-gluestack.md).
Référence : [charte PWA et inventaire des routes](../design/pwa/README.md).

## Objectif métier

Rendre les outils d’investigation cohérents et lisibles tout en conservant les capacités de diagnostic et les limites de lecture des journaux.

## Périmètre

- Migrer /admin/history, /admin/calendar-history, /admin/calendar-day, /admin/daily-summary et /admin/logs.
- Conserver filtres, sélection, détails, actions de traitement et lecture audio du résumé selon les contrats existants.
- Intégrer les lecteurs de logs aux composants partagés en gardant la monospace pour le contenu technique.

## Hors périmètre

Réécriture des API de logs ou contrats de pagination/fragmentation, chargement intégral de journaux, changement de traitements calendrier/résumé, remplacement des modifications locales admin.

## Approche technique

- Inventorier les actions et sous-vues de ces cinq routes avant migration ; conserver hooks, services, curseurs, bornes et annulation/nettoyage existants.
- Préserver api.ts et headers.ts dans features/admin/lib/rustLogger ainsi que tests/rustLoggerHeaders.test.mjs ; partir de leur état courant, jamais d’un ancien HEAD pour écraser ce travail.
- Utiliser tableaux/détails/filtres partagés et défilement interne aux contenus techniques ; préserver sélection/copier du texte et distinction des niveaux de logs.
- Conserver lecture bornée, gestion des gros fichiers, erreurs/fin de flux et réponses tardives ; aucun chargement non borné pour faciliter le nouveau rendu.
- Réutiliser les moteurs audio existants pour les contrôles du résumé ; ne pas démarrer un second lecteur ou modifier leur politique d’arbitrage.

## Critères d’acceptation

- Parité de toutes les fonctions existantes des cinq routes, y compris sous-vues et lecteurs techniques.
- Permissions, filtres, limites de données lues et comportements d’annulation inchangés ; traitement sûr de l’erreur et de la reconnexion.
- Monospace seulement pour contenu technique, contraste lisible, texte copiable et page sans débordement global.
- Les correctifs préexistants d’en-têtes du lecteur Rust sont conservés ; aucun secret exposé dans captures ou messages.

## Tests attendus

- Régressions des lecteurs bornés : gros fichiers simulés, pagination/fragments, sélection rapide, fin de données, erreur réseau, démontage et accès refusé ; vérifier les limites de requêtes.
- Tester historiques, détails calendrier, résumé et actions autorisées avec les données de test existantes ; aucune action serveur en production pour la recette locale.
- Exécuter les tests rustLoggerHeaders disponibles, check:pwa et build:pwa ; deux thèmes aux cinq largeurs, clavier, zoom et captures locales.

## Dépendances

- [NEWS-235](./NEWS-235-refonte-admin-supervision-pwa.md).

Les commentaires du code livré restent en français. Les preuves de recette locale restent distinctes de tout déploiement.

## Livraison et preuves

Historiques, calendrier, résumé et logs migrés. Fenêtre de logs bornée testée ; les trois fichiers du correctif local sont identiques.

[Recette locale, commandes, résultats et captures](../design/pwa/recette/2026-09-16/README.md).
Aucun commit, push, déploiement ou livraison native.
