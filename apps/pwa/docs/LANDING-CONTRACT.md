# LANDING-CONTRACT - Landing statique pour produit PWA

Version : 1.0

Statut : contrat autonome et réutilisable

Langue du contrat : français

## 1. Objet

Ce document fournit un prompt complet pour concevoir, implémenter et valider une
landing page publique destinée à présenter une PWA.

Le contrat est autonome : il peut être copié dans un autre dépôt sans dépendre
du code, des données, des routes internes, de la roadmap ou de la documentation
de la PWA. La landing et la PWA peuvent être développées, construites, testées
et déployées séparément.

## 2. Règle d'indépendance

La landing respecte les règles suivantes :

- aucun import depuis le code source de la PWA ;
- aucun package partagé obligatoire avec la PWA ;
- aucune lecture de base de données, d'API métier ou de stockage applicatif ;
- aucune dépendance envers un document interne de la PWA ;
- aucun couplage au framework utilisé par la PWA ;
- aucune modification du code, du build ou du déploiement de la PWA ;
- les CTA utilisent uniquement des URL absolues fournies dans le manifeste ;
- les contenus publics sont fournis explicitement dans le manifeste ;
- la landing possède ses propres assets, polices, tests et pipeline de build.

L'interface entre les deux produits est donc limitée à des liens HTTPS :

```txt
landing publique -> APP_HOME_URL
landing publique -> APP_LOGIN_URL
landing publique -> APP_SIGNUP_URL, seulement si cette URL est fournie
```

## 3. Manifeste à renseigner

Copier ce bloc, remplacer toutes les valeurs entre `{{...}}`, puis le joindre au
prompt de la section 4.

```yaml
project:
    name: "{{PROJECT_NAME}}"
    category: "{{PRODUCT_CATEGORY}}"
    audience: "{{TARGET_AUDIENCE}}"
    locale: "fr-FR"
    public_origin: "https://{{LANDING_DOMAIN}}"
    pwa_home_url: "https://{{PWA_DOMAIN}}/{{HOME_PATH}}"
    pwa_login_url: "https://{{PWA_DOMAIN}}/{{LOGIN_PATH}}"
    pwa_signup_url: null

positioning:
    promise: "{{SUPPORTED_PRODUCT_PROMISE}}"
    description: "{{SEO_DESCRIPTION_140_TO_160_CHARS}}"
    primary_cta: "{{PRIMARY_CTA_LABEL}}"
    secondary_cta: "{{SECONDARY_CTA_LABEL}}"
    proof_points:
        - "{{SUPPORTED_PROOF_POINT_1}}"
        - "{{SUPPORTED_PROOF_POINT_2}}"
        - "{{SUPPORTED_PROOF_POINT_3}}"

brand:
    direction: "{{VISUAL_DIRECTION}}"
    background: "{{BACKGROUND_HEX}}"
    surface: "{{SURFACE_HEX}}"
    elevated_surface: "{{ELEVATED_SURFACE_HEX}}"
    accent: "{{ACCENT_HEX}}"
    text: "{{TEXT_HEX}}"
    muted_text: "{{MUTED_TEXT_HEX}}"
    border: "{{BORDER_HEX}}"
    heading_font: "{{HEADING_FONT}}"
    body_font: "{{BODY_FONT}}"
    logo_source: "{{LOGO_FILE_OR_NONE}}"
    visual_reference: "{{OPTIONAL_SCREENSHOT_PATH_OR_NONE}}"

content:
    navigation:
        - label: "{{NAV_LABEL_1}}"
          target: "#{{SECTION_ID_1}}"
        - label: "{{NAV_LABEL_2}}"
          target: "#{{SECTION_ID_2}}"
        - label: "{{NAV_LABEL_3}}"
          target: "#{{SECTION_ID_3}}"
    benefits:
        - title: "{{BENEFIT_1_TITLE}}"
          description: "{{BENEFIT_1_DESCRIPTION}}"
        - title: "{{BENEFIT_2_TITLE}}"
          description: "{{BENEFIT_2_DESCRIPTION}}"
        - title: "{{BENEFIT_3_TITLE}}"
          description: "{{BENEFIT_3_DESCRIPTION}}"
    features:
        - title: "{{FEATURE_1_TITLE}}"
          description: "{{FEATURE_1_DESCRIPTION}}"
        - title: "{{FEATURE_2_TITLE}}"
          description: "{{FEATURE_2_DESCRIPTION}}"
        - title: "{{FEATURE_3_TITLE}}"
          description: "{{FEATURE_3_DESCRIPTION}}"
    steps:
        - title: "{{STEP_1_TITLE}}"
          description: "{{STEP_1_DESCRIPTION}}"
        - title: "{{STEP_2_TITLE}}"
          description: "{{STEP_2_DESCRIPTION}}"
        - title: "{{STEP_3_TITLE}}"
          description: "{{STEP_3_DESCRIPTION}}"

media:
    hero_brief: "{{HERO_IMAGE_BRIEF_WITH_SUBJECT_POSITION}}"
    final_cta_brief: "{{FINAL_IMAGE_BRIEF_WITH_SUBJECT_POSITION}}"
    social_image_brief: "{{SOCIAL_IMAGE_BRIEF}}"

constraints:
    static_framework: "Astro"
    package_manager: "{{pnpm_npm_yarn_or_bun}}"
    application_javascript: false
    minimum_width: 320
    lighthouse_categories_minimum: 1.0
    lighthouse_runs_per_profile: 3
```

