# Pages légales pour un couple landing + app

Version du guide : 1.0 — 12 septembre 2026. Adaptation Fil Finance : voir le [manifeste](./MANIFESTE.md) et la [recette locale](./RECETTE-FILFINANCE.md).

Ce dossier explique le dispositif implémenté dans ACREA Fitness et fournit une méthode pour le reproduire dans un autre projet. Il peut être copié intégralement dans un autre dépôt. Les références au code ACREA servent à comprendre l'origine du dispositif ; les modèles restent utilisables sans ce code.

Le socle à reproduire est simple : **la landing héberge les documents publics, l'app pointe vers leurs URL, et la CI vérifie les pages générées avant publication**. Les textes doivent décrire les données et le fonctionnement du nouveau service.

## Utilisation du dossier

1. Lire ce guide pour comprendre l'existant et les décisions à prendre.
2. Consulter le [manifeste Fil Finance](./MANIFESTE.md), qui fixe le périmètre retenu ici. Le [manifeste ACREA original](./MANIFESTE-ACREA.md) conserve l’exemple source. Pour un autre projet, remplir une copie de [MANIFESTE-MODELE.md](./MANIFESTE-MODELE.md).
3. Utiliser [IMPLEMENTATION.md](./IMPLEMENTATION.md) pour construire les pages, les liens et les contrôles.
4. Fournir [PROMPT-REUTILISABLE.md](./PROMPT-REUTILISABLE.md) et le manifeste à un développeur ou à un agent de code.
5. Exécuter la recette de ce guide et conserver les résultats avant publication.

Le guide distingue trois niveaux : **observé dans le code ACREA**, **modèle proposé pour réutilisation**, **vérification restant à faire**. Il documente une implémentation technique ; il ne certifie pas la conformité d'ACREA ou du futur service.

## 1. Ce qui existe dans ACREA

Lecture du dépôt au 12 septembre 2026, commit de référence `ff3a3b32`. Aucun audit navigateur ni déploiement de production n'a été exécuté pour rédiger ce dossier. Le jalon M6 de `docs/ROADMAP.md` est encore `IN_PROGRESS`.

### Routes et points d'entrée

| Document | Route publique sur la landing | Footer landing | Inscription app |
| --- | --- | --- | --- |
| Mentions légales | `/mentions-legales/` | Oui | Pas de lien direct dans le helper actuel |
| Politique de confidentialité | `/politique-de-confidentialite/` | Oui | Oui |
| Conditions générales d'utilisation | `/conditions-generales-utilisation/` | Oui | Oui |
| Cookies et traceurs | `/cookies/` | Oui | Oui |

La landing de référence est `https://acrea-fitness.fr`, l'app `https://app.acrea-fitness.fr`. Il s'agit des destinations configurées dans le dépôt, pas d'une preuve de disponibilité distante.

```text
Landing publique
  accueil → footer → 4 pages HTML publiques
  page légale → footer → autres documents + retour accueil

App / inscription
  information avant création du compte
  → URL absolues des documents de la landing
  → aucune copie des textes dans l'app

CI
  informations de publication → build landing → validation HTML → artefact
  origine de la landing → build app → artefact
  publication conditionnée aux jobs requis
```

### Carte des fichiers source

Tous les chemins de ce tableau sont relatifs à la racine du dépôt ACREA.

