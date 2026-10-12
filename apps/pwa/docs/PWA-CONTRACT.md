# Contrat PWA - Installation, versions et mise à jour

## 1. Objet du document

Ce document définit les règles de conception, de configuration, de mise en œuvre et de validation d'une Progressive Web App (PWA). Il sert de contrat technique et produit pour tout projet web qui doit être installable, maintenable, utilisable en conditions réseau dégradées et capable de gérer ses versions proprement.

Le document est volontairement neutre. Les noms, domaines, chemins, services et marques doivent être remplacés par des placeholders propres au projet cible.

Placeholders recommandés :

- `[APP_NAME]` : nom public de l'application.
- `[APP_SHORT_NAME]` : nom court affiché sous l'icône.
- `[APP_ORIGIN]` : origine HTTPS de production.
- `[APP_VERSION]` : version applicative visible par l'utilisateur ou le support.
- `[BUILD_ID]` : identifiant unique du build déployé.
- `[CACHE_PREFIX]` : préfixe de cache propre au projet.
- `[SUPPORT_CONTACT]` : canal de support utilisateur.

## 2. Principes contractuels

Une PWA est consideree conforme uniquement si elle respecte les points suivants :

- Elle est servie en HTTPS en production.
- Elle fournit un manifest web valide et reference par toutes les pages d'entree.
- Elle dispose d'icones adaptees aux surfaces d'installation principales.
- Elle gere explicitement son mode installe, son mode navigateur et ses cas non supportes.
- Elle utilise un service worker uniquement si sa strategie de cache, de mise a jour et de rollback est documentee.
- Elle informe l'utilisateur lorsque l'application est offline, en cours de mise a jour ou bloquee par une version obsolete.
- Elle expose une information de version exploitable par l'utilisateur, le support et les equipes techniques.
- Elle permet de diagnostiquer l'installation, les caches et l'etat du service worker.
- Elle ne stocke aucune donnee sensible dans un cache public ou non chiffre.
- Elle conserve un comportement web normal lorsque l'installation PWA n'est pas supportee.

## 3. État de conformité

Chaque projet doit suivre l'etat de conformite PWA avec les statuts ci-dessous.

| Statut | Definition | Critere de sortie |
|---|---|---|
| Non demarre | Aucun element PWA n'est configure. | Un responsable et un perimetre PWA sont identifies. |
| A configurer | Les choix fonctionnels sont connus mais la configuration n'est pas complete. | Manifest, icones, service worker et regles d'installation sont implementes ou explicitement exclus. |
| Implemente | Les elements techniques existent dans le code. | Les validations manuelles et automatisees sont executees. |
| Valide | Les tests d'installation, update, offline et rollback sont passes. | La version peut etre deployee ou reutilisee comme reference. |
| Bloque | Un probleme navigateur, infra, produit ou securite empeche la conformite. | Une decision documentee leve le blocage ou modifie le perimetre. |

Un projet ne peut pas etre marque `Valide` si l'un des sujets suivants est incomplet : manifest, installation, service worker ou strategie sans service worker, update, rollback, desinstallation, diagnostics, QA multi-navigateurs.

## 4. Configuration minimale PWA

### 4.1 Manifest web

Le fichier `manifest.webmanifest` ou `manifest.json` doit etre servi avec un type MIME compatible JSON et reference dans le HTML :

```html
<link rel="manifest" href="/manifest.webmanifest" />
```

Le manifest doit contenir au minimum :

- `id` : identifiant stable de l'application.
- `name` : nom complet de l'application.
- `short_name` : nom court affiche sur l'ecran d'accueil.
- `description` : description courte et non marketing.
- `start_url` : URL d'ouverture de l'application installee.
- `scope` : perimetre de navigation controle par la PWA.
- `display` : `standalone` par defaut, sauf decision contraire documentee.
- `background_color` : couleur de lancement.
- `theme_color` : couleur systeme principale.
- `icons` : icones 192x192 et 512x512 au minimum.
- `prefer_related_applications` : absent ou `false`, sauf strategie store documentee.