Les preuves produit doivent être factuelles. Une valeur inconnue ne doit jamais
être remplacée par une statistique, un témoignage, un tarif ou une promesse
inventés.

## 4. Prompt reproductible prêt à copier

Copier le prompt ci-dessous avec le manifeste renseigné.

````text
Tu es un développeur frontend senior spécialisé en Astro, accessibilité, SEO
technique et performance web. Tu dois auditer le dépôt, puis construire une
landing page publique statique pour le produit PWA décrit dans le manifeste
fourni après ce prompt.

OBJECTIF

Livrer une landing page de production, rapide, accessible, responsive et
orientée conversion. Elle doit traduire l'identité du produit sans simuler
l'application, sans inventer de preuve et sans dépendre de son code.

AUTONOMIE OBLIGATOIRE

La landing est un produit autonome.

- Ne modifie aucun fichier de la PWA.
- N'importe aucun module, composant, type, asset ou package depuis la PWA.
- Ne lis aucune base de données et n'appelle aucune API métier.
- Ne dépends d'aucune documentation interne à la PWA.
- Ne déduis pas les routes applicatives depuis son code.
- Utilise uniquement les URL absolues données par pwa_home_url,
  pwa_login_url et, si elle existe, pwa_signup_url.
- Centralise tous les contenus et URL de la landing dans sa propre configuration.
- Conserve un package.json, un build, des assets et des validations propres à la
  landing.

Si une donnée obligatoire du manifeste manque, signale-la avant de l'inventer.
Si une donnée facultative manque, choisis une solution neutre et documente-la.

ÉTAPE 1 - AUDIT DU DÉPÔT

Avant toute modification :

1. Vérifie l'état Git et préserve les modifications existantes.
2. Identifie le gestionnaire de paquets et la structure du workspace.
3. Localise une éventuelle landing existante et ses commandes.
4. Identifie le pipeline CI qui doit accueillir le build statique.
5. Vérifie les versions Node et Astro supportées par le dépôt.
6. Inventorie les assets publics déjà dédiés à la landing.
7. Établis la liste exacte des fichiers que tu vas créer ou modifier.

Ne refactorise pas les autres applications du dépôt. Si aucune landing n'existe,
crée un package autonome dans apps/landing ou dans le dossier public déjà prévu
par le dépôt.

ÉTAPE 2 - ARCHITECTURE ATTENDUE

Utilise Astro en sortie statique, sans hydratation de composant et sans
JavaScript applicatif. La structure cible doit rester proche de :

apps/landing/
  astro.config.mjs
  package.json
  public/
    fonts/
    icons/
    images/
    social/
    robots.txt ou route équivalente
  scripts/
    prepare-assets.mjs
    validate-production-build.mjs
  src/
    assets/source/
    config/site.config.ts
    layouts/BaseLayout.astro
    pages/index.astro
    styles/global.css

N'ajoute des composants Astro séparés que s'ils réduisent réellement la
complexité. N'introduis pas de framework client, de bibliothèque UI lourde ou de
gestion d'état.

ÉTAPE 3 - CONFIGURATION CENTRALE

Crée une configuration typée contenant au minimum :

- nom, titre SEO, description et langue ;
- origine canonique de la landing ;
- URL absolues des CTA vers la PWA ;
- navigation interne ;
- libellés des CTA ;
- bénéfices, fonctionnalités et étapes ;
- chemins de l'image sociale et des icônes ;
- données nécessaires au contenu structuré JSON-LD.

