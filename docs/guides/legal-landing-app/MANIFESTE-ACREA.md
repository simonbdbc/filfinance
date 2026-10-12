# Manifeste ACREA Fitness

Renseigné le 12 septembre 2026 à partir du dépôt, commit `ff3a3b32`.

Ce manifeste décrit les données configurées et les mécanismes présents dans ACREA. Les informations non établies portent la mention **À confirmer** ; elles ne sont pas remplacées par des suppositions. La configuration distante, le contenu actuellement servi et les contrats des prestataires n'ont pas été vérifiés lors de cette rédaction.

Pour un autre projet, utiliser [MANIFESTE-MODELE.md](./MANIFESTE-MODELE.md), conservé vierge. Les références `[S1]` à `[S14]` renvoient aux sources locales listées en fin de document.

## Identité, périmètre et destinations

```yaml
reference:
  collected_at: "2026-09-12"
  repository_commit: "ff3a3b32"
  evidence_scope: "Lecture du code, des exemples de configuration et de la documentation du dépôt"
  production_verified: false

project:
  name: "ACREA Fitness"
  description: >-
    Programme personnalisé au poids du corps, avec évaluation de niveau,
    séances hebdomadaires de renforcement, cardio et proprioception,
    guidage, retours d'entraînement et conservation locale des données.
  locale: "fr-FR"
  countries:
    - "À confirmer : contenu en français et CGU soumises au droit français ; périmètre commercial géographique non défini dans les sources consultées"
  audience: "Utilisateurs d'un programme sportif au poids du corps ; âge minimum et accueil des mineurs à confirmer"
  business_model: "À confirmer : aucune offre payante définie dans les pages légales consultées ; CGV prévues par M6 avant une offre payante"
  landing_framework: "Astro, génération HTML statique, TypeScript et CSS"
  app_framework: "Expo, Expo Router et React Native, TypeScript"
  app_platforms:
    - "Web / PWA : cible de publication du pipeline"
    - "Android / iOS : scripts de lancement présents ; disponibilité dans les stores non vérifiée"

publisher:
  legal_name: "SB ALGO"
  legal_form: "À confirmer : statut juridique non renseigné dans les sources consultées"
  registered_office: "60 rue François 1er 75008 Paris"
  registration: "À confirmer : SIREN, SIRET et immatriculation applicables non renseignés"
  share_capital: "À confirmer selon la forme juridique ; aucune valeur établie"
  vat_number: "À confirmer : numéro ou régime de TVA non renseigné"
  publication_director: "sbdev"
  contact_email: "support@acrea-fitness.fr"
  contact_phone: "À confirmer : aucun téléphone de l'éditeur identifié"
  privacy_contact: "support@acra-fitness.fr"
  privacy_contact_status: >-
    Valeur présente dans la CI et l'exemple landing. Diverge du contact
    support@acrea-fitness.fr documenté pour la messagerie et configuré dans
    l'exemple PWA. Adresse à arbitrer avant reprise dans des textes approuvés.
  dpo_contact: "À confirmer : aucune désignation ni coordonnée de DPO établie"
  regulated_activity_details: "À confirmer selon l'activité et le statut ; ne pas déduire une qualification professionnelle du contenu sportif"
  information_source_and_owner: >-
    Identité, adresse, publication et contact légal issus de .gitlab-ci.yml
    et apps/landing/.env.example [S1]. Contact support issu de [S3] et [S4].
    Valeurs relevées le 12 septembre 2026 ; validation factuelle à réaliser
    par l'éditeur SB ALGO, responsable opérationnel de la revue à désigner.

hosting:
  landing:
    name: "Cloudflare, Inc."
    service: "Cloudflare Pages, projet acrea-fitness"
    address: "101 Townsend St., San Francisco, CA 94107, États-Unis"
    phone: "+1 (650) 319-8930"
    website: "https://www.cloudflare.com/"
    evidence: "Coordonnées reprises de site.config.ts [S2], projet documenté dans [S3] ; contrat non vérifié"
  app_and_backend:
    providers_and_roles: >-
      Cloudflare Pages pour la PWA, projet acrea-fitness-app ; Supabase pour
      l'authentification et les données distantes décrites dans le dépôt ;
      SQLite sur l'appareil pour la persistance locale ; Stalwart pour la
      réception du support et Brevo pour les envois externes et les emails Auth.
      API Rust : cible documentée, activation conditionnée à DNS/TLS/health,
      disponibilité distante non établie par ce manifeste.
    data_locations_and_transfers: >-
      À confirmer : régions effectives, sous-traitants, lieux des sauvegardes,
      transferts et garanties contractuelles non établis. L'adresse sociale
      de Cloudflare ne constitue pas une localisation des données de l'app.

environments:
  local:
    landing_origin: "http://localhost:4321"
    app_origin: "http://localhost:8081"
    api_origin: "http://localhost:8080"
    evidence: "Exemple PWA et script de lancement Web [S4] ; ports configurables"
  staging:
    landing_origin: "À confirmer : aucune origine de recette dédiée établie"
    app_origin: "À confirmer : aucune origine de recette actuelle établie"
    historical_app_origin: "https://acrea-fitness-app.sbcenter.fr"
    evidence: "Ancienne origine mentionnée pour une transition Auth dans [S3] ; ne vaut pas environnement de recette actuel"
  production:
    landing_origin: "https://acrea-fitness.fr"
    app_origin: "https://app.acrea-fitness.fr"
    api_origin: "https://acrea-fitness-api.sbcenter.fr"
    api_status: "Cible conditionnelle dans [S3] ; activation effective non vérifiée"

legal:
  paths:
    legal_notice: "/mentions-legales/"
    privacy: "/politique-de-confidentialite/"
    terms: "/conditions-generales-utilisation/"
    cookies: "/cookies/"
  scope: "Landing acrea-fitness.fr et application associée app.acrea-fitness.fr, selon les CGU et la politique présentes dans le dépôt"
  updated_at: "25 août 2026"
  version: "Aucune version éditoriale distincte configurée ; date commune dans legalConfig"
  terms_acceptance: "Information de lecture dans AuthGate ; aucun mécanisme d'acceptation versionnée ajouté par cette intégration"
  permanent_app_entry: "Aucun point d'entrée permanent identifié par la recherche des liens légaux ; trois liens dans AuthGate en mode authentification configurée"
  offline_access: "Documents ouverts sur la landing via Linking.openURL ; aucune copie locale versionnée des textes identifiée"
  paid_offer_documents: "CGV absentes des quatre routes ; M6 prévoit leur ajout avant toute offre payante"
  data_retention_as_configured: >-
    Les données du compte, de profil et d'entraînement sont conservées pendant
    la durée d'activité du compte. À la demande de suppression adressée à
    support@acra-fitness.fr, elles sont supprimées ou anonymisées dans les
    30 jours, sauf obligation légale ou nécessité de défense de droits.
  data_retention_status: "Texte configuré dans [S1] ; procédure d'exécution, périmètre des copies et preuve de suppression à confirmer"

tracking:
  mode: "Aucun traceur non essentiel configuré par défaut selon la position documentée ; endpoint d'analytics PWA refusé par le job CI"
  audit_date: "À confirmer : aucun audit navigateur anonyme achevé établi ; revue documentaire et du code le 12 septembre 2026"
  inventory_evidence: "Inventaire préliminaire ci-dessous ; audit des stockages et appels réseau encore ouvert dans M6 [S5]"
  consent_manager_if_needed: "Aucune CMP dans le dispositif légal documenté ; à prévoir avant activation d'un traceur soumis à consentement"
  consent_withdrawal_entry: "Aucun mécanisme de retrait identifié dans ce dispositif sans CMP"
  cross_origin_behavior: "Aucun choix de consentement partagé implémenté dans le dispositif ; landing et app ont des origines distinctes"
  analytics_endpoint: "EXPO_PUBLIC_PWA_ANALYTICS_ENDPOINT vide dans l'exemple PWA ; test -z dans build_pwa"
  runtime_override: "Le lecteur Web permet une surcharge par window.__ACREA_FITNESS_ENV__ ; configuration effective à contrôler"

publication:
  content_owner: "SB ALGO, éditeur configuré ; personne chargée de la validation à confirmer"
  content_review: "M6 IN_PROGRESS ; validation juridique des CGU et recette publique encore ouvertes dans la roadmap consultée"
  ci_provider: "GitLab CI"
  html_validation_command: "PUBLIC_LEGAL_PUBLICATION_ENFORCED=true pnpm --dir apps/landing validate"
  prerequisite: "Variables légales fournies et pnpm --dir apps/landing build exécuté avec les mêmes valeurs"
  deploy_depends_on_validation: true
  deployment_job: "release_production, branche main, après les jobs requis dont build_landing et build_pwa"
  publication_status: "Mécanisme constaté dans le YAML ; dernier pipeline et contenu distant non vérifiés"
```