Les icones doivent inclure :

- Une icone 192x192.
- Une icone 512x512.
- Une variante `maskable` pour les plateformes qui rognent l'icone.
- Une icone Apple touch lorsque la cible iOS/iPadOS est supportee.
- Des fichiers optimises, servis avec un cache long si leur nom est versionne.

Les champs optionnels suivants sont recommandes lorsque le produit le justifie :

- `screenshots` pour enrichir le prompt d'installation sur les plateformes compatibles.
- `shortcuts` pour exposer des actions frequentes.
- `categories` pour aider la classification.
- `orientation` uniquement si l'application ne supporte pas correctement toutes les orientations.

### 4.2 HTML et metadata

Chaque page d'entree doit declarer les elements suivants :

- lien vers le manifest ;
- `meta name="theme-color"` ;
- viewport responsive ;
- titre clair et stable ;
- icones principales ;
- meta Apple si l'installation iOS/iPadOS est visee ;
- canonical ou strategie SEO si l'application expose des URLs publiques.

Les pages ne doivent pas dependre de fichiers charges en HTTP non securise. Les contenus mixtes actifs sont interdits.

### 4.3 HTTPS et origine

En production, la PWA doit etre disponible sur une origine HTTPS stable. Les environnements locaux peuvent utiliser `localhost` ou `127.0.0.1`.

Le contrat d'origine doit preciser :

- domaine de production ;
- domaines de preview ;
- politique de redirection HTTP vers HTTPS ;
- gestion des sous-domaines ;
- politique CORS pour les ressources controlees par le service worker ;
- comportement attendu si l'utilisateur arrive sur une ancienne origine.

### 4.4 Routing et fallback

Pour une application SPA, les routes profondes doivent fonctionner au rechargement direct. Le serveur ou l'hebergeur doit renvoyer le shell applicatif pour les routes front-end et laisser passer les assets statiques.

Regles minimales :

- Les routes applicatives retournent le shell HTML.
- Les assets statiques inexistants retournent une erreur explicite, pas le shell HTML.
- La page offline est servie depuis le cache lorsque le reseau est indisponible.
- Les redirections permanentes sont documentees avec une procedure de rollback.

## 5. Service worker et cache

### 5.1 Decision obligatoire

Chaque projet doit choisir l'une des deux strategies suivantes :

- `Sans service worker` : la PWA peut etre installable selon les plateformes, mais aucune promesse offline n'est faite.
- `Avec service worker` : le projet prend en charge le cycle install/activate/fetch, le cache, l'invalidation, les mises a jour et les fallbacks.

Le choix doit etre documente dans le projet. Une PWA avec service worker non maitrise est consideree plus risquee qu'une PWA sans service worker.

### 5.2 Cycle de vie

Le service worker doit gerer explicitement :

- `install` : preparation du cache minimal.
- `activate` : nettoyage des anciennes versions de cache.
- `fetch` : strategie de reponse pour documents, scripts, styles, images, polices, API et assets externes.
- `message` : communication avec l'application pour l'activation d'une mise a jour lorsque necessaire.

L'utilisation de `skipWaiting()` et `clients.claim()` doit etre volontaire. Elle est interdite par defaut si elle peut remplacer une version active sans consentement utilisateur ou sans sauvegarde d'etat.

### 5.3 Strategies de cache

Les strategies doivent etre choisies par type de ressource :

| Ressource | Strategie recommandee | Regle |
|---|---|---|
| Shell HTML | Network first avec fallback cache | Eviter de bloquer les updates. |
| JS/CSS versionnes | Cache first | Nom de fichier hashe obligatoire. |
| Images statiques versionnees | Cache first | Cache long possible. |
| Images utilisateur | Network first ou stale while revalidate | Ne pas afficher de contenu obsolete critique. |
| API lecture | Network first avec fallback controle | Le fallback doit etre visible. |
| API ecriture | Pas de cache silencieux | Utiliser une file d'attente explicite si offline. |
| Page offline | Precache | Disponible sans reseau. |