Le template Astro ne doit pas disperser les URL ou le contenu métier dans le
markup. Valide les URL avec l'API URL, pas avec une concaténation de chaînes.

ÉTAPE 4 - DIRECTION UI/UX

Respecte exactement les couleurs, polices et la direction visuelle du manifeste.
Si une référence visuelle est fournie, traite-la comme vérité de composition,
sans recopier ses textes non validés.

La page doit avoir une esthétique propre au domaine du produit. Elle ne doit pas
ressembler à un template SaaS générique.

Règles de composition :

- header compact avec marque, ancres et CTA de connexion ;
- hero plein cadre avec le nom du produit comme H1, promesse en soutien et deux
  CTA ;
- aperçu visible de la section suivante dans le premier viewport ;
- sections en bandes pleine largeur, avec contenu intérieur contraint ;
- pas de carte décorative autour de chaque section ;
- pas de carte imbriquée dans une autre carte ;
- pas de faux téléphone, faux dashboard ou capture d'application inventée ;
- utiliser les cartes uniquement pour des éléments répétés ou un véritable outil
  encadré ;
- rayons contenus entre 4 et 8 px, sauf contrainte de marque explicite ;
- icônes Lucide compilées côté build quand une icône standard existe ;
- aucune forme décorative abstraite, orbe ou dégradé gratuit ;
- aucun texte de démonstration expliquant l'interface à l'utilisateur final.

Ordre recommandé :

1. Header compact.
2. Hero avec CTA principal et secondaire.
3. Trois bénéfices ou piliers.
4. Aperçu code-native d'un usage réel, uniquement à partir de données fournies.
5. Fonctionnalités de guidage ou capacités clés.
6. Parcours en trois étapes.
7. CTA final illustré.
8. Footer compact avec liens existants et année courante.

Adapte cet ordre uniquement si la nature du produit le justifie clairement.

ÉTAPE 5 - CONTENU ET INTÉGRITÉ PRODUIT

- Corrige la langue, les accents et la ponctuation.
- Écris des phrases courtes, concrètes et orientées bénéfice.
- Ne promets que les capacités présentes dans le manifeste.
- N'invente aucun témoignage, client, partenaire, résultat, métrique, tarif,
  disponibilité hors ligne ou niveau de personnalisation.
- N'ajoute aucun lien légal, contact ou réseau social sans URL fournie.
- Un exemple produit doit être marqué comme exemple et provenir des données du
  manifeste.
- Un titre de séance dominant ne doit pas masquer les autres blocs qui composent
  réellement la séance.

ÉTAPE 6 - IMAGES ET POLICES

Utilise deux visuels de production cohérents avec la direction de marque :

- un hero 16:9, sujet placé de façon à préserver la lisibilité du texte ;
- un bandeau final 2:1, avec cadrage complémentaire.

Les visuels doivent montrer le produit, l'usage ou le résultat inspectable, et
non une ambiance générique. Utilise une image fournie, une image sous licence
compatible ou une image générée spécifiquement.

Pour chaque visuel :

- conserver une source haute définition dans src/assets/source ;
- produire des variantes 640, 1024 et 1600 px ;
- produire AVIF et WebP ;
- déclarer width et height ;
- utiliser picture, srcset et sizes ;
- charger le hero en priorité ;
- charger le visuel final en différé ;
- vérifier le cadrage à chaque breakpoint.

Produis une image sociale 1200 x 630 et des icônes adaptées : favicon 32 px,
icône 64 px, icône 192 px et Apple Touch Icon. N'utilise pas une image 1024 px
comme favicon affiché à 32 ou 64 px.

Auto-héberge les polices WOFF2 avec les graisses réellement utilisées. Précharge
uniquement les fichiers nécessaires au premier écran. Applique font-display:
swap et prévois une pile de repli stable.

ÉTAPE 7 - RESPONSIVE

La page doit être exploitable sans débordement horizontal à partir de 320 px.

- Desktop de référence : 1440 x 1024.
- Tablette : 768 x 1024.
- Mobile : 390 x 844.
- Largeur minimale : 320 x 800.

Sur mobile :

- utiliser un menu natif details/summary ;
- rendre les CTA principaux pleine largeur ;
- conserver des cibles tactiles d'au moins 44 x 44 px ;
- laisser les badges et listes revenir proprement à la ligne ;
- ne jamais réduire la typographie selon la largeur du viewport ;
- garantir que le mot le plus long tient dans son conteneur ;
- empêcher tout chevauchement entre texte, image et commandes.