| Fichier | Responsabilité et éléments à reprendre |
| --- | --- |
| `apps/landing/src/config/site.config.ts` | `legalPaths`, `legalNavigation`, `legalConfig`, origines landing/app et données de l'éditeur |
| `apps/landing/src/layouts/BaseLayout.astro` | Document HTML, langue, titre, description, canonical, métadonnées et CSS |
| `apps/landing/src/layouts/LegalLayout.astro` | En-tête, retour accueil, lien d'évitement, article, date et footer communs |
| `apps/landing/src/components/SiteFooter.astro` | Liens légaux sur l'accueil et les documents ; correction des ancres avec `useHomepageAnchors` |
| `apps/landing/src/pages/mentions-legales.astro` | Éditeur, domiciliation, directeur de publication, hébergeur, contact, propriété intellectuelle |
| `apps/landing/src/pages/politique-de-confidentialite.astro` | Données et finalités, bases légales, destinataires, conservation, droits, paragraphe santé |
| `apps/landing/src/pages/conditions-generales-utilisation.astro` | Site et app, compte, sécurité, usage sportif, propriété, évolution et droit applicable |
| `apps/landing/src/pages/cookies.astro` | Position sans traceurs non essentiels configurés, stockages techniques et évolution future |
| `apps/landing/src/styles/global.css` | Bloc `.legal-*`, dépendances communes et adaptations aux seuils 1040 et 700 px |
| `apps/landing/src/pages/sitemap.xml.ts` | Accueil et `Object.values(legalPaths)` |
| `apps/landing/src/pages/robots.txt.ts` | Exploration autorisée et URL du sitemap |
| `apps/landing/astro.config.mjs` | Sortie statique et slash final systématique |
| `apps/landing/public/_headers` | Politique de sécurité appliquée par l'hébergement compatible |
| `apps/landing/scripts/validate-production-build.mjs` | Contrôles des fichiers générés et blocage des informations juridiques manquantes |
| `apps/landing/.env.example` | Cinq informations publiques de publication ; contient des valeurs ACREA à remplacer |
| `apps/pwa/src/shared/legal/legalLinks.ts` | Construction des trois URL légales utilisées dans l'inscription |
| `apps/pwa/src/shared/config/runtimeEnv.ts` | Lecture des variables publiques de build et surcharge Web à l'exécution |
| `apps/pwa/src/features/auth/components/AuthGate.tsx` | Mention d'information et liens avec `Linking.openURL` |
| `apps/pwa/src/features/pwa/services/pwaAnalytics.ts` | Envoi optionnel d'événements si un endpoint est configuré |
| `.gitlab-ci.yml` | `build_landing`, `build_pwa`, contrôles bloquants et dépendances de `release_production` |

### Fonctionnement exact

La landing est statique : chaque document devient un fichier `dist/<route>/index.html`. Les textes sont écrits dans les quatre fichiers Astro. Seules certaines informations sont centralisées ; le nom de marque, des titres, le copyright, les coordonnées d'hébergement et des formulations restent spécifiques à ACREA.

Le layout légal utilise le même thème que l'accueil. Il affiche un seul `h1` fourni par la page, des sections `h2`, puis la date commune `25 août 2026`. Son footer transforme `#methode` en `/#methode`, afin de revenir à la bonne section de l'accueil depuis une autre page. Les documents sont indexables et disposent chacun d'un canonical propre.

Dans l'app, les liens sont affichés dans le formulaire d'authentification lorsque le service est configuré et qu'aucune session n'est active. Ils ne sont pas affichés dans la branche « session locale ». Lorsque l'utilisateur est connecté, `AuthGate` rend ses enfants. Le code examiné ne fournit pas de rubrique légale permanente dans les paramètres.

La mention d'inscription indique que la personne reconnaît avoir lu les CGU et la politique de confidentialité. Le bouton appelle `signUp(credentials)` : **ce dispositif n'ajoute ni case d'acceptation, ni version de CGU, ni preuve d'acceptation enregistrée côté serveur**.

`getLegalLinks()` lit `EXPO_PUBLIC_LANDING_ORIGIN`, puis utilise le domaine ACREA comme valeur de secours. `readPublicEnv()` privilégie une valeur non vide de `window.__ACREA_FITNESS_ENV__`, puis la valeur de build. Ce mécanisme est propre à cette app.

### Contrôles réellement présents

Le job `build_landing` exige cinq variables non vides, exporte `PUBLIC_LEGAL_PUBLICATION_ENFORCED=true`, construit la landing et lance `validate`.

Le validateur vérifie notamment :

- l'existence du HTML des quatre pages ;
- leur titre, canonical, meta robots et unique `h1` ;
- les quatre liens légaux et les ancres vers l'accueil dans chaque document ;
- la présence des routes dans le sitemap ;
- l'absence du texte de remplacement « À renseigner avant publication en production » lorsque le mode strict est activé.