Les caches doivent etre prefixes par `[CACHE_PREFIX]` et inclure une version. Exemple :

```text
[CACHE_PREFIX]-shell-v[APP_VERSION]
[CACHE_PREFIX]-assets-v[ASSET_VERSION]
[CACHE_PREFIX]-runtime-v[BUILD_ID]
```

### 5.4 Invalidation

Une nouvelle version doit pouvoir :

- supprimer les caches obsoletes ;
- conserver les donnees utilisateur autorisees ;
- migrer les donnees locales si le schema change ;
- refuser l'activation si une migration critique echoue ;
- afficher un etat d'erreur recuperable si l'application ne peut pas etre mise a jour.

Les noms de cache non versionnes sont interdits.

## 6. Installation de l'application

### 6.1 Detection de l'installabilite

L'application doit distinguer les etats suivants :

- Installation supportee et disponible.
- Application deja installee.
- Installation non supportee par le navigateur.
- Installation possible uniquement via le menu du navigateur.
- Installation bloquee par une configuration incomplete.
- Installation masquee apres refus utilisateur.

Le front-end ne doit jamais promettre une installation automatique. Le navigateur ou l'OS garde le controle final.

### 6.2 Prompt et bouton d'installation

Un composant `InstallAppBanner` ou equivalent doit respecter ces regles :

- ne pas s'afficher avant que l'application soit utilisable ;
- ne pas masquer une action critique ;
- proposer une action principale claire ;
- permettre le rejet sans penaliser l'utilisateur ;
- memoriser le rejet selon une duree documentee ;
- ne pas s'afficher si l'application est deja installee ;
- fournir une alternative textuelle lorsque le prompt natif n'est pas disponible.

Un composant `InstallAppButton` peut etre affiche dans un menu ou un ecran de parametres. Il doit indiquer l'etat reel : disponible, deja installe, non supporte, instructions manuelles.

Un bouton permanent `MenuInstallButton` ou equivalent doit etre disponible depuis le menu principal ou le menu utilisateur. Il doit rester visible comme point d'entree PWA, mais son libelle et son action doivent changer selon l'etat :

- `Installer l'app` lorsque l'installation native est disponible ;
- `Comment installer l'app` lorsque seules des instructions manuelles sont possibles ;
- `App installee` lorsque le mode installe est detecte ;
- `Mise a jour disponible` si une nouvelle version doit etre appliquee ;
- `Gerer l'app installee` lorsque l'utilisateur doit acceder aux informations, au stockage ou a l'aide de desinstallation.

### 6.3 Ecran d'installation

Un ecran dedie `InstallAppScreen` doit exister lorsque l'installation est une fonctionnalite attendue du produit. Il peut etre accessible depuis le menu, une page aide, un bouton d'installation ou un prompt.

L'ecran doit contenir :

- l'etat d'installation courant ;
- l'action d'installation native lorsque disponible ;
- les instructions manuelles par plateforme lorsque le prompt natif n'est pas disponible ;
- les prerequis d'installation manquants si l'application n'est pas installable ;
- une explication courte de ce que change le mode installe ;
- un lien vers les informations de version et de diagnostic PWA ;
- une action de retour claire vers l'application.

L'ecran ne doit pas promettre que l'application sera installee automatiquement. Il doit expliquer que la confirmation finale depend du navigateur ou de l'OS.

### 6.4 Comportement par plateforme

Le projet doit documenter le comportement attendu au minimum pour :

- Desktop Chromium.
- Android Chromium.
- Android navigateurs alternatifs si supportes.
- iOS/iPadOS Safari.
- iOS/iPadOS navigateurs tiers si supportes.
- Desktop Safari si cible.
- Navigateurs non compatibles.

Les instructions iOS/iPadOS doivent etre separees du prompt `beforeinstallprompt`, car ce prompt n'est pas disponible partout.

