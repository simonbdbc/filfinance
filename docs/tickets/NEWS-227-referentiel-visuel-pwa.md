# NEWS-227 — Référentiel visuel, maquettes et préparation de la refonte PWA

## Statut

**Terminé — documentation vérifiée le 16 septembre 2026.** Voir la [recette documentaire](../design/pwa/recette-documentaire-2026-09-16.md). Aucune implémentation PWA réalisée.

EPIC : [EPIC-039](../epics/EPIC-039-refonte-pwa-gluestack.md).
Référence : [charte PWA et inventaire des routes](../design/pwa/README.md).

## Objectif métier

Disposer d’une référence unique et vérifiable pour réaliser la direction « La table de session » sur toute la PWA, en cohérence avec la landing.

## Périmètre

- Créer EPIC-039 et les onze tickets NEWS-227 à NEWS-237 suivant le contrat du dépôt.
- Documenter palettes, états, typographie, navigation, thèmes, composants gluestack et compositions métier dans docs/design/pwa/README.md.
- Archiver les cinq originaux ImageGen 1644 × 957 px. Corriger uniquement direction générale et Calendrier à cinq jours ouvrés ; conserver leurs originaux dans historique/.
- Enregistrer provenance, dates, tailles, dimensions, SHA-256 et statut ; intégrer une galerie, l’inventaire complet des routes et les liens README/roadmap.

## Hors périmètre

Implémentation PWA, installation gluestack, changement des images des trois autres pages, alternatives non retenues, commit, push ou déploiement.

## Approche technique

- Copier les PNG sans réencodage. Utiliser deux nouvelles versions pour les corrections ; ne jamais écraser un original.
- Vérifier lundi 14 à vendredi 18 septembre 2026, mardi sélectionné : sélecteur ordinateur de la direction, sélecteurs ordinateur et mobile de Calendrier. La direction mobile ne possède pas de grille : ne pas en ajouter.
- Définir les responsabilités UI et conserver les contrats existants ; distinguer référence générée, capture de recette locale et preuve de déploiement.
- Comparer l’inventaire aux fichiers app/ et vérifier les identifiants disponibles avant création. Préserver les bilans historiques et les trois fichiers locaux du lecteur de logs.

## Critères d’acceptation

- Cinq planches actives et deux originaux historiques ; cinq originaux strictement identiques aux sources, sept PNG tracés au total.
- Les deux corrections affichent cinq jours cohérents, sans modification intentionnelle du reste de la composition ; vérification visuelle documentée.
- Palettes, typographie, interactions, états sémantiques et écarts aux images documentés ; aucune confusion avec une implémentation livrée.
- Seize routes et trois layouts rattachés aux tickets ; onze identifiants uniques et liens valides.
- EPIC planifié ; seul NEWS-227 clôturé après vérification documentaire.

## Tests attendus

- Comparer les octets de chaque copie à sa source et recalculer SHA-256, taille et dimensions ; inspecter toutes les planches, dont les deux corrections.
- Valider liens relatifs, cinq inclusions de galerie, unicité des fichiers EPIC/tickets, sections obligatoires et couverture des routes.
- Vérifier git diff --check et les empreintes des trois fichiers admin préexistants. Aucun test applicatif prétendu pour cette livraison documentaire.

## Dépendances

Aucune dépendance de ticket. Réutilise la charte landing et les cinq sources ImageGen sélectionnées.
