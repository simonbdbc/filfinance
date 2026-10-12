# NEWS-229 — Primitives gluestack et composants Fil Finance réutilisables

## Statut

**Terminé localement — 17 septembre 2026.** gluestack v5 / UniWind / Tailwind CSS 4.

EPIC : [EPIC-039](../epics/EPIC-039-refonte-pwa-gluestack.md).
Référence : [charte PWA et inventaire des routes](../design/pwa/README.md).

## Objectif métier

Fournir des interactions et présentations cohérentes à toutes les pages sans réimplémenter les mêmes boutons, filtres, tableaux ou panneaux.

## Périmètre

- Créer les primitives partagées gluestack : boutons, textes, champs, sélecteurs, badges, dialogues, panneaux mobiles et chargement.
- Créer PageHeader, Section, DataRow, ResponsiveTable, DetailPanel, FilterBar, AsyncState, StatusBadge et PreferenceGroup selon le référentiel.
- Définir exports publics, variantes sémantiques, propriétés contrôlées et règles d’accessibilité, avec exemples locaux de tous les états.

## Hors périmètre

Appels API ou imports de features dans shared/ui, nouvelle bibliothèque UI concurrente, migration métier des écrans ou duplication des tokens.

## Approche technique

- Ranger les primitives sous shared/ui/primitives et les exposer via les exports partagés ; limiter la connaissance de gluestack à cette couche.
- Les composants reçoivent données, états et callbacks typés. Séparer ligne sélectionnée, état désactivé et erreur ; libellé et icône accompagnent la couleur.
- Créer des variantes cohérentes plutôt que des couleurs par écran ; documenter le mapping des composants anciens et la suppression des adaptateurs en NEWS-237.
- Gérer focus initial, Échap et retour au déclencheur dans les panneaux/dialogues ; conserver labels de champs, messages d’erreur et en-têtes de tableau.
- Prévoir textes longs, zoom, 44 × 44 px tactiles et affichage compact sans défilement horizontal de page.

## Critères d’acceptation

- Les mêmes composants rendent les variantes utilisateur/admin et les deux thèmes, sans accès API.
- Aucune dépendance de shared vers features ; exports publics stables et variantes visuelles bornées.
- États normal, sélectionné, focus, chargé/chargement, vide, erreur et désactivé compréhensibles au clavier et sans couleur.
- Panneaux fermables, retour du focus fiable, libellés et statuts annoncés ; métadonnées au moins 14 px.

## Tests attendus

- Tests de comportement : callbacks, non-activation désactivée, libellés accessibles, sélection, Échap/retour du focus et reprise après erreur ; ne pas se limiter à des instantanés de classes CSS.
- Démontrer tableaux et panneaux aux petites largeurs et zoom 200 %, dans les deux thèmes.
- Exécuter contrôles d’architecture, npm run check:pwa et npm run build:pwa.

## Dépendances

- [NEWS-228](./NEWS-228-socle-gluestack-themes-pwa.md).

Les commentaires du code livré restent en français. Les preuves de recette locale restent distinctes de tout déploiement.

## Livraison et preuves

Primitives gluestack et composants Fil Finance partagés livrés ; dépendances métier interdites dans shared/ui.

[Recette locale, commandes, résultats et captures](../design/pwa/recette/2026-09-16/README.md).
Aucun commit, push, déploiement ou livraison native.