### 6.5 Mode installe

Le front-end doit detecter le mode d'affichage lorsque possible :

- navigateur classique ;
- standalone ;
- fullscreen ;
- minimal-ui ;
- contexte inconnu.

Le mode installe ne doit pas changer les fonctionnalites metier sans justification. Il peut adapter la navigation, les marges systeme, les raccourcis et les messages d'installation.

## 7. Désinstallation et remise à zéro

### 7.1 Desinstallation par l'utilisateur

La desinstallation est geree par le navigateur ou l'OS. Le projet doit fournir une aide utilisateur neutre :

- comment retirer l'application installee ;
- quelles donnees peuvent rester dans le navigateur ;
- comment effacer les donnees locales si necessaire ;
- comment reinstaller proprement.

L'application ne doit pas presenter la desinstallation comme une action interne si elle ne peut pas l'executer elle-meme.

### 7.2 Ecran de desinstallation

Un ecran dedie `UninstallAppScreen` doit exister si l'application propose une aide PWA ou un menu de gestion de l'app installee.

L'ecran doit contenir :

- l'etat courant : navigateur, installe, installation non detectee ou contexte inconnu ;
- des instructions de retrait de l'application par plateforme cible ;
- une explication claire des donnees qui peuvent rester apres suppression de l'icone ;
- un acces aux actions de nettoyage applicatif autorisees ;
- une action `Vider les caches non critiques` si cette operation est supportee ;
- une action `Reinitialiser les donnees locales` si cette operation est supportee ;
- une confirmation avant toute suppression de donnees ;
- une explication de la reinstallation propre apres nettoyage.

L'ecran doit eviter tout libelle trompeur comme `Desinstaller maintenant` si l'application ne peut pas declencher elle-meme la desinstallation au niveau OS ou navigateur.

### 7.3 Nettoyage applicatif

Un composant `StorageUsagePanel` ou une action de parametres doit permettre, selon le contexte produit :

- d'afficher l'espace local utilise ;
- de vider les caches applicatifs non critiques ;
- de supprimer les donnees offline ;
- de reinitialiser les preferences locales ;
- de conserver les donnees synchronisees cote serveur ;
- d'expliquer les consequences avant confirmation.

Les actions destructives doivent demander confirmation.

### 7.4 Reinstallation

Une reinstallation doit fonctionner apres :

- suppression de l'icone installee ;
- vidage des caches ;
- expiration de session ;
- changement de version majeure ;
- changement d'origine documente.

Le projet doit tester que la reinstallation ne laisse pas l'utilisateur dans un etat bloque par un ancien service worker.

## 8. Versioning

### 8.1 Identifiants obligatoires

Chaque build de production doit exposer :

- `[APP_VERSION]` : version fonctionnelle lisible.
- `[BUILD_ID]` : identifiant unique et non ambigu du build.
- `[COMMIT_SHA]` ou equivalent : reference technique de compilation.
- `[RELEASE_CHANNEL]` : production, preview, staging ou autre canal.
- Date de build ou date de release.

Ces informations doivent etre visibles dans un ecran `AppVersionInfo` et accessibles aux logs front-end.

### 8.2 Regles SemVer

La version applicative doit suivre une logique stable :

- `MAJOR` : changement incompatible, migration lourde, rupture de donnees locales ou comportement utilisateur majeur.
- `MINOR` : fonctionnalite compatible.
- `PATCH` : correction compatible.

Une version majeure doit inclure :

- notes de migration ;
- strategie de compatibilite cache et donnees locales ;
- plan de rollback ;
- communication utilisateur si l'impact est visible.

### 8.3 Version des caches

Les caches doivent etre versionnes separement de la version produit lorsque necessaire :

- version shell ;
- version assets ;
- version runtime ;
- version schema local ;
- version metadata distante.

Une modification de structure de cache doit incrementer la version concernee.

## 9. Mise à jour de l'application

### 9.1 Detection d'une nouvelle version

