# NEWS-232 — Refonte du calendrier sur cinq jours ouvrés

## Statut

**Terminé localement — 17 septembre 2026.** gluestack v5 / UniWind / Tailwind CSS 4.

EPIC : [EPIC-039](../epics/EPIC-039-refonte-pwa-gluestack.md).
Référence : [charte PWA et inventaire des routes](../design/pwa/README.md).

## Objectif métier

Préparer la semaine de trading avec cinq jours cohérents, des filtres clairs et des événements consultables sans changer les règles métier.

## Périmètre

- Migrer /calendar vers la référence corrigée lundi à vendredi et le catalogue partagé.
- Conserver sélection du jour, navigation de semaine, filtres, événements et détails existants.
- Présenter les cinq jours sur desktop/mobile et adapter les contrôles de filtres et le panneau de détail.

## Hors périmètre

Week-end ajouté, modification des règles de calendrier, fournisseurs, fuseaux ou API ; hardcoding des dates/heures d’illustration ; modification de la landing à sept jours.

## Approche technique

- Réutiliser les fonctions métier existantes de semaine ouvrée et de jour par défaut ; préserver leur comportement en week-end et aux frontières de semaine/mois/année.
- Séparer sélection du jour et filtrage ; réutiliser la persistance existante des filtres devises/importance sans nouvelle préférence serveur.
- La maquette corrigée sert au cadrage : rendre les dates et horaires des données réelles, sans copier l’incohérence horaire d’une ligne illustrative mobile.
- Déployer les composants partagés de filtres, lignes, états et détail ; maintenir 44 px par cible et retour du focus.

## Critères d’acceptation

- Cinq jours exactement, lundi à vendredi, dates cohérentes dans navigation, titre et événements.
- Règles de sélection actuelle et de repli week-end inchangées ; filtres actuels et leur remise à zéro conservés.
- Détails lisibles dans les deux thèmes, états sans événement/chargement/erreur explicites.
- Pas de défilement horizontal de page ni de nouvelle logique métier dans shared/ui.

## Tests attendus

- Cas semaine du 14–18 septembre 2026, week-end, changement de mois et d’année ; comparer le résultat aux helpers métier existants.
- Filtres combinés, persistance, remise à zéro, sélection et navigation de semaine ; vide/erreur et retour du panneau.
- Deux thèmes aux cinq largeurs, clavier/zoom ; check:pwa, build:pwa et preuves datées.

## Dépendances

- [NEWS-230](./NEWS-230-navigation-audio-persistant-pwa.md).

Les commentaires du code livré restent en français. Les preuves de recette locale restent distinctes de tout déploiement.

## Livraison et preuves

Calendrier distinct à cinq jours ouvrés, filtres, sélection et détail livrés ; règles métier et contrôles conservés.

[Recette locale, commandes, résultats et captures](../design/pwa/recette/2026-09-16/README.md).
Aucun commit, push, déploiement ou livraison native.
