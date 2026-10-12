# EPIC-038 — Refonte visuelle de la landing et thèmes clair/sombre

## Statut

**Terminé localement le 12 septembre 2026.** La première livraison
documentaire (NEWS-222) a été suivie de l'implémentation et de la recette
Astro (NEWS-223 à NEWS-226). Les deux thèmes, les quatre services et les
comportements documentés sont implémentés et vérifiés.

Preuves : [recette d’EPIC-038](../../apps/landing/design-qa.md),
[captures et résultats datés](../design/landing/recette/2026-09-12/README.md).
`check:landing` passe, avec 15 tests ; la recette navigateur comporte
80 contrôles réussis. Les 12 audits Lighthouse obtiennent 100 dans chacune
des quatre catégories. Ces résultats concernent le working tree local
identifié par ses empreintes ; aucun déploiement n'a été réalisé.

## Objectif métier

Présenter clairement les quatre services de Fil Finance dans une landing
éditoriale sobre, disponible en clair et en sombre : flux audio de news en
français, fil d'actualités financières, résumé audio du jour en français et
calendrier économique hebdomadaire.

Le flux de news accompagne la session de trading ; le résumé audio du jour
constitue un service distinct. La landing présente ces fonctionnalités et
donne accès à la PWA, sans devenir un lecteur public ni une source de données
de marché actualisées.

## Références

- [Référentiel visuel, palettes, textes et comportements](../design/landing/README.md).
- [Maquette claire retenue](../design/landing/maquettes/landing-clair-v1.png).
- [Maquette sombre retenue](../design/landing/maquettes/landing-sombre-v1.png).
- [Architecture landing et PWA](../architecture.md).
- [Roadmap et contrat des tickets](../ROADMAP.md).

Les maquettes sont des références ImageGen, pas des captures d'une refonte
déjà livrée. Les textes, valeurs hexadécimales et règles d'accessibilité du
référentiel priment sur les approximations de rendu des PNG. La commande de
thème est une addition fonctionnelle documentée, absente des maquettes.

## État de départ — avant l'implémentation

- La landing publique se trouve dans `apps/landing`, avec Astro et un build
  statique indépendant de la PWA.
- Les contenus sont centralisés dans `src/config/site.config.ts` ; la page,
  le layout et les styles se trouvent dans `src/pages/index.astro`,
  `src/layouts/BaseLayout.astro` et `src/styles/global.css`.
- La composition actuelle comporte des photographies et d'anciens blocs
  explicatifs. Elle ne possède pas de thème sombre sélectionnable.
- Un composant local `AudioMark.astro` est déjà utilisé dans l'en-tête et le
  pied de page ; ses modifications locales préexistantes doivent être conservées.
- Le validateur de production interdit actuellement tout script autre que
  JSON-LD. Le thème nécessitera une exception strictement limitée à son script.
- Les résultats historiques du document `apps/landing/design-qa.md` ne
  constituent pas une validation de cette nouvelle cible.

## Périmètre

Ordre définitif des blocs :

1. En-tête : icône audio, marque « fil finance », navigation, connexion et thème.
2. Introduction : « Écoutez les marchés. » puis « Restez dans votre session. »
   en italique, texte d'accompagnement et accès au flux dans l'application.
3. Présentation des quatre services immédiatement sous l'introduction.
4. Flux audio : bandeau contrasté sur toute la largeur de la page.
5. Fil d'actualités : texte de présentation et liste illustrative.
6. Résumé audio du jour : section distincte, contrastée sur toute la largeur.
7. Calendrier économique hebdomadaire : exemple sur sept jours.
8. Pied de page : icône audio, marque, copyright et navigation.

Les fonds des deux sections audio vont bord à bord ; leurs contenus restent
alignés dans le même conteneur centré que le reste de la page. La section
gamma, les anciennes photographies et les anciens blocs de présentation sont
absents de la cible. Les textes retenus sont transcrits dans le référentiel.

| Usage | Clair | Sombre |
| --- | --- | --- |
| Fond principal | `#F1E5D3` | `#111827` |
| Surfaces contrastées | `#FBF5EB` | `#192535` |
| Texte principal | `#382E26` | `#FBF5EB` |
| Texte secondaire | `#796B5D` | `#F1E5D3` |
| Accent | `#7B5E45` | `#8B5E3C` |
| Bouton principal | `#7B5E45` | `#714426` |
| Texte du bouton | `#FBF5EB` | `#FBF5EB` |