L'application doit pouvoir detecter une version disponible via au moins un mecanisme :

- cycle de mise a jour du service worker ;
- endpoint de version ;
- manifest de release ;
- comparaison `[BUILD_ID]` courant / distant ;
- evenement de deploiement pousse par l'infrastructure si disponible.

La detection ne doit pas provoquer de boucle de reload.

### 9.2 Types de mise a jour

| Type | Usage | Comportement utilisateur |
|---|---|---|
| Silencieuse | Patch sans impact visible | Activation au prochain chargement ou lorsque l'application est inactive. |
| Optionnelle | Amelioration ou correction non critique | Prompt non bloquant avec action "Mettre a jour". |
| Recommandee | Correction importante | Prompt visible, report possible selon politique produit. |
| Forcee | Incompatibilite, securite, schema obsolete | Blocage controle avec action de rechargement ou mise a jour. |

Le mode `Forcee` doit rester exceptionnel et justifie par un risque concret.

### 9.3 Prompt de mise a jour

Un composant `UpdateAvailablePrompt` doit :

- indiquer qu'une nouvelle version est disponible ;
- expliquer l'impact en une phrase courte ;
- proposer `Mettre a jour` ;
- proposer `Plus tard` uniquement si l'update n'est pas forcee ;
- sauvegarder ou prevenir la perte d'etat local avant reload ;
- gerer l'echec de mise a jour avec un message recuperable.

### 9.4 Rollback

Le projet doit permettre un rollback de release sans casser les clients deja ouverts.

Regles minimales :

- conserver au moins une version stable de reference ;
- eviter les migrations locales irreversibles sans garde-fou ;
- versionner les endpoints incompatibles ;
- refuser de servir un shell obsolete avec des assets supprimes ;
- documenter la procedure de retour arriere.

## 10. Offline, reseau degrade et synchronisation

### 10.1 Etats reseau

Le front-end doit distinguer :

- reseau Supabase connecte ;
- reseau Supabase hors ligne ;
- API backend disponible ;
- API backend indisponible ;
- session expiree ;
- donnees locales disponibles ;
- donnees locales absentes.

