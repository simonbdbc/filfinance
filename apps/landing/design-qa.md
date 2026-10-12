# Design QA — Landing Fil Finance

## EPIC-038 — Recette du 12 septembre 2026

**Validation locale réussie.** Les quatre services et les deux thèmes sont
implémentés. Cette recette concerne la version locale basée sur
`8f49c007b34c76aa0327412ed0d1a5937ea65312`, avec modifications non commitées.
Les [empreintes de sources et de build](../../docs/design/landing/recette/2026-09-12/preuves.json)
identifient exactement l'état testé. Aucun déploiement n'est attesté.

### Références et méthode

- [Référentiel écrit](../../docs/design/landing/README.md) et ses deux PNG
  887 × 1774 px, conservés à l'identique dans `docs/design/landing/maquettes/`.
- Comparaisons côte à côte des deux maquettes et des captures desktop finales.
  Pour comparer la composition, une copie de chaque capture 1440 px a été
  ramenée à 887 px de large, sans déformer les proportions ; les originaux
  documentaires et les captures de recette restent intacts. Le PNG de
  conception représente une composition desktop, pas le breakpoint CSS 887 px.
- Inspection des huit captures aux largeurs 320, 390, 768 et 1440 px,
  plus parcours dans le navigateur intégré. Les captures complètes fiables
  sont celles du navigateur isolé de Lighthouse ; une tentative de capture
  longue du navigateur intégré produisait un assemblage incorrect et a été
  écartée des preuves.
- [Galerie et résultats datés](../../docs/design/landing/recette/2026-09-12/README.md) ;
  [mesures et 80 contrôles navigateur](../../docs/design/landing/recette/2026-09-12/browser-qa.json).

### Registre de fidélité

| Point | État final |
| --- | --- |
| Composition | Huit blocs dans l'ordre, quatre services immédiatement sous l'introduction |
| Identité | Marque en minuscules et onde audio locale à cinq traits dans l'en-tête et le footer |
| Typographie | Titres Georgia/Times, corps Arial/Helvetica, deuxième ligne du H1 en italique |
| Palette | Sept tokens exacts dans chaque thème ; surfaces sombres `#192535` |
| Bandes audio | Flux et résumé distincts, fonds bord à bord, contenu dans le conteneur centré |
| Aperçus | Trois actualités et sept jours fixes, mentions illustratives, accès PWA sans faux lecteur |
| Responsive | Services 4/2/1 colonnes, sections empilées sous 1024 px, menu natif et thème toujours accessible |
| Contenu retiré | Aucune section gamma, photographie ou ancien bloc de présentation dans la page |

### Corrections et écarts intentionnels

- Titres initialement trop gras : graisse allégée. Descriptions et pictogrammes
  desktop initialement trop petits : proportions augmentées. Retours de ligne
  des titres actualités et résumé rapprochés de la maquette.
- La commande de thème est ajoutée au header ; les zones de marque ont une
  cible minimale de 44 px de haut. L'icône seule du thème utilise le texte
  principal, et non le cuivre décoratif.
- Les fonds utilisent les hexadécimaux exacts, sans texture ou dégradé des PNG.
  Le petit texte sur avoine est brun ; sur crème, le taupe reste autorisé.
  Les liens interactifs sombres restent crème avec soulignement.
- Le conteneur maximal de 1180 px et les familles locales priment sur la
  géométrie approximative de l'image générée. Les longueurs de lignes et
  la hauteur totale ne sont donc pas une reproduction pixel pour pixel.
  Le titre du service calendrier peut occuper trois lignes sur desktop.
- Sur mobile, le tableau conserve les sept jours dans une zone défilable
  nommée ; la page entière ne déborde pas. Les ondes sont statiques et décoratives.
- Aucun écart bloquant de composition, de contenu ou d'interaction ne reste
  ouvert dans cette recette. Les écarts ci-dessus sont conformes au référentiel écrit.

### Vérifications fonctionnelles et qualité

- `npm run check:landing` : Astro sans erreur ni avertissement ; 15 tests
  réussis ; build statique et contrôle SEO/ancres/CTA/assets/scripts réussis.
