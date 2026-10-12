# NEWS-237 — Retrait des anciens composants et recette globale de la refonte PWA

## Statut

**Terminé localement — 17 septembre 2026.** Migration complète et retrait de l’ancien socle vérifiés.

EPIC : [EPIC-039](../epics/EPIC-039-refonte-pwa-gluestack.md).
Référence : [charte PWA et inventaire des routes](../design/pwa/README.md).

## Objectif métier

Clôturer la refonte avec une seule architecture UI cohérente et des preuves de non-régression sur l’ensemble des parcours utilisateur et administrateur.

## Périmètre

- Retirer anciens composants, adaptateurs temporaires, palettes concurrentes et styles d’écran remplacés.
- Vérifier l’ensemble des routes et états du référentiel, les deux thèmes, l’accessibilité et la continuité audio.
- Conserver captures comparatives datées, résultats des tests et état du code testé ; mettre à jour les statuts uniquement selon les preuves.

## Hors périmètre

Suppression des tests métier pour obtenir du vert, retrait de fonctions non représentées par les maquettes, publication native, commit/push/déploiement non autorisés séparément.

## Approche technique

- Suivre l’inventaire des adaptateurs de NEWS-228/229 jusqu’à zéro ; contrôler imports et valeurs de couleurs dans application/, features/ et shared/. Justifier les exceptions métier, dont monospace des logs.
- La source unique des tokens doit alimenter tous les composants ; supprimer les chemins visuels historiques devenus inutiles et les dépendances UI remplacées.
- Adapter les validateurs qui inspectaient noms de composants/classes/couleurs historiques, en maintenant les assertions d’auth, thème, HLS, idle polling, historique, shell, profil, blacklist et légal.
- Créer une matrice route × rôle × thème × largeur avec états pertinents et comparaisons avant/après, données simulées explicitement identifiées.
- Ranger les captures sous docs/design/pwa/recette/<date>/ avec commit ou empreintes du working tree, environnement et commandes. Séparer toute preuve ultérieure d’un environnement publié.

## Critères d’acceptation

- NEWS-231 à NEWS-236 terminés avec parité fonctionnelle ; toutes les routes et états d’auth couverts.
- Un seul système UI gluestack et Fil Finance, aucune palette concurrente ni adaptateur temporaire restant ; frontières d’architecture respectées.
- Deux thèmes à 320, 390, 768, 1024 et 1440 px, zoom 200 %, pas de débordement horizontal de page ; seuils 1024 et 1280 vérifiés.
- Clavier, focus/retour, lecteur d’écran, contrastes, cibles tactiles, stockage refusé et restauration initiale validés.
- Navigation et changement de thème pendant direct/résumé sans interruption ou second abonnement ; auth, calendrier ouvré, légal et admin préservés.
- npm run check:pwa, npm run build:pwa et tests comportementaux des composants passent ; logs toujours lus de manière bornée.
- Preuves locales reproductibles, aucune confusion avec les cinq maquettes ou avec une preuve de production/native.

## Tests attendus

- Matrice des seize routes et trois layouts, rôles visiteur/auditeur/modérateur/admin, états chargé/chargement/vide/erreur/déconnecté/refus/reconnexion ; couverture ciblée des formulaires et actions.
- Matrice thème, largeur et accessibilité ; cas de restauration asynchrone/stockage indisponible, bascule et navigation pendant lecture.
- Contrôles architecture/imports/tokens, tests composants partagés et fonctions métier ; check:pwa et build:pwa avec résultats archivés.
- Captures comparatives, console sans nouvelle erreur et revue des suppressions ; ne clôturer EPIC-039 qu’après satisfaction de tous ses critères.

## Dépendances

- [NEWS-231](./NEWS-231-refonte-actualites-pwa.md).
- [NEWS-232](./NEWS-232-refonte-calendrier-pwa.md).
- [NEWS-233](./NEWS-233-refonte-profil-pwa.md).
- [NEWS-234](./NEWS-234-refonte-authentification-pwa.md).
- [NEWS-235](./NEWS-235-refonte-admin-supervision-pwa.md).
- [NEWS-236](./NEWS-236-refonte-admin-historiques-logs-pwa.md).

Les commentaires du code livré restent en français. Les preuves de recette locale restent distinctes de tout déploiement.

## Livraison et preuves

NEWS-228 à NEWS-236 livrés. `check:pwa` (contrats, architecture, 36 tests et types),
installation et export Web réussis. Matrice de 150 rendus complétée par 10 rendus
de la composition admin finale ; 30 audits axe et revue ciblée complémentaire.
Thèmes, rôles, formulaires, panneaux, continuité audio, erreurs/relance et reconnexion
sont testés. Les empreintes du correctif local de logs sont inchangées.

[Résultats détaillés, limites, captures et empreintes](../design/pwa/recette/2026-09-16/README.md).
Clôture locale de l’EPIC ; aucun commit, push, déploiement ou test natif.

## Correctif du défilement — 17 septembre 2026

La recette initiale ne prouvait pas le défilement vertical des pages connectées.
Le cadre partagé utilise désormais un conteneur défilant contraint, avec retour
en haut au changement de chemin et conservation de position au changement de thème.
La navigation fixe et le runtime audio sont conservés.

[Recette complémentaire : défilement réel, clavier, tactile, panneaux, logs et audio](../design/pwa/recette/2026-09-17-scroll/README.md).
Les preuves précédentes restent historiques ; ce complément couvre la régression.

### Complément du 17 septembre 2026 — authentification silencieuse

Le renouvellement du JWT ne sert plus de dépendance aux chargements et sélections d’écran. La session réseau stable mutualise les renouvellements et rejoue une seule fois les requêtes du service en 401. Voir la [régression observée, les garanties et la recette locale](../design/pwa/recette/2026-09-17-auth/README.md). Les résultats historiques ci-dessus sont conservés ; ce complément ne constitue pas une preuve de déploiement.