Le composant `ConnectionStatus` doit informer sans interrompre inutilement. Il
affiche deux badges separes : le badge reseau Supabase (`Connecté` ou
`Hors ligne`) et le badge backend (`API`, en vert ou rouge selon l'etat).

### 10.2 Ecran offline

Un `OfflineScreen` doit etre disponible lorsque l'utilisateur atteint une zone non chargeable. Il doit contenir :

- etat reseau clair ;
- actions possibles ;
- bouton de nouvelle tentative ;
- indication des donnees encore accessibles ;
- lien vers l'accueil si disponible offline ;
- aucun message technique brut.

### 10.3 Donnees locales

Les donnees locales doivent etre classees :

- cache statique ;
- cache runtime ;
- preferences utilisateur ;
- donnees metier offline ;
- file d'attente de synchronisation ;
- logs techniques ;
- donnees sensibles.

Chaque classe doit avoir une regle de conservation, de suppression et de migration.

### 10.4 Ecritures offline

Si le projet accepte des ecritures offline, il doit definir :

- file d'attente persistante ;
- identifiants temporaires ;
- statut visible de synchronisation ;
- gestion des conflits ;
- retry avec backoff ;
- limite de tentatives ;
- annulation utilisateur si possible ;
- message d'erreur final.

Les ecritures offline silencieuses sont interdites.

## 11. Écrans et composants front-end attendus

### 11.1 `InstallAppBanner`

Role : proposer l'installation lorsque le contexte le permet.

Etats obligatoires :

- pret a installer ;
- deja installe ;
- non supporte ;
- prompt non disponible ;
- refuse temporairement ;
- erreur d'installation.

Regles :

- visible uniquement apres detection fiable ;
- dismissible ;
- accessible au clavier ;
- texte court ;
- pas de repetition agressive.

### 11.2 `InstallAppButton`

Role : action permanente disponible dans les parametres, le menu utilisateur ou une page aide.

Etats obligatoires :

- installer ;
- ouvrir les instructions ;
- deja installe ;
- indisponible ;
- diagnostic necessaire.

### 11.3 `MenuInstallButton`

Role : fournir un point d'entree PWA stable dans le menu principal, le menu utilisateur ou le menu lateral de l'application.

Etats obligatoires :

- installation disponible ;
- instructions manuelles disponibles ;
- app deja installee ;
- mise a jour disponible ;
- gestion de l'app installee ;
- installation non supportee.

Regles :

- rester accessible sans dependre d'une banniere temporaire ;
- utiliser un libelle adapte a l'etat courant ;
- ouvrir `InstallAppScreen` si l'installation ou les instructions sont pertinentes ;
- ouvrir `UpdateAvailablePrompt` ou l'ecran de version si une mise a jour est prioritaire ;
- ouvrir `UninstallAppScreen` ou les parametres PWA si l'app est deja installee ;
- ne pas afficher une action native impossible comme si elle etait disponible.

### 11.4 `InstallAppScreen`

Role : centraliser l'installation, les instructions par plateforme et les diagnostics d'installabilite.

Etats obligatoires :

- pret a installer ;
- instructions manuelles ;
- deja installe ;
- installation indisponible ;
- prerequis manquants ;
- erreur ou diagnostic necessaire.

Contenu attendu :

- statut d'installation courant ;
- bouton d'installation natif lorsque disponible ;
- instructions desktop, Android et iOS/iPadOS selon le support cible ;
- explication courte du mode installe ;
- lien vers les diagnostics PWA ;
- lien vers les informations de version.

### 11.5 `UninstallAppScreen`

Role : guider l'utilisateur pour retirer l'application installee et gerer les donnees locales restantes.

Etats obligatoires :

- app installee detectee ;
- app non installee ;
- contexte d'installation inconnu ;
- nettoyage disponible ;
- nettoyage en cours ;
- nettoyage termine ;
- erreur de nettoyage.

Contenu attendu :

- instructions de desinstallation par plateforme cible ;
- rappel que la suppression de l'icone ne supprime pas toujours les donnees locales ;
- acces aux actions de nettoyage autorisees ;
- confirmation avant suppression de donnees ;
- lien vers la reinstallation propre ;
- contact support ou diagnostic si la desinstallation laisse un etat bloque.

### 11.6 `UpdateAvailablePrompt`

Role : gerer l'activation d'une nouvelle version.

Etats obligatoires :

- update disponible ;
- update en cours ;
- update appliquee ;
- reload necessaire ;
- echec ;
- update forcee.

### 11.7 `OfflineScreen`

Role : fournir une experience controlee sans reseau.

Etats obligatoires :

- aucune donnee locale ;
- donnees locales disponibles ;
- action impossible offline ;
- synchronisation en attente ;
- nouvelle tentative en cours.

### 11.8 `ConnectionStatus`

Role : afficher des statuts discrets et separes pour Supabase et pour le backend.

Etats obligatoires :

- Supabase connecte : badge `Connecté`, variante succes ;
- Supabase hors ligne : badge `Hors ligne`, variante danger ;
- backend disponible : badge `API`, variante succes ;
- backend indisponible : badge `API`, variante danger.

### 11.9 `StorageUsagePanel`

Role : diagnostiquer et nettoyer les donnees locales.

Fonctions attendues :

- afficher l'espace estime ;
- afficher les categories de stockage ;
- vider les caches non critiques ;
- reinitialiser les donnees offline si autorise ;
- afficher la date de derniere synchronisation ;
- avertir avant toute suppression definitive.

### 11.10 `AppVersionInfo`

Role : exposer les informations de version.

Champs attendus :

- nom application ;
- version ;
- build ;
- canal ;
- date de release ;
- origine ;
- statut service worker ;
- statut cache ;
- lien ou texte de support.

### 11.11 `PwaDiagnosticsScreen`

Role : aider le support et la QA a comprendre l'etat local.

Diagnostics recommandes :

- manifest detecte ;
- mode d'affichage ;
- service worker supporte ;
- service worker actif ;
- caches connus ;
- espace local estime ;
- version courante ;
- version distante si disponible ;
- dernier echec de mise a jour ;
- statut offline/sync.

## 12. Regles UX et accessibilite

Les composants PWA doivent respecter les regles suivantes :

- Ne jamais bloquer l'utilisateur pour une installation facultative.
- Ne jamais afficher un prompt d'installation avant que l'utilisateur comprenne la valeur de l'application.
- Ne pas multiplier les bannieres concurrentes.
- Ne pas afficher un message "offline" si seul un endpoint secondaire echoue.
- Les actions destructives doivent etre confirmees.
- Les messages doivent etre courts, non techniques et traduisibles.
- Les boutons doivent etre utilisables au clavier.
- Les prompts doivent etre annonces correctement aux lecteurs d'ecran.
- Les couleurs d'etat ne doivent pas etre le seul moyen de comprehension.
- Les composants doivent fonctionner sur mobile et desktop.

## 13. Securite et confidentialite

Regles obligatoires :

- HTTPS obligatoire en production.
- Aucune donnee sensible dans Cache Storage.
- Aucune donnee sensible dans localStorage sans justification et revue securite.
- Les tokens d'authentification ne doivent pas etre precaches.
- Les endpoints authentifies doivent refuser les reponses servies depuis un cache obsolete si cela expose des donnees utilisateur.
- Les permissions navigateur doivent etre demandees au moment utile, avec contexte.
- Les notifications push doivent etre opt-in.
- Les logs front-end ne doivent pas exposer de secrets.
- La politique CSP doit etre compatible avec le service worker et documentee.
- Les origines tierces controlees par le service worker doivent etre limitees et justifiees.

## 14. Observabilite et support

Le projet doit pouvoir diagnostiquer :

- installation reussie ou refusee ;
- mode installe ou navigateur ;
- activation du service worker ;
- echec d'enregistrement du service worker ;
- echec de fetch critique ;
- changement de version ;
- update acceptee, reportee ou echouee ;
- cache nettoye ;
- stockage local sature ;
- erreur de synchronisation offline.

Les evenements analytics doivent etre anonymises ou pseudonymises selon la politique de confidentialite du projet.

## 15. Tests et validation

### 15.1 Validation automatisee

Les tests automatises doivent couvrir, lorsque l'outillage le permet :

- presence du manifest ;
- validite JSON du manifest ;
- presence des icones critiques ;
- reference du manifest dans le HTML ;
- presence du service worker si attendu ;
- build production ;
- absence de contenu mixte ;
- routes profondes ;
- page offline ;
- endpoint de version ;
- rendu des composants PWA principaux.

### 15.2 Validation manuelle

Avant release, la QA doit verifier :

- installation desktop ;
- installation Android ;
- installation ou instructions iOS/iPadOS ;
- bouton PWA visible et actionnable depuis le menu ;
- ecran d'installation accessible depuis le menu ;
- ecran de desinstallation ou de gestion de l'app accessible depuis le menu ;
- ouverture depuis l'icone installee ;
- mode standalone ;
- refus du prompt d'installation ;
- reaffichage controle du prompt ;
- mise a jour depuis une version N vers N+1 ;
- update forcee si applicable ;
- rollback ;
- navigation offline ;
- action impossible offline ;
- suppression des caches ;
- desinstallation puis reinstallation ;
- affichage de la version dans l'application.

### 15.3 Navigateurs cibles

Le projet doit lister ses navigateurs cibles avec un niveau de support :

| Navigateur / plateforme | Niveau | Notes |
|---|---|---|
| Desktop Chromium | Support complet | Installation, update, offline. |
| Android Chromium | Support complet | Installation, update, offline. |
| iOS/iPadOS Safari | Support adapte | Instructions specifiques et tests manuels. |
| Desktop Safari | Support adapte | Selon version cible. |
| Firefox | Support web | Installation selon plateforme, comportement degrade accepte. |
| Autres navigateurs | Support degrade | Application web utilisable sans garantie d'installation. |

Un navigateur non compatible PWA doit rester capable d'utiliser l'application comme site web lorsque les fonctionnalites metier le permettent.

## 16. Definition of Done PWA

La PWA est consideree terminee lorsque :

- le manifest est valide et complet ;
- les icones critiques sont presentes ;
- l'application est servie en HTTPS ;
- l'installation est testee sur les plateformes cibles ;
- les cas non supportes ont une experience claire ;
- le service worker est implemente ou explicitement exclu ;
- les caches sont versionnes ;
- la page offline est disponible si une promesse offline existe ;
- la strategie de mise a jour est implementee ;
- la strategie de rollback est documentee ;
- l'utilisateur peut voir la version de l'application ;
- les composants front-end PWA existent et couvrent leurs etats ;
- le bouton PWA du menu ouvre l'action adaptee a l'etat courant ;
- l'ecran d'installation couvre l'installation native, les instructions manuelles et les cas non supportes ;
- l'ecran de desinstallation explique le retrait de l'app et la gestion des donnees locales ;
- la desinstallation et la remise a zero sont documentees ;
- les tests QA sont executes ;
- les risques de securite et confidentialite sont revus.

## 17. Checklist projet

| Sujet | Statut | Responsable | Preuve attendue |
|---|---|---|---|
| Manifest reference | À renseigner | À renseigner | URL manifest + validation. |
| Icônes critiques | À renseigner | À renseigner | Liste fichiers + rendu plateforme. |
| HTTPS production | À renseigner | À renseigner | URL + certificat valide. |
| Routing profond | À renseigner | À renseigner | Tests routes directes. |
| Installation desktop | À renseigner | À renseigner | Capture ou PV QA. |
| Installation mobile | À renseigner | À renseigner | Capture ou PV QA. |
| Bouton PWA dans le menu | À renseigner | À renseigner | Capture + test des etats. |
| Ecran d'installation | À renseigner | À renseigner | Parcours installation + instructions. |
| Ecran de desinstallation | À renseigner | À renseigner | Parcours retrait + nettoyage local. |
| Instructions iOS/iPadOS | À renseigner | À renseigner | Ecran ou documentation utilisateur. |
| Service worker | À renseigner | À renseigner | Strategie + tests. |
| Cache versionne | À renseigner | À renseigner | Noms de cache + nettoyage. |
| Page offline | À renseigner | À renseigner | Test reseau coupe. |
| Update N vers N+1 | À renseigner | À renseigner | Scenario QA. |
| Rollback | À renseigner | À renseigner | Procedure testee. |
| Version visible | À renseigner | À renseigner | Ecran version. |
| Nettoyage stockage | À renseigner | À renseigner | Action utilisateur ou support. |
| Securite cache | À renseigner | À renseigner | Revue securite. |
| Observabilite | À renseigner | À renseigner | Evenements ou logs. |

## 18. Regles de maintenance

Ce document doit etre relu :

- avant le lancement d'un nouveau projet PWA ;
- avant chaque version majeure ;
- apres un changement de navigateur cible ;
- apres un changement d'hebergeur ou de domaine ;
- apres l'ajout d'une fonctionnalite offline ;
- apres un incident de cache, update ou installation.

Les projets qui reutilisent ce contrat peuvent l'etendre, mais ne doivent pas supprimer une exigence sans documenter la raison, le risque accepte et la decision de validation.

## 19. Références neutres

- [MDN Web Docs - Making PWAs installable](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Guides/Making_PWAs_installable)
- [MDN Web Docs - Web application manifest](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Manifest)
- [MDN Web Docs - Using Service Workers](https://developer.mozilla.org/en-US/docs/Web/API/Service_Worker_API/Using_Service_Workers)