- Thème : première visite claire malgré un système sombre, bascule clavier,
  persistance, restauration, valeur invalide, lecture et écriture refusées,
  absence de JavaScript, cohérence de `aria-pressed`, `color-scheme` et
  `theme-color`. La préférence sombre est appliquée avant l'insertion du `body`.
- Navigation : quatre ancres, menu au clavier, retour en haut, lien d'évitement,
  focus 3 px et défilement du calendrier au clavier. Le menu reste utilisable
  sans JavaScript. Console sans erreur pertinente lors du parcours manuel.
- Chaque rendu est mesuré : page sans débordement horizontal, deux bandes
  pleine largeur, quatre services, sept jours et tokens exacts.
- Destinations normales et origines alternatives `landing.example.test` /
  `pwa.example.test` validées par build, sans accès aux services distants.
- Les scripts manquants, supplémentaires, dupliqués, modifiés, externes ou
  munis d'attributs inattendus sont refusés, même avec l'identifiant du thème.
- Lighthouse : trois runs par combinaison mobile/desktop × clair/sombre,
  **100/100/100/100 sur les douze runs**, y compris la valeur pessimiste.
  Seuils conservés : performance ≥ 80, autres catégories à 100.
- Le HTML final pèse 20 623 octets. Pas de bibliothèque client, d'audio intégré,
  de ressource tierce bloquante ou de maquette documentaire dans les assets publics.

final result: passed

---

## Historique — recette de la composition antérieure à EPIC-038

Les résultats ci-dessous sont conservés tels quels et ne servent pas de preuve
pour la refonte du 12 septembre 2026.

## Référence et méthode

- Concept accepté: composition ImageGen conservée comme référence de travail non versionnée.
- Production: composition éditoriale générée avec ImageGen, puis implémentée en Astro/CSS sans JavaScript applicatif.
- Capture finale vérifiée localement à 1440 × 1024 dans le navigateur intégré; les captures temporaires ne sont pas versionnées.
- Viewports vérifiés: 320 × 800, 390 × 844, 768 × 1024 et 1440 × 1024.

## Fidelity ledger

1. Palette ivoire, encre et cuivre reprise dans les variables CSS.
2. Hero bureau éditorial, sujet placé à droite et espace de lecture à gauche conservés.
3. Hiérarchie serif/sans-serif et titre desktop sur une ligne alignés sur le concept.
4. Navigation, CTA primaire plein et CTA secondaire contour conservés.
5. Iconographie linéaire cuivre et séparateurs fins repris dans les sections.
6. Densité et rayons discrets conservés; aucune grille de cartes décoratives ajoutée.

## Écarts intentionnels

- Le concept ne montrait qu'un aperçu du contenu; l'implémentation ajoute des titres de section explicites pour la structure sémantique et la navigation.
- Le texte final est issu de la configuration produit vérifiée, sans métrique, témoignage, prix ou preuve inventée.
- Sur mobile, l'image précède le texte pour préserver l'identité visuelle; les deux CTA restent visibles dans le premier écran à 320 et 390 px.

## Mismatches corrigés

- Le titre desktop se repliait initialement sur deux lignes: maintien sur une ligne à partir de 1280 px.
- Le contraste du texte de la section « En pratique » était à 4,44:1: couleur assombrie, audit WCAG/Lighthouse repassé à 100.
- Le menu mobile et les largeurs de boutons ont été vérifiés à 320, 390 et 768 px; aucun débordement horizontal.

## Validation

- Menu mobile: ouverture native, liens visibles, ancre `#fonctionnalites` atteinte.
- CTA: uniquement `https://app.filfinance.fr/` et `https://app.filfinance.fr/login`.
- Console: aucune erreur ni aucun avertissement pertinent.
- Lighthouse: l'audit initial a atteint 100/100 dans les quatre catégories sur 3 runs mobile et 3 runs desktop; la gate CI pessimiste exige désormais au moins 80 en performance et conserve 100 en accessibilité, bonnes pratiques et SEO.
- Astro: typecheck, build statique et validation production réussis.

final result: passed
