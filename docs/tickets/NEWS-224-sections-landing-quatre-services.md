# NEWS-224 — Sections et aperçus des quatre services de la landing

## Statut

**Terminé localement le 12 septembre 2026.** Les huit blocs, les textes et
les SVG locaux sont intégrés. Les deux bandes audio occupent toute la largeur ;
les quatre services, la liste illustrative et les sept jours sont contrôlés
dans le build et le navigateur. Les photographies sont retirées de la page.
Voir la [comparaison visuelle](../../apps/landing/design-qa.md) et les
[captures des deux thèmes](../design/landing/recette/2026-09-12/README.md).

EPIC : [EPIC-038](../epics/EPIC-038-refonte-visuelle-landing-themes-clair-sombre.md).
Source de vérité : [textes et maquettes](../design/landing/README.md).

## Objectif métier

Présenter les quatre services de Fil Finance immédiatement sous l'introduction,
puis montrer chacun d'eux dans une composition éditoriale cohérente. Distinguer
le flux audio de news du résumé audio du jour.

## Périmètre

- Reconstruire les huit blocs : en-tête, introduction, présentation des quatre
  services, flux audio, fil d'actualités, résumé audio du jour, calendrier et
  pied de page.
- Reprendre tous les textes du référentiel dans la configuration de contenu.
- Donner aux deux sections audio un fond contrasté pleine largeur, avec leur
  contenu aligné sur le conteneur général.
- Réutiliser l'icône audio dans les deux zones de marque, conserver les titres
  serif et la seconde ligne du H1 en italique.
- Retirer les photographies et les anciens blocs de présentation de la page ;
  ne pas ajouter de section gamma.

## Hors périmètre

API publiques, lecture réelle, données de marché actualisées, autoplay,
streaming, PWA, nouvelles routes, graphiques et carnet d'ordres. Les PNG de
documentation ne sont pas des assets de la page publique.

## Approche technique

- Adapter `src/config/site.config.ts` pour centraliser les textes, la
  navigation et les données illustratives ; conserver les origines configurées
  et les destinations d'accès existantes.
- Décomposer la page Astro en blocs lisibles et composants réutilisables si
  nécessaire, sans îlot client. Tous les fonds, textes et contrôles consomment
  les tokens de NEWS-223.
- Placer le fond contrasté sur la section extérieure, et la contrainte de
  largeur sur son contenu intérieur. Ne pas simuler la pleine largeur par un
  débordement de carte ou un masquage global du débordement.
- Employer du HTML sémantique, un seul H1, des H2 de section et des SVG
  décoratifs pour les ondes et pictogrammes. Conserver `AudioMark.astro`.
- Reproduire les trois lignes illustratives du fil et le tableau fixe des
  sept jours du 7 au 13 septembre 2026, avec leurs mentions d'exemple.
- Les disques de lecture sont des liens d'accès à la PWA, jamais des boutons
  sans effet ni des lecteurs simulés. NEWS-225 complète la recette des accès.
- Retirer les références aux photographies dans la composition sans casser
  la génération des assets encore requis par le build, les favicons et l'image
  sociale. Ne pas réutiliser une maquette complète comme image sociale.
- Commenter les modifications de code en français.

## Critères d'acceptation

- Les quatre services sont présents sous l'introduction, avant le bandeau du
  flux, et chacun possède ensuite sa présentation dédiée.
- L'ordre des huit blocs et les textes correspondent au référentiel.
- Les bandes du flux et du résumé sont contrastées bord à bord dans les deux
  thèmes ; leurs contenus restent centrés et alignés.
- Marque en minuscules et icône audio présentes dans l'en-tête et le footer.
- Sept jours de calendrier, liste et mentions illustratives conservés ;
  aucune fausse disponibilité, durée audio ou actualité réelle inventée.
- Pas de gamma, photographie ou ancien bloc explicatif dans le rendu cible.
- Un seul H1, hiérarchie lisible, aucun PNG documentaire dans les assets publics.

## Tests attendus

Comparer les rendus clair et sombre aux deux maquettes à 1440 px. Contrôler
l'ordre des sections, les textes, la présence des quatre services, les
mentions d'exemple et les sept jours. Vérifier les bords des deux bandes et
les alignements du contenu. Exécuter `npm run check:landing` pour les ancres,
les CTA, les assets et la structure du build. La recette responsive complète
est portée par NEWS-225.

## Dépendances

- [NEWS-223](./NEWS-223-themes-landing-clair-sombre.md).
- Débloque [NEWS-225](./NEWS-225-landing-responsive-navigation-accessibilite.md).