ÉTAPE 8 - ACCESSIBILITÉ

Respecte WCAG 2.2 niveau AA au minimum :

- un seul H1 ;
- structure header/nav/main/section/footer ;
- lien d'évitement vers le contenu ;
- ordre de tabulation logique ;
- focus visible avec contraste suffisant ;
- cibles interactives de 44 px minimum ;
- noms accessibles pour les menus et CTA ;
- textes alternatifs utiles pour les images porteuses de sens ;
- alt vide pour les images strictement décoratives ;
- contraste AA des textes et composants ;
- aucune information portée uniquement par la couleur ;
- prefers-reduced-motion pour neutraliser les mouvements ;
- navigation et conversion possibles sans JavaScript.

ÉTAPE 9 - SEO

Ajoute et valide :

- title unique et descriptif ;
- meta description ;
- canonical absolu ;
- robots index/follow ;
- Open Graph avec URL absolues ;
- image Open Graph 1200 x 630 ;
- Twitter summary_large_image ;
- langue et locale cohérentes ;
- theme-color ;
- favicon et Apple Touch Icon ;
- robots.txt ;
- sitemap.xml ;
- JSON-LD WebSite ;
- JSON-LD SoftwareApplication seulement avec des propriétés factuelles.

Le HTML doit contenir un seul H1, des ancres existantes et des CTA dont les URL
correspondent exactement au manifeste.

ÉTAPE 10 - PERFORMANCE

Objectif : 100 dans Performance, Accessibilité, Bonnes pratiques et SEO sur le
build local de production.

Contraintes :

- sortie HTML statique ;
- aucun JavaScript applicatif ;
- CSS critique disponible au premier rendu ;
- aucune ressource tierce bloquante ;
- images responsives et dimensionnées ;
- aucun layout shift visible ;
- pas de préchargement inutile ;
- pas de dépendance réseau pour les polices ou icônes ;
- budgets d'assets contrôlés par le validateur.

Ajoute trois scripts :

- lighthouse:mobile ;
- lighthouse:desktop ;
- lighthouse:ci.

Configure trois exécutions par profil avec `minScore: 0.9` pour la performance
et `minScore: 1` pour l'accessibilité, les bonnes pratiques et le SEO. Utilise
l'agrégation `pessimistic` afin qu'une seule exécution sous son seuil fasse
échouer la CI.

ÉTAPE 11 - VALIDATEUR DE BUILD

Crée un script qui échoue si l'un des contrats suivants est rompu :

- index.html absent ;
- title ou description absents ;
- canonical, robots, Open Graph ou Twitter incomplets ;
- image sociale non absolue ou dimensions incorrectes ;
- nombre de H1 différent de un ;
- JSON-LD absent ou JSON invalide ;
- URL de CTA différente du manifeste ;
- ancre interne sans cible ;
- asset référencé absent ;
- image dépassant le budget défini ;
- script applicatif inattendu dans le HTML ;
- robots.txt ou sitemap.xml absent.

Utilise un parseur HTML ou des API structurées lorsque disponibles. Évite les
validations fragiles fondées uniquement sur des expressions régulières.

ÉTAPE 12 - CI

Ajoute un job dédié au pipeline existant, sans modifier les jobs de la PWA.

Le job doit :

1. installer les dépendances de la landing ;
2. préparer les assets ;
3. produire le build statique ;
4. exécuter le validateur ;
5. servir le dossier de production localement ;
6. lancer Lighthouse mobile puis desktop avec Chromium ;
7. conserver les rapports comme artefacts en cas d'échec ;
8. bloquer le déploiement de la landing si un score est inférieur à 100.

Ne fais dépendre ce job d'aucun build, secret ou artefact de la PWA.

ÉTAPE 13 - QA NAVIGATEUR

Teste le build de production dans un vrai navigateur aux quatre viewports.

Vérifie :

- menu mobile ouvert et fermé ;
- ancres ;
- tous les CTA ;
- navigation clavier ;
- focus visible ;
- absence de débordement ;
- absence de chevauchement ;
- chargement des images ;
- dimensions réservées ;
- absence d'erreur console ;
- cadrage des visuels ;
- cohérence typographique et colorimétrique.

Si une référence visuelle est fournie, compare composition, couleurs, polices,
espacements, images, icônes et contenu. Corrige les écarts P0 à P2.

Crée design-qa.md avec les preuves, viewports, écarts corrigés et résultats
Lighthouse. La dernière ligne doit être exactement :