Les cinq valeurs légales de `[S1]` sont reproduites telles qu'elles apparaissent dans les fichiers, y compris `sbdev` et le domaine `acra-fitness.fr`. Leur présence ne valide pas leur exactitude juridique. Les champs supplémentaires de ce manifeste, comme `version` ou `privacy_contact_status`, sont documentaires ; ils ne sont pas automatiquement consommés par le build.

## Inventaire des traitements

Inventaire préliminaire fondé sur le code et la documentation, sans observation réseau ni examen des contrats des prestataires. Les bases légales ci-dessous reprennent les déclarations de la politique actuelle lorsqu'elles existent ; elles ne constituent pas une nouvelle validation juridique.

| Traitement | Données / caractère obligatoire | Finalité | Base déclarée ou à confirmer | Destinataires / prestataires | Lieux / garanties | Conservation et suppression | Sources |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Compte / authentification | Email et mot de passe demandés par le formulaire ; session et profil de compte. Ne pas consigner les identifiants dans les preuves. | Créer le compte, ouvrir et sécuriser la session | Exécution du contrat selon la politique actuelle | Supabase Auth, app ; Brevo pour les emails Auth | Région et garanties à confirmer | Activité du compte puis demande de suppression selon le texte configuré ; exécution et sauvegardes non vérifiées | [S1], [S4], [S6], [S7] |
| Profil / évaluation / programmes / retours d'entraînement | Profil renseigné, niveaux, prescriptions, séances et retours ; détail obligatoire/facultatif à inventorier par écran | Personnaliser les séances, conserver l'historique et reprendre l'activité | Exécution du contrat annoncée pour la fourniture du service ; qualification des données et cas particuliers à confirmer | Stockage local SQLite ; données distantes selon les fonctions configurées. Ne pas supposer que tout le contenu local est synchronisé. | Appareil utilisateur ; destinations distantes et garanties à inventorier | Texte commun de [S1] ; purge des copies locales et sauvegardes à démontrer | [S6], [S8], [S9] |
| Support / messagerie | Adresse email, contenu de demande et métadonnées de courrier | Recevoir et traiter les demandes, répondre ; transmettre les emails d'authentification | À documenter par finalité ; le routage SMTP ne définit pas la base légale | Stalwart entrant ; Brevo sortant et Auth ; personnes habilitées au support à confirmer | Localisations effectives et garanties à confirmer | Durées des boîtes, journaux et sauvegardes à définir ; aucune durée propre au support établie | [S3] |
| Sécurité / diagnostics | Messages d'erreur, contexte et date ; journaux des services externes à inventorier | Diagnostiquer les dégradations, maintenir le fonctionnement | Intérêt légitime annoncé pour les données techniques de sécurité ; périmètre à vérifier | Console et mémoire du frontend pour le mécanisme examiné ; exploitants des services pour leurs journaux à vérifier | Frontend local pour les diagnostics observés ; autres destinations à établir | Liste des 50 diagnostics récents en mémoire dans le module ; conservation des consoles, logs distants et exports à confirmer | [S6], [S10] |
| Génération personnalisée / IA | Profil, évaluation, catalogue et historique décrits pour le moteur de programme | Composer une semaine et classer les exercices | À détailler dans l'information sur la personnalisation ; ne pas l'assimiler à un consentement d'entraînement de modèle | Architecture locale Rust/Burn/WASM décrite ; endpoint Rust également documenté. Flux exacts de la version publiée à vérifier. | Calcul local décrit ; ne pas inventer de fournisseur de LLM distant | Programmes persistés localement dans le repository examiné ; entraînement sur données réelles présenté comme une évolution future conditionnelle | [S9] |
| Événements PWA optionnels | Nom de l'événement, payload et horodatage ; envoi Beacon/fetch si endpoint défini | Suivre des événements d'installation et de cycle de vie | À qualifier avant activation ; aucun consentement livré par les liens légaux | Endpoint configuré, aucun destinataire fixe renseigné dans l'exemple | À établir si activation ; surcharge Web à contrôler | Pas d'envoi depuis ce module sans endpoint ; conservation du destinataire non définie | [S1], [S4], [S11] |
| Paiement / marketing | Aucun parcours de paiement ni traitement marketing établi par les sources consultées | À préciser si une offre ou une campagne est ajoutée | À déterminer si applicable | Aucun prestataire à inscrire sans preuve | Non établi | Aucune durée à inventer ; CGV prévues avant offre payante dans M6 | [S5], [S6] |