Il effectue aussi des contrôles propres à l'accueil ACREA : CTA, copyright, JSON-LD, assets, poids des images et fichier `_headers`. Le refus de JavaScript applicatif porte sur le HTML de l'accueil ; ne pas présenter ce test comme un scan de tous les scripts de toutes les pages.

Le job `build_pwa` refuse une valeur non vide pour `EXPO_PUBLIC_PWA_ANALYTICS_ENDPOINT`. Pourtant, le code d'envoi existe et fonctionne dès qu'un endpoint est fourni. La surcharge Web à l'exécution et les services injectés par l'hébergeur doivent donc également être vérifiés. Un contrôle de variable CI ne prouve pas à lui seul l'absence de collecte sur le site publié.

Les builds ont des sorties distinctes, mais le pipeline ACREA n'est pas entièrement découplé : `.node_job` installe les dépendances des deux frontends et `release_production` attend puis publie les deux artefacts. Un nouveau projet peut reprendre le contrat d'URL tout en conservant des pipelines séparés.

## 2. Points ACREA à ne pas propager aveuglément

| Observation | Conséquence pour la réutilisation |
| --- | --- |
| `.gitlab-ci.yml` et `.env.example` utilisent `support@acra-fitness.fr`, alors que `docs/architecture/MAIL-PRODUCTION.md` décrit `support@acrea-fitness.fr` | Confirmer le contact réel et sa réception. Le guide ne tranche pas la validité des boîtes et ne les modifie pas. |
| Les cinq variables sont définies dans le YAML, tandis que la roadmap évoque aussi des variables GitLab | Lire les valeurs effectives du pipeline avec leurs priorités ; le contenu du dépôt ne prouve pas le HTML publié. |
| La date, le nom, l'hébergeur, l'année et les domaines sont en partie codés en dur | Faire un inventaire complet des remplacements, pas seulement un changement de variable de domaine. |
| Une valeur non vide suffit au premier contrôle CI | « TODO », une fausse adresse ou une durée irréalisable peuvent passer ; ajouter une revue factuelle et un refus des placeholders. |
| Les durées ACREA incluent une suppression ou anonymisation annoncée sous 30 jours | Ne pas reprendre cette promesse sans procédure couvrant backend, sauvegardes et copies locales. La page n'implémente pas la suppression. |
| Trois liens dans l'inscription ; aucun en mode invité via ce composant | Prévoir les quatre documents depuis un point d'entrée permanent de la nouvelle app, y compris sans compte si elle collecte des données. |
| Pas d'acceptation contractuelle versionnée ajoutée par cette intégration | Si nécessaire au nouveau service, concevoir le parcours et la preuve backend comme un travail distinct. |
| M6 conserve ouverts l'audit anonyme, la recette du parcours et la validation juridique | Ne pas présenter ce socle comme une conformité déjà entièrement validée. |

## 3. Décisions à prendre pour le nouveau projet

### Où publier les textes ?

Conserver une source éditoriale unique sur la landing, publique sans connexion. Les liens de l'app sont absolus ; ils ne doivent pas pointer vers `/cookies/` sur le domaine de l'app par accident. La politique précise explicitement les domaines et applications couverts.

Une navigation entre domaines n'exige pas d'ouvrir CORS, de partager les cookies de session ou d'envoyer un jeton utilisateur. Ne pas ajouter de compte, d'email ou de token aux URL légales.

Pour une app native, ouvrir le navigateur ou un écran public approprié. Pour une PWA, tester le retour au formulaire et le comportement de l'app installée. Les liens externes ne garantissent pas la disponibilité hors connexion : si nécessaire, prévoir une copie versionnée avec source et date, ainsi qu'une procédure de mise à jour. Cette copie est une extension du modèle ACREA.

### Quels documents ?

