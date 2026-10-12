# NEWS-226 — Recette de la landing et des deux thèmes

## Statut

**Terminé localement le 12 septembre 2026.** `check:landing` passe (15 tests).
La recette navigateur passe 80 contrôles et produit huit captures. Les trois
runs de chacune des quatre configurations Lighthouse donnent 100 en
performance, accessibilité, bonnes pratiques et SEO. Voir le
[compte rendu QA](../../apps/landing/design-qa.md), les
[captures datées](../design/landing/recette/2026-09-12/README.md) et les
[empreintes des sources et du build](../design/landing/recette/2026-09-12/preuves.json).
Aucune preuve de CI distante ou de déploiement n'est revendiquée.

EPIC : [EPIC-038](../epics/EPIC-038-refonte-visuelle-landing-themes-clair-sombre.md).
Source de vérité : [référentiel visuel](../design/landing/README.md).

## Objectif métier

Attester que la landing implémentée correspond aux références approuvées,
fonctionne dans les deux thèmes et conserve ses garanties d'accessibilité,
de SEO et de performance avant toute livraison ultérieure.

## Périmètre

- Recette de la nouvelle composition, des textes, des interactions et thèmes.
- Vérification responsive aux quatre largeurs et contrôle du build de production.
- Audits Lighthouse avec les seuils existants, sans les abaisser.
- Mise à jour du compte rendu QA avec captures et preuves propres à cette refonte.

## Hors périmètre

Déploiement, smoke de services publics, relance backend, modification de la
PWA, données réelles ou validation rétroactive à partir d'anciens résultats.

## Approche technique

- Tester l'artefact Astro construit localement. Identifier la révision testée
  et ses éventuelles modifications locales dans le compte rendu.
- Comparer les deux thèmes aux maquettes retenues et aux spécifications
  écrites, notamment leurs écarts intentionnels : commande de thème ajoutée,
  petits textes contrastés et absence de textures artificielles.
- Compléter `apps/landing/design-qa.md` par une recette datée de l'EPIC-038,
  en conservant les anciens résultats comme historiques. Les nouvelles
  captures sont des preuves d'implémentation, séparées des PNG de conception.
- Exécuter `npm run check:landing` et `npm run lighthouse:landing`. Conserver
  les trois runs mobile et les trois runs desktop des configurations actuelles
  et leurs résultats pessimistes. Pour le sombre, préparer le stockage du
  navigateur d'audit avec `filfinance.landing.theme=dark` avant navigation,
  puis répéter les audits avec les mêmes seuils. Ne pas ajouter de paramètre
  public ou de mode système uniquement pour les tests.
- Vérifier SEO, URL canonique, métadonnées sociales, JSON-LD, sitemap, robots,
  liens configurés et budgets des assets. Les maquettes documentaires ne
  doivent pas apparaître dans les fichiers servis.
- Documenter les défauts trouvés et leurs corrections ; ne clôturer que sur
  l'état final effectivement testé. Commenter les éventuels ajouts de code
  ou de tests en français.

## Critères d'acceptation

- Huit blocs dans le bon ordre, quatre services, deux sections audio pleine
  largeur, icône dans les deux zones de marque, sans gamma ni anciennes photos.
- Rendus clair/sombre vérifiés à 320, 390, 768 et 1440 px ; aucun débordement
  horizontal de page et calendrier complet sur sept jours.
- Cas thème vérifiés : première visite claire malgré système sombre,
  bascule, rechargement avec préférence, valeur invalide, lecture/écriture
  de stockage refusée et JavaScript désactivé.
- Aucun flash de thème incorrect lors d'un retour avec préférence sombre.
  État accessible de la commande cohérent avec le thème affiché.
- Ancres, menu, accès PWA, navigation clavier, focus et contrastes conformes.
- `npm run check:landing` passe, y compris l'autorisation strictement ciblée
  du script de thème et le rejet des scripts inattendus.
- Lighthouse, mobile et desktop, clair et sombre : performance ≥ 80 ;
  accessibilité, bonnes pratiques et SEO à 100, sans affaiblir les gates.
- Les preuves sont datées et associées à la révision finale. Les validations
  locales ne sont pas présentées comme des preuves de déploiement.

## Tests attendus

| Domaine | Scénarios |
| --- | --- |
| Visuel | Deux thèmes × quatre largeurs, comparaison aux références et aux tokens |
| Thème | Défaut clair, deux valeurs mémorisées, invalide, lecture/écriture refusée, sans JS, premier affichage |
| Navigation | Clavier, menu mobile, quatre ancres, marque, lien d'évitement, connexions et accès PWA |
| Aperçus | Mentions illustratives, liens au lieu de faux lecteurs, sept jours, aucune donnée réelle annoncée |
| Build | Typecheck, génération statique, scripts autorisés/refusés, SEO, assets et destinations |
| Qualité | Lighthouse clair/sombre mobile/desktop, console sans erreur pertinente, compte rendu et captures |

## Dépendances

- [NEWS-225](./NEWS-225-landing-responsive-navigation-accessibilite.md).
- Clôture possible d'EPIC-038 seulement après validation de NEWS-223 à NEWS-226 ;
  NEWS-222 seul ne valide pas la refonte. Aucun déploiement n'est inclus ici.