Les capacités `basic`, `premium` et `admin` mentionnées dans le dépôt ne suffisent pas à établir une offre commerciale payante. Le document IA `[S9]` contient lui-même un statut `BLOCKED` pour une bascule V4 : son architecture décrit le projet, sans prouver quelle version est actuellement active à distance.

## Inventaire des stockages et traceurs

Cet inventaire décrit des mécanismes identifiables dans les sources. Il ne remplace pas l'audit anonyme demandé dans M6. La qualification « nécessaire » de la page cookies reste à vérifier pour chaque finalité ; les durées exactes ne sont pas déduites du seul type de stockage.

| Surface | Mécanisme identifié | Type / déclenchement | Finalité | Durée constatée ou à confirmer | Destinataire | Qualification / vérification restante | Sources |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Landing | Pages HTML statiques ; aucun bandeau dans le dispositif | Chargement des pages et ressources publiques | Présentation et information | Cache navigateur/CDN selon configuration, à relever | Navigateur et hébergeur | Position sans cookie publicitaire, personnalisation ou mesure d'audience déclarée ; vérifier les injections et réponses réelles | [S2], [S6] |
| App | Session Supabase avec `persistSession: true`, `autoRefreshToken: true`, `storage: AsyncStorage` | Authentification et restauration | Maintenir la session | Cycle réel des tokens et effacement à vérifier dans le runtime cible | Appareil et Supabase Auth | Mécanisme d'authentification ; inventaire de la clé effective et des durées à réaliser sans enregistrer sa valeur | [S7] |
| App, mode local | `acrea-fitness.demo-session` | AsyncStorage, création/restauration de session locale | Mode invité / démonstration | Suppression explicite dans le flux de déconnexion local ; autres cas à tester | Appareil | Inclure le mode invité dans l'audit et l'accès aux documents | [S7] |
| App | `acrea-fitness.db` | SQLite via expo-sqlite ; runtime Web OPFS documenté | Catalogue et données locales de l'app | Politique de purge et éviction navigateur à confirmer | Appareil ; synchronisations selon fonction | Nécessité et purge à documenter ; ne pas confondre OPFS avec un cookie HTTP | [S8], [S9] |
| App | `acrea-fitness.theme` | AsyncStorage, lecture puis changement du thème | Préférence d'affichage | Pas d'expiration constatée dans le composant de thème examiné | Appareil | Classer la préférence et vérifier sa suppression effective | [S12] |
| App | Persistance de session d'affichage | AsyncStorage dans le module de persistance | Reprise d'écran et de contexte | Clés, usages effectifs par écran et nettoyage à compléter | Appareil | La présence du module ne prouve pas son activation sur tous les écrans | [S12] |
| PWA | `/service-worker.js`, caches préfixés `acrea-fitness` | Service worker et Cache Storage générés au build | Ressources locales, fonctionnement PWA et mises à jour | Règles de version et nettoyage présentes dans le générateur ; mesurer l'état réel | Appareil ; requêtes de ressources selon les règles du worker | Inventorier les caches présents dans une PWA installée | [S13] |
| PWA | `EXPO_PUBLIC_PWA_ANALYTICS_ENDPOINT` | Beacon/fetch lors d'événements si endpoint non vide | Événements optionnels de cycle de vie | Côté destinataire à définir si activation | URL de configuration | Envoi désactivé sans endpoint ; gate CI ne couvre pas à lui seul les surcharges runtime | [S11] |
| Infrastructure | Services Cloudflare et prestataires de l'app | Requêtes réseau, logs et éventuels mécanismes anti-abus | Diffusion et sécurité | À relever auprès des prestataires et sur le navigateur | Prestataires configurés | Aucun nom de cookie ou délai fournisseur inventé ; paramètres effectifs à auditer | [S3], [S5] |