Le socle ACREA contient quatre pages. Il s'agit d'une organisation de contenu réutilisable, pas d'une liste universelle d'obligations. Les mentions à publier dépendent notamment du statut de l'éditeur et de son activité. Vérifier le cas [entrepreneur individuel](https://entreprendre.service-public.gouv.fr/vosdroits/F31228) ou [société](https://entreprendre.service-public.gouv.fr/vosdroits/F37351).

Pour une vente à distance à des consommateurs, traiter aussi les CGV et l'information précontractuelle, notamment prix, paiement, exécution, rétractation et médiation selon l'offre. Les CGU du service gratuit ne suffisent pas à décrire ce parcours. Voir [les règles du commerce en ligne](https://entreprendre.service-public.gouv.fr/vosdroits/F23455).

### Information, contrat et consentement

Séparer dans l'interface et dans les données :

| Mécanisme | But | Traduction technique |
| --- | --- | --- |
| Information sur les données | Expliquer le traitement au moment de la collecte | Notice courte et lien vers une politique détaillée |
| Acceptation des CGU, si retenue | Formaliser l'accord contractuel | Action explicite adaptée et, si nécessaire, version + preuve backend |
| Consentement à un traitement facultatif | Autoriser une finalité déterminée | Choix distinct, non précoché, retrait possible, état propre à la finalité |
| Choix sur les traceurs | Autoriser les accès au terminal soumis à consentement | Blocage préalable, choix, mémorisation et retrait |

Une reconnaissance de lecture de la politique ne constitue pas un consentement général à toutes les utilisations des données. Le consentement doit notamment être libre, spécifique, éclairé et univoque. Source : [CNIL — consentement](https://www.cnil.fr/fr/les-bases-legales/consentement).

Si l'acceptation versionnée est demandée, définir au minimum l'identifiant du document, sa version immuable, l'utilisateur, la date serveur et l'action effectuée. Vérifier la version côté serveur ; définir conservation, accès aux preuves, archivage du texte accepté et traitement des nouvelles versions. Ne pas collecter automatiquement IP ou user-agent sans finalité justifiée. Ce mécanisme n'est pas livré par les liens ACREA.

## 4. Préparer les contenus

Le [modèle de manifeste](./MANIFESTE-MODELE.md) est la fiche de collecte à remettre au responsable du projet. Le [manifeste ACREA rempli](./MANIFESTE-ACREA.md) fournit un exemple sourcé, avec ses incertitudes. Chaque valeur doit avoir une source ou un responsable de validation.

### Mentions légales

Préparer l'identité juridique réelle, la forme/statut, l'adresse professionnelle, les contacts, le responsable de publication et l'hébergeur réel. Ajouter selon le statut et l'activité les informations d'immatriculation, de capital, de TVA ou de profession réglementée applicables. Ne pas prendre le nom commercial pour l'identité juridique ni les cinq variables d'ACREA pour une liste exhaustive. Références : [entreprise individuelle](https://entreprendre.service-public.gouv.fr/vosdroits/F31228), [société](https://entreprendre.service-public.gouv.fr/vosdroits/F37351).

### Politique de confidentialité

Décrire l'éditeur responsable, les finalités et bases légales, les données obligatoires ou facultatives et les conséquences d'un refus, les destinataires, les durées, les droits et le contact. Selon les traitements, préciser transferts et garanties, profilage/décisions automatisées, retrait du consentement et sources de collecte indirecte. La notice courte doit être présente au moment de la collecte et renvoyer vers le détail. Source : [CNIL — information et transparence](https://www.cnil.fr/fr/conformite-rgpd-information-des-personnes-et-transparence).

Pour préparer la rédaction, compléter une ligne par traitement dans le manifeste : compte, support, données métier, paiement, emails, sécurité, statistiques ou IA seulement s'ils existent. Distinguer l'hébergement de la landing de celui des données de l'app. Ne pas affirmer « données en France » à partir du seul emplacement du frontend.

Pour un service sportif, la phrase « ceci n'est pas un avis médical » ne détermine pas la qualification des données. Des restrictions, douleurs ou croisements de mesures peuvent révéler un état de santé. Évaluer les données réellement utilisées avant de reprendre le paragraphe ACREA. Source : [CNIL — définition des données de santé](https://www.cnil.fr/fr/quest-ce-ce-quune-donnee-de-sante).

### CGU

Utiliser comme plan éditorial : périmètre du site et de l'app, public admis, création/suppression de compte, sécurité, fonctionnalités, usages autorisés, propriété intellectuelle, suspension, disponibilité, responsabilité, évolution des conditions, contact et droit applicable. Remplacer les clauses sportives par les règles du nouveau produit. Adapter les restrictions au public et aux fonctionnalités ; faire examiner les clauses qui engagent l'éditeur ou limitent les droits des utilisateurs.

### Cookies et stockages

ACREA retient une landing sans traceurs non essentiels configurés et sans bandeau. Reprendre cette position uniquement après inventaire du nouveau produit. La CNIL prévoit des exemptions pour certains accès strictement nécessaires ; la règle concerne aussi d'autres techniques que les cookies HTTP. Source : [CNIL — cookies et autres traceurs](https://cnil.fr/fr/cookies-et-autres-traceurs/que-dit-la-loi).

Inventorier cookies, localStorage, sessionStorage, IndexedDB, cache/service worker, SDK, contenus embarqués, appels réseau et ajouts de l'hébergeur. Classer chaque usage par finalité : un stockage local n'est pas automatiquement exempté.

Si des traceurs soumis à consentement sont ajoutés, prévoir le blocage avant choix, un refus aussi simple que l'acceptation, des réglages par finalité et un accès permanent au retrait. Vérifier que le retrait arrête effectivement les lectures/écritures concernées. Source : [CNIL — recommandation pratique consolidée](https://www.cnil.fr/sites/default/files/2026-01/recommandation_cookies_consolidee.pdf).

Pour landing et app sur deux origines, tester leurs choix séparément. Ne pas supposer qu'un localStorage ou un choix sur la landing est disponible dans l'app. Toute synchronisation doit avoir un périmètre explicite. Le socle documenté ici n'implémente pas de gestionnaire de consentement.

## 5. Procédure de reproduction

1. **Inventorier** le dépôt cible : frameworks, routage, formulaire de création de compte, footer, configuration publique, pipeline et artefacts.
2. **Remplir** le manifeste, l'inventaire des traitements et celui des stockages. Marquer les données inconnues ; ne pas en inventer.
3. **Fixer** les quatre chemins publics, les origines par environnement et la convention de slash. Si le site est sous un préfixe de chemin, adapter toutes les URL ; les exemples supposent un hébergement à la racine.
4. **Créer** le layout légal, la navigation commune, les quatre contenus et la date/version éditoriale.
5. **Intégrer** les liens à l'accueil, aux documents et à l'app avant collecte ; ajouter un accès permanent dans l'app comme amélioration du modèle.
6. **Configurer** canonical, sitemap, robots et diffusion statique. Réserver les règles SPA au domaine de l'app pour éviter une fausse page légale qui affiche l'accueil.
7. **Installer** les contrôles de publication : variables, placeholders, HTML réel, URL et absence d'activation de traceurs non autorisée.
8. **Recetter** le build puis l'environnement publié. Vérifier les données et contenus avec leur responsable.
9. **Archiver** la version des textes, le commit, les résultats et les limites de validation.

Le modèle de `docs/architecture/LANDING-CONTRACT.md` d'ACREA interdit les modifications de PWA dans son propre périmètre « landing seule ». Ce dossier décrit un autre périmètre : l'intégration légale du couple landing + app, qui inclut explicitement les points d'entrée légaux dans l'app.

## 6. Recette reproductible

### Matrice minimale

| Contrôle | Action | Résultat attendu |
| --- | --- | --- |
| Pages publiques | Ouvrir les quatre URL en session anonyme | HTTP 200, document correspondant, aucun écran de connexion |
| Absence de soft 404 | Comparer titre et `h1` de chaque URL, recharger directement | Chaque document possède son propre contenu ; aucun repli sur l'accueil |
| Maillage | Depuis l'accueil puis chacune des quatre pages, suivre les liens légaux | Tous les documents restent accessibles |
| Retour accueil | Suivre logo, retour et liens de sections | Accueil ou `/#section` existante |
| SEO | Lire HTML, sitemap et robots | Canonical public propre, routes cohérentes, pas de domaine ACREA dans le nouveau projet |
| Inscription | Ouvrir les liens sans soumettre de compte | Textes accessibles avant envoi des données ; formulaire récupérable au retour |
| App connectée/invitée | Ouvrir le point d'entrée permanent choisi | Documents accessibles dans les états prévus |
| Mobile et clavier | Tester 320, 390, 768, 1440 px, Tab, activation et retour | Aucun débordement, texte lisible, focus visible, liens activables |
| App installée/native | Ouvrir le document puis revenir | Comportement cohérent et message exploitable en cas d'échec |
| Hors connexion | Tester le lien externe | Limite clairement gérée ; ne pas promettre une copie hors ligne inexistante |
| Informations | Comparer HTML construit et manifeste approuvé | Pas de valeur manquante, contact exact, promesses opérationnelles vérifiées |
| CI négative | Retirer une donnée requise puis laisser un placeholder | Publication refusée dans les deux cas par les contrôles cibles |
| Traceurs désactivés | Navigation anonyme sur les deux origines, stockage et réseau | Inventaire cohérent avec la page ; aucune collecte facultative activée par surprise |
| Consentement, si présent | Aucun choix → refus → acceptation → retrait → rechargement | Les flux suivent le choix, séparément sur les surfaces concernées |

La vérification HTTP complète la lecture de `dist` : un fichier correct ne prouve pas que le CDN, ses redirections et ses injections servent le même contenu. Enregistrer les en-têtes reçus, pas seulement ceux du fichier `_headers`.

### Audit des stockages et du réseau

Créer un profil navigateur de test vide. Pour chaque origine, relever l'état avant interaction, après navigation, après connexion avec un compte de test, puis après déconnexion. Si une CMP existe, ajouter refus, acceptation et retrait. Relever noms/types, finalités, durées et domaines destinataires ; masquer les valeurs de session et autres données personnelles dans les preuves. Examiner aussi les paramètres d'analytics ou de suivi activés dans les tableaux de bord des prestataires.

Cette recette s'exécute sur un environnement identifié. Ne pas effacer les données du navigateur habituel d'un utilisateur pour simuler une session vierge.

### Fiche de preuve à copier

```yaml
date: "{{DATE}}"
environment: "{{LOCAL_PREVIEW_STAGING_PRODUCTION}}"
commit: "{{SHA}}"
landing_origin: "{{URL}}"
app_origin: "{{URL}}"
legal_document_versions: "{{VERSIONS_OU_HASHES}}"
build_and_html_validation: "{{COMMANDES_ET_RESULTATS}}"
negative_publication_checks: "{{RESULTATS}}"
http_routes_and_headers: "{{RESULTATS}}"
anonymous_signup_and_return: "{{RESULTATS}}"
mobile_keyboard_installed_app: "{{RESULTATS}}"
storage_network_inventory: "{{CHEMIN_PREUVE}}"
content_review_owner_and_date: "{{RESPONSABLE_DATE}}"
open_points: []
```

## 7. Maintenance

Revoir le dispositif à chaque changement d'éditeur, contact, hébergeur, prestataire, finalité, donnée collectée, durée, public visé, mécanisme IA, paiement ou traceur. Mettre à jour contenu et date/version, reconstruire la landing puis contrôler le HTML publié et les liens de l'app. Une nouvelle URL peut imposer aussi une livraison de l'app ; conserver des redirections publiques pour les anciennes versions.

Conserver l'historique éditorial dans Git. Si des versions sont acceptées contractuellement, conserver également les textes immuables correspondant aux preuves. Une modification de politique peut nécessiter une information supplémentaire ou un nouveau consentement selon sa nature ; changer seulement la date n'en tient pas lieu.

Les sources officielles citées ont été consultées le 12 septembre 2026. Les revérifier lors d'une réutilisation ultérieure et selon le pays et l'activité du nouveau service.