final result: passed

Ne l'écris que lorsque tous les contrôles sont réellement validés.

ÉTAPE 14 - COMMANDES ET LIVRABLES

Expose au minimum des commandes équivalentes à :

- dev ;
- build ;
- preview ;
- validate ;
- lighthouse:mobile ;
- lighthouse:desktop ;
- lighthouse:ci.

Livre :

- le code Astro statique ;
- la configuration centralisée ;
- les assets sources et optimisés ;
- les métadonnées et données structurées ;
- le validateur de production ;
- les configurations Lighthouse ;
- le job CI indépendant ;
- design-qa.md ;
- un court compte rendu final.

CRITÈRES D'ACCEPTATION

La tâche est terminée uniquement si :

- la landing se construit seule ;
- la PWA n'a subi aucune modification ;
- aucun import ou package partagé avec la PWA n'est requis ;
- les CTA utilisent les URL absolues du manifeste ;
- le contenu ne contient aucune affirmation inventée ;
- la page fonctionne sans JavaScript applicatif ;
- le rendu est correct de 320 à 1440 px ;
- le clavier, le focus et le menu mobile sont validés ;
- le validateur SEO et liens passe ;
- les six exécutions Lighthouse atteignent au moins 90/100 en performance et
  100/100 dans les trois autres catégories ;
- design-qa.md se termine par final result: passed.

FORMAT DU COMPTE RENDU FINAL

Indique :

1. URL locale de prévisualisation.
2. Résumé des changements.
3. Fichiers principaux créés ou modifiés.
4. Résultats build, validation et Lighthouse.
5. Viewports testés.
6. Éventuelles limites factuelles restantes.

MANIFESTE PROJET

Remplace toutes les valeurs entre {{...}} avant de commencer. Une valeur
facultative absente doit être remplacée par null, jamais par une donnée inventée.

```yaml
project:
  name: "{{PROJECT_NAME}}"
  category: "{{PRODUCT_CATEGORY}}"
  audience: "{{TARGET_AUDIENCE}}"
  locale: "fr-FR"
  public_origin: "https://{{LANDING_DOMAIN}}"
  pwa_home_url: "https://{{PWA_DOMAIN}}/{{HOME_PATH}}"
  pwa_login_url: "https://{{PWA_DOMAIN}}/{{LOGIN_PATH}}"
  pwa_signup_url: null

positioning:
  promise: "{{SUPPORTED_PRODUCT_PROMISE}}"
  description: "{{SEO_DESCRIPTION_140_TO_160_CHARS}}"
  primary_cta: "{{PRIMARY_CTA_LABEL}}"
  secondary_cta: "{{SECONDARY_CTA_LABEL}}"
  proof_points:
    - "{{SUPPORTED_PROOF_POINT_1}}"
    - "{{SUPPORTED_PROOF_POINT_2}}"
    - "{{SUPPORTED_PROOF_POINT_3}}"

brand:
  direction: "{{VISUAL_DIRECTION}}"
  background: "{{BACKGROUND_HEX}}"
  surface: "{{SURFACE_HEX}}"
  elevated_surface: "{{ELEVATED_SURFACE_HEX}}"
  accent: "{{ACCENT_HEX}}"
  text: "{{TEXT_HEX}}"
  muted_text: "{{MUTED_TEXT_HEX}}"
  border: "{{BORDER_HEX}}"
  heading_font: "{{HEADING_FONT}}"
  body_font: "{{BODY_FONT}}"
  logo_source: "{{LOGO_FILE_OR_NONE}}"
  visual_reference: "{{OPTIONAL_SCREENSHOT_PATH_OR_NONE}}"

content:
  navigation:
    - label: "{{NAV_LABEL_1}}"
      target: "#{{SECTION_ID_1}}"
    - label: "{{NAV_LABEL_2}}"
      target: "#{{SECTION_ID_2}}"
    - label: "{{NAV_LABEL_3}}"
      target: "#{{SECTION_ID_3}}"
  benefits:
    - title: "{{BENEFIT_1_TITLE}}"
      description: "{{BENEFIT_1_DESCRIPTION}}"
    - title: "{{BENEFIT_2_TITLE}}"
      description: "{{BENEFIT_2_DESCRIPTION}}"
    - title: "{{BENEFIT_3_TITLE}}"
      description: "{{BENEFIT_3_DESCRIPTION}}"
  features:
    - title: "{{FEATURE_1_TITLE}}"
      description: "{{FEATURE_1_DESCRIPTION}}"
    - title: "{{FEATURE_2_TITLE}}"
      description: "{{FEATURE_2_DESCRIPTION}}"
    - title: "{{FEATURE_3_TITLE}}"
      description: "{{FEATURE_3_DESCRIPTION}}"
  steps:
    - title: "{{STEP_1_TITLE}}"
      description: "{{STEP_1_DESCRIPTION}}"
    - title: "{{STEP_2_TITLE}}"
      description: "{{STEP_2_DESCRIPTION}}"
    - title: "{{STEP_3_TITLE}}"
      description: "{{STEP_3_DESCRIPTION}}"

media:
  hero_brief: "{{HERO_IMAGE_BRIEF_WITH_SUBJECT_POSITION}}"
  final_cta_brief: "{{FINAL_IMAGE_BRIEF_WITH_SUBJECT_POSITION}}"
  social_image_brief: "{{SOCIAL_IMAGE_BRIEF}}"

constraints:
  static_framework: "Astro"
  package_manager: "{{pnpm_npm_yarn_or_bun}}"
  application_javascript: false
  minimum_width: 320
  lighthouse_categories_minimum: 1.0
  lighthouse_runs_per_profile: 3
```
````