Aucune valeur de cookie, de session, aucun mot de passe ni clé de service n'est inclus dans ce manifeste.

## Contenus déjà en place

| Élément | Données ACREA présentes | Compléments ou vérifications |
| --- | --- | --- |
| Mentions légales | Éditeur, domiciliation, publication, Cloudflare, contact et propriété intellectuelle | Statut/immatriculation et autres informations applicables ; contact à arbitrer |
| Confidentialité | Compte, profil, évaluation, programmes, séances, retours ; finalités, bases annoncées, conservation, droits et paragraphe santé | Destinataires et régions effectives, cas particuliers, durées par traitement, preuve de suppression |
| CGU | Site et app, compte/sécurité, usage sportif, propriété, évolution/responsabilité et droit français | Public/âge, revue juridique et évolution de l'offre ; aucune preuve d'acceptation versionnée fournie par cette intégration |
| Cookies | Absence déclarée de traceurs non essentiels par défaut, stockages techniques, engagement avant activation | Audit anonyme des deux origines ; règles effectives et paramètres de l'hébergeur |
| Formulaire | Mention de lecture, liens CGU/confidentialité/cookies dans AuthGate lorsque l'authentification est configurée | Pas de lien direct aux mentions dans ce helper ; absence du bloc en mode session locale ; accès permanent à définir |