Sur fond avoine, le petit texte utilise le brun principal : le taupe n'y
produit qu'un contraste de 4,15:1. Le cuivre sombre sert aux accents décoratifs,
pas aux petits libellés interactifs. Les règles complètes figurent dans le
référentiel.

## Comportements et interfaces

- Première visite en clair, indépendamment du système. Deux choix seulement :
  clair et sombre ; commande accessible dans l'en-tête, disponible sur mobile.
- Préférence locale `filfinance.landing.theme`, valeurs `light` ou `dark`,
  appliquée avant le premier affichage. Aucun échange avec le profil ou le
  thème de la PWA.
- Sans JavaScript, sans préférence valide ou si le stockage est inaccessible
  au chargement : clair. Un échec d'enregistrement n'empêche pas la bascule
  pour la visite courante.
- Lecteurs, liste et calendrier explicitement illustratifs. Les accès produit
  utilisent `externalDestinations.home`, la connexion
  `externalDestinations.login`, selon l'environnement déjà configuré.
- Aucun nouveau contrat HTTP/WebSocket, aucune migration de données,
  aucune modification d'authentification ou de route PWA.
- HTML/CSS Astro statique, avec le seul script nécessaire au thème ; le
  validateur conserve le refus de tout autre script applicatif inattendu.

## Hors périmètre

- Implémentation frontend dans la livraison documentaire initiale.
- Données de marché publiques actualisées, connexion aux API, HLS, lecture
  audio réelle, autoplay ou simulation d'une lecture en cours.
- Refonte de la PWA, du backend, des préférences de compte ou de l'authentification.
- Niveaux gamma, graphiques de trading, carnet d'ordres et nouvelles offres.
- Déploiement, changement d'infrastructure, de domaine ou de secrets.

## Tickets et dépendances

| Ticket | Livraison | Dépendance | Statut |
| --- | --- | --- | --- |
| [NEWS-222](../tickets/NEWS-222-referentiel-visuel-landing.md) | Palettes, textes, maquettes et liens documentaires | Aucune | Terminé — documentation vérifiée |
| [NEWS-223](../tickets/NEWS-223-themes-landing-clair-sombre.md) | Tokens, commande, mémorisation et affichage initial | NEWS-222 | Terminé — recette locale |
| [NEWS-224](../tickets/NEWS-224-sections-landing-quatre-services.md) | Sections, contenus et aperçus | NEWS-223 | Terminé — recette locale |
| [NEWS-225](../tickets/NEWS-225-landing-responsive-navigation-accessibilite.md) | Responsive, navigation, accès PWA et accessibilité | NEWS-224 | Terminé — recette locale |
| [NEWS-226](../tickets/NEWS-226-recette-landing-themes.md) | Recette visuelle, fonctionnelle, SEO et performance | NEWS-225 | Terminé — 12 audits réussis |

EPIC-038 réutilise le socle Astro et les destinations de la PWA existants.
Il ne réouvre pas EPIC-016 (renommage du produit) ni EPIC-022 (architecture
et thème de la PWA). Les commentaires des futures modifications de code
restent en français, conformément au standard du dépôt.

## Critères de clôture

- Les huit blocs et les quatre services correspondent au référentiel ; les
  deux bandeaux audio sont pleine largeur et gamma est absent.
- Les deux thèmes fonctionnent à 320, 390, 768 et 1440 px ; aucun débordement
  horizontal de page, sept jours conservés avec défilement limité au tableau
  sur petit écran si nécessaire.
- Première visite claire, bascule, rechargement, préférence invalide,
  stockage indisponible et absence de JavaScript sont vérifiés.
- Navigation clavier, focus visible, annonce du thème, contrastes et liens
  vers la PWA sont validés ; aucun contrôle ne simule une lecture réelle.
- `npm run check:landing` passe, avec contrôle ciblé des scripts autorisés,
  SEO, ancres, destinations et budgets d'assets maintenus.
- Les audits Lighthouse mobile et desktop maintiennent les seuils existants :
  performance ≥ 80 ; accessibilité, bonnes pratiques et SEO à 100.
- NEWS-223 à NEWS-226 disposent de preuves propres à cette implémentation.
  La seule livraison documentaire peut clôturer NEWS-222, jamais l'EPIC.
- Les preuves locales et toute éventuelle preuve de déploiement ultérieur
  restent séparées. Aucun déploiement n'est requis par la livraison documentaire.