## 5. Contrat de fichiers

Le package landing doit pouvoir être extrait du monorepo avec ses seuls fichiers
et son lockfile. Les chemins ci-dessous sont indicatifs, mais chaque
responsabilité doit rester couverte.

| Responsabilité            | Fichier recommandé                                 |
| ------------------------- | -------------------------------------------------- |
| Contenu, SEO et URL       | `src/config/site.config.ts`                        |
| Structure HTML et JSON-LD | `src/layouts/BaseLayout.astro`                     |
| Composition de page       | `src/pages/index.astro`                            |
| Thème et responsive       | `src/styles/global.css`                            |
| Sources haute définition  | `src/assets/source/`                               |
| Assets publics optimisés  | `public/images/`, `public/icons/`, `public/fonts/` |
| Préparation des assets    | `scripts/prepare-assets.mjs`                       |
| Validation de production  | `scripts/validate-production-build.mjs`            |
| Lighthouse mobile         | `.lighthouserc.mobile.cjs`                         |
| Lighthouse desktop        | `.lighthouserc.desktop.cjs`                        |
| QA visuelle               | `design-qa.md`                                     |

## 6. Contrat d'interface URL

Les URL de la PWA sont des entrées de configuration, jamais des dépendances de
code. Elles doivent être absolues et validées au build.

Exemple générique :

```ts
const requireHttpsUrl = (value: string, name: string) => {
    const url = new URL(value);
    if (url.protocol !== "https:") {
        throw new Error(`${name} doit utiliser HTTPS.`);
    }
    return url.toString();
};

export const externalDestinations = {
    home: requireHttpsUrl(
        import.meta.env.PUBLIC_PWA_HOME_URL,
        "PUBLIC_PWA_HOME_URL",
    ),
    login: requireHttpsUrl(
        import.meta.env.PUBLIC_PWA_LOGIN_URL,
        "PUBLIC_PWA_LOGIN_URL",
    ),
};
```

Cette configuration ne doit pas :

- importer un routeur applicatif ;
- partager des types avec la PWA ;
- supposer un domaine, un sous-chemin ou un framework ;
- contourner une valeur manquante avec une URL inventée.

## 7. Definition of Done synthétique

- [ ] Le manifeste est complet et factuel.
- [ ] La landing se construit sans la PWA.
- [ ] Aucun fichier de la PWA n'est modifié.
- [ ] Aucun import ou package partagé n'est nécessaire.
- [ ] Les CTA utilisent uniquement des URL absolues configurées.
- [ ] Le contenu est centralisé.
- [ ] La page ne contient aucun JavaScript applicatif.
- [ ] Les assets sont locaux, responsifs et dimensionnés.
- [ ] Le rendu tient sans débordement à 320, 390, 768 et 1440 px.
- [ ] Le clavier, le focus, les contrastes et la réduction des mouvements passent.
- [ ] Les métadonnées SEO, JSON-LD, robots et sitemap sont validés.
- [ ] Les liens et ancres sont contrôlés au build.
- [ ] Lighthouse passe trois fois en mobile et trois fois en desktop à 100.
- [ ] Le job CI de la landing ne dépend d'aucun job ou artefact de la PWA.
- [ ] `design-qa.md` se termine par `final result: passed`.