La date commune des documents est **25 août 2026**, issue du code. La date **12 septembre 2026** correspond à la préparation de ce manifeste, sans mise à jour des pages publiques.

## Décisions et données encore manquantes

Le tableau indique les points à résoudre pour finaliser le dossier. Il ne déclenche pas de modification du code, de la CI ou des services, et ne présente pas ces points comme des contrôles déjà implémentés.

| Point | Responsable à mobiliser | Source / preuve attendue | Impact | État au 12 septembre 2026 |
| --- | --- | --- | --- | --- |
| Statut juridique, immatriculation, TVA, capital si applicable, identité de publication | Éditeur SB ALGO | Justificatifs et validation des mentions | Empêche de déclarer les informations d'éditeur complètes | À confirmer ; `SB ALGO`, adresse et `sbdev` repris de la configuration |
| `support@acra-fitness.fr` contre `support@acrea-fitness.fr` | Éditeur / responsable messagerie | Adresse retenue et test de réception | Contact d'exercice des droits à fiabiliser avant publication de textes approuvés | Divergence conservée explicitement ; aucun email envoyé lors de cette rédaction |
| Pays, public, âge minimum et modèle économique | Responsable produit | Décision de périmètre et offre | Conditionne les contenus et documents supplémentaires | À confirmer |
| Régions, transferts, sous-traitants et garanties | Responsable infrastructure / éditeur | Configuration et contrats effectifs | Politique de confidentialité à compléter | Non vérifiés |
| Conservation et suppression | Responsable données / support | Procédure couvrant bases, copies locales, sauvegardes, logs et preuve d'exécution | La promesse configurée sous 30 jours reste à démontrer | Texte repris de [S1], exécution non validée |
| Inventaire navigateur / réseau | Responsable technique | Audit anonyme landing + PWA et configuration prestataires | M6 conserve cette recette ouverte | Inventaire source préliminaire uniquement |
| Parcours public, clavier, mobile et accès permanent app | Responsable frontend / QA | Recette landing → inscription, ouverture/retour et états connecté/invité | M6 conserve le parcours public ouvert ; accès permanent non identifié | Non testé dans ce travail documentaire |
| Validation des CGU et cas particuliers | Éditeur / conseil juridique | Revue datée ; CGV avant offre payante selon M6 | Empêche de présenter M6 comme entièrement terminé | Ouvert dans la roadmap |
| Valeurs effectives de CI et contenu publié | Responsable publication | Pipeline, commit d'artefact et HTML servi | Distinguer la configuration du dépôt de la production réelle | Aucun pipeline distant consulté ni déploiement exécuté |

