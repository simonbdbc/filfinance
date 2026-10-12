# NEWS-225 — Responsive, navigation et accessibilité de la landing

## Statut

**Terminé localement le 12 septembre 2026.** Les deux thèmes sont vérifiés
à 320, 390, 768 et 1440 px, sans débordement de page. Le menu natif, les
ancres, le thème et le calendrier sont utilisables au clavier. Les builds
avec destinations normales puis origines de test valident les liens PWA
sans appeler les services distants. Voir les
[preuves de recette](../design/landing/recette/2026-09-12/README.md).

EPIC : [EPIC-038](../epics/EPIC-038-refonte-visuelle-landing-themes-clair-sombre.md).
Source de vérité : [règles de composition et d'interaction](../design/landing/README.md).

## Objectif métier

Permettre de comprendre l'offre, choisir le thème et rejoindre l'application
sur mobile comme sur ordinateur, avec des interactions accessibles et sans
confondre une illustration avec une fonctionnalité active.

## Périmètre

- Adapter les sections et la navigation aux petits écrans.
- Vérifier les ancres des quatre services et les destinations des accès PWA.
- Conserver la commande de thème visible hors du menu mobile replié.
- Appliquer les règles de contraste, focus, sémantique et lecture au clavier.

## Hors périmètre

Nouvelle route PWA, synchronisation de thème entre applications, lecteur
audio public, API, publication ou changement de l'offre.

## Approche technique

- Appliquer les règles responsive du référentiel : conteneur maximal 1180 px,
  gouttières 24/16 px, grille des services 4/2/1 colonnes, sections éditoriales
  empilées sous 1024 px. Garder le menu mobile natif ; seul le thème ajoute du JS.
- Conserver `#top` et le lien d'évitement `#contenu`. Utiliser `#offre`,
  `#flux-audio`, `#actualites`, `#resume-audio` et `#calendrier` pour les sections.
  En-tête et pied de page partagent la navigation des quatre services.
- Utiliser des liens en même onglet vers `externalDestinations.home` pour
  écouter le flux, consulter le fil et écouter le résumé ; utiliser
  `externalDestinations.login` pour la connexion. Conserver le contrat
  `data-cta` attendu par le validateur.
- Donner aux disques de lecture les noms accessibles « Écouter le flux dans
  l’application » et « Écouter le résumé dans l’application ». Conserver les
  mentions illustratives visibles à proximité des aperçus.
- Préserver le tableau à sept colonnes dans une région défilable nommée et
  utilisable au clavier si sa largeur dépasse l'écran. La page entière ne doit
  jamais nécessiter de défilement horizontal.
- Utiliser les couleurs de texte contrastées documentées pour les libellés ;
  le taupe sur avoine et le cuivre sur surface sombre ne conviennent pas au
  petit texte. Garder des cibles de 44 × 44 px, des focus visibles, des liens
  soulignés et des SVG décoratifs masqués aux technologies d'assistance.
- Respecter la réduction des mouvements ; pas d'animation simulant une lecture.
  Commenter les modifications de code en français.

## Critères d'acceptation

- À 320, 390, 768 et 1440 px, dans les deux thèmes : aucune section coupée,
  aucun chevauchement de la marque, du menu, des accès et du bouton de thème.
- La grille et l'ordre de lecture s'adaptent ; le calendrier conserve sept
  jours avec débordement limité à son conteneur si nécessaire.
- Les quatre liens de navigation atteignent les bonnes sections ; la marque
  retourne en haut et le lien d'évitement atteint le contenu.
- Les accès produit et de connexion respectent les origines configurées,
  sans URL d'environnement codée en dur ni nouveau deep link.
- Les parcours clavier, menu mobile, choix du thème et liens sont utilisables
  et annoncés. Aucun contrôle ne prétend lire de l'audio sur la landing.
- Les contrastes de texte et des commandes sont conformes aux règles du
  référentiel dans tous les états utiles.

## Tests attendus

Parcourir les deux thèmes aux quatre largeurs cibles, ouvrir/fermer le menu,
atteindre chaque ancre et action, puis recommencer au clavier. Tester le
défilement local du calendrier et l'absence de débordement du document.
Vérifier la configuration de production et une origine PWA de test via la
configuration de build existante, sans appeler les services distants.
Contrôler les noms accessibles, le focus et les contrastes ; exécuter
`npm run check:landing`. La recette finale Lighthouse relève de NEWS-226.

## Dépendances

- [NEWS-224](./NEWS-224-sections-landing-quatre-services.md).
- Débloque [NEWS-226](./NEWS-226-recette-landing-themes.md).
