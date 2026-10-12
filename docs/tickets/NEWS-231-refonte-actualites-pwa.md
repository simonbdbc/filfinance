# NEWS-231 — Refonte de la page Actualités et du détail des news

## Statut

**Terminé localement — 17 septembre 2026.** gluestack v5 / UniWind / Tailwind CSS 4.

EPIC : [EPIC-039](../epics/EPIC-039-refonte-pwa-gluestack.md).
Référence : [charte PWA et inventaire des routes](../design/pwa/README.md).
Maquette retenue : [Actualités, sans calendrier](../design/pwa/maquettes/pwa-actualites-v1.png),
confirmée par le fichier fourni « Image Codex 16 sept. 2026, 11_01_10.png ».
Conserver cette image à l'identique ; aucune correction graphique nécessaire.

## Objectif métier

Lire et explorer les actualités pendant l’écoute avec une sélection lisible et un détail accessible sur mobile comme sur ordinateur.

## Périmètre

- Migrer /feed et ses compositions vers le catalogue partagé ; appliquer la planche Actualités et les deux thèmes.
- Conserver historique, périodes, filtres et sélection actuels, chargement du détail et liens sources.
- Adapter liste/détail aux seuils du cadre ; contrôler les sources avec les données et permissions existantes.
- La page comporte le lecteur audio, la liste et le détail ; aucun calendrier ou aperçu de semaine. Le lien de navigation Calendrier reste disponible vers `/calendar`.

## Hors périmètre

Nouvelle API, réduction de l’historique à Aujourd’hui/Hier, nouvelle permission de source, contenu illustratif injecté ou lecteur audio supplémentaire.
Ajout d'un calendrier à `/feed` ou reprise du bloc « Cette semaine » de la planche de direction générale.

## Approche technique

- Conserver hooks/services et requêtes de la fonctionnalité ; faire évoluer les compositions et non les contrats de données.
- Relier lignes et détail avec identifiants stables ; protéger le détail contre les réponses tardives lors d’une sélection rapide.
- Réutiliser FilterBar, DataRow, DetailPanel et AsyncState ; exposer chargé, chargement, vide, erreur et reprise sans masquer la liste utile.
- Les onglets et catégories dessinés ne créent pas de nouvelles données ; conserver les sources/horodatages et leur sémantique réels.

## Critères d’acceptation

- Historique et détails accessibles sans perte de fonctions actuelles ; le détail correspond à la sélection et aux permissions.
- Composition conforme à la planche Actualités inchangée, sans calendrier intégré dans les deux thèmes et à toutes les largeurs.
- À 1280 px et plus, deux colonnes ; en dessous, panneau avec retour vers la ligne déclenchante.
- Texte long, erreur de détail ou fil vide ne provoquent pas de débordement de page.
- Lecture audio maintenue pendant navigation, sélection et thème ; aucun duplicata de palette ou de primitive.

## Tests attendus

- Parcours avec historique fourni, vide, erreur/reprise et sélection rapide ; vérifier détails traduits et liens de source existants.
- Tests clavier/retour du panneau et deux thèmes sur les cinq largeurs de recette, avec titres très longs.
- Vérifier l'absence de bloc calendrier sur `/feed` et le fonctionnement de son lien de navigation vers `/calendar`.
- Exécuter check:pwa dont qa:history, build:pwa et conserver captures comparatives datées.

## Dépendances

- [NEWS-230](./NEWS-230-navigation-audio-persistant-pwa.md).

Les commentaires du code livré restent en français. Les preuves de recette locale restent distinctes de tout déploiement.

## Livraison et preuves

Fil sans calendrier, sélection/détail, sources selon droits et états asynchrones livrés ; focus et relance testés.

[Recette locale, commandes, résultats et captures](../design/pwa/recette/2026-09-16/README.md).
Aucun commit, push, déploiement ou livraison native.