## Sources locales

Les liens ci-dessous pointent vers les fichiers examinés dans ce dépôt. Après copie de ce manifeste seul vers un autre dépôt, ils servent de références de provenance et doivent être adaptés ou archivés.

- **[S1]** [Pipeline GitLab](../../../.gitlab-ci.yml) et [exemple des variables légales landing](../../../apps/landing/.env.example).
- **[S2]** [Configuration de la landing](../../../apps/landing/src/config/site.config.ts), [layout légal](../../../apps/landing/src/layouts/LegalLayout.astro), [package landing](../../../apps/landing/package.json) et [package PWA](../../../apps/pwa/package.json).
- **[S3]** [URLs et cibles de production](../../architecture/PRODUCTION-URLS.md) et [messagerie de production](../../architecture/MAIL-PRODUCTION.md). La messagerie contient des preuves datées du 22 août 2026, non rejouées ici.
- **[S4]** [Configuration d'exemple PWA](../../../apps/pwa/.env.example) et [lancement du Web local](../../../apps/pwa/scripts/start-web.mjs). Aucune clé de cette configuration n'est recopiée dans le manifeste.
- **[S5]** [Roadmap, jalon M6](../../ROADMAP.md).
- **[S6]** Pages [mentions légales](../../../apps/landing/src/pages/mentions-legales.astro), [confidentialité](../../../apps/landing/src/pages/politique-de-confidentialite.astro), [CGU](../../../apps/landing/src/pages/conditions-generales-utilisation.astro) et [cookies](../../../apps/landing/src/pages/cookies.astro).
- **[S7]** [Client d'authentification](../../../apps/pwa/src/shared/config/supabaseClient.ts), [AuthProvider](../../../apps/pwa/src/features/auth/providers/AuthProvider.tsx), [AuthGate](../../../apps/pwa/src/features/auth/components/AuthGate.tsx) et [helper légal](../../../apps/pwa/src/shared/legal/legalLinks.ts).
- **[S8]** [Contrat de stockage et synchronisation](../../architecture/pwa/OFFLINE-SYNC.md) et [ouverture SQLite](../../../apps/pwa/src/shared/db/database.ts).
- **[S9]** [Programme hebdomadaire et évolution IA](../../app/epics/EPIC-028-ia-programme-hebdomadaire.md) et [repository des programmes](../../../apps/pwa/src/features/programming/services/weeklyProgramRepository.ts).
- **[S10]** [Diagnostics techniques](../../../apps/pwa/src/shared/observability/technicalDiagnostics.ts).
- **[S11]** [Événements PWA](../../../apps/pwa/src/features/pwa/services/pwaAnalytics.ts) et [configuration publique avec surcharge Web](../../../apps/pwa/src/shared/config/runtimeEnv.ts).
- **[S12]** [Préférence de thème](../../../apps/pwa/src/shared/theme/theme.tsx) et [persistance d'affichage](../../../apps/pwa/src/shared/persistence/displaySessionPersistence.ts).
- **[S13]** [Client service worker](../../../apps/pwa/src/features/pwa/services/serviceWorkerClient.ts) et [générateur de l'artefact PWA](../../../apps/pwa/scripts/prepare-cloudflare-pages.mjs).
- **[S14]** [Validateur HTML landing](../../../apps/landing/scripts/validate-production-build.mjs).

Le manifeste est documentaire : aucun script ACREA ne lit automatiquement ce YAML. Le remplissage n'a changé ni les pages publiques, ni les variables CI, ni le fonctionnement de l'app.
