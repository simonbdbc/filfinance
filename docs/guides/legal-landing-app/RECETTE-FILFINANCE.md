# Pages légales Fil Finance — intégration et recette locale

12 septembre 2026. Le [manifeste approuvé](./MANIFESTE.md) fixe les informations et le périmètre. Les textes sont dans la landing ; l’app contient les liens et la notice d’information. Aucun déploiement, création de compte réel, changement de base/API ou preuve d’acceptation n’a été réalisé.

## Configuration

| Variable | Valeur / règle |
| --- | --- |
| `PUBLIC_LEGAL_ENTITY_NAME` | `SB ALGO` |
| `PUBLIC_LEGAL_REGISTERED_OFFICE` | `60 rue François 1er, 75008 Paris` |
| `PUBLIC_LEGAL_PUBLICATION_DIRECTOR` | `sbdev` |
| `PUBLIC_LEGAL_PRIVACY_CONTACT` | `support@filfinance.fr` |
| `PUBLIC_LEGAL_DATA_RETENTION` | Texte complet dans `apps/landing/.env.example` ; compte existant et demande au support, sans promesse de 30 jours |
| `PUBLIC_LEGAL_PUBLICATION_ENFORCED` | `true` dans `landing:check` : cinq informations obligatoires, non provisoires et identiques au HTML |
| `EXPO_PUBLIC_LANDING_ORIGIN` | Local `http://localhost:4321`, DEV `https://dev.filfinance.fr`, PROD `https://filfinance.fr` |

Les cinq informations publiques ont des valeurs locales approuvées dans [legal.config.ts](../../../apps/landing/src/config/legal.config.ts). La CI les fournit explicitement, avec les priorités GitLab habituelles. Le mode strict refuse une variable absente même si une valeur locale par défaut existe. Les variables doivent être exportées pour **le build et la validation**.

L’origine PWA est lue par [runtimeEnv.ts](../../../apps/pwa/src/shared/config/runtimeEnv.ts). Le prébuild utilise `@expo/env` pour lire les mêmes fichiers que l’export Expo, puis refuse une origine invalide. Le serveur statique et Docker conservent la surcharge publique existante. HTTPS est requis hors loopback ; chemin, identifiants, paramètres et fragment sont refusés.

## Commandes courantes

Depuis la racine, avec les dépendances déjà installées :

```sh
set -a
source apps/landing/.env.example
set +a
npm run check:landing
npm run check:pwa
EXPO_PUBLIC_LANDING_ORIGIN=http://localhost:4321 npm run build:pwa
CHROME_PATH=/chemin/vers/chromium npm run lighthouse:landing
```

`check:landing` effectue le typecheck, les tests du thème, le build des cinq pages, les tests négatifs légaux puis la validation du HTML. Le validateur ne se contente pas de chercher des fichiers : il vérifie titre, description, langue, canonical, H1, date, liens, ancres d’accueil, sitemap et script de thème exact. Il refuse notamment un document remplacé par l’accueil ou une information modifiée après le build.

`npm --prefix apps/pwa run qa:legal` teste les URL et la surcharge publique. Le contrôle d’artefact s’exécute après un build configuré pour l’environnement correspondant :

```sh
node apps/pwa/scripts/check-web-public-env.cjs --target dev --dist apps/pwa/dist
# Ou --target prod pour un build PROD.
```

Les cinq variables légales sont publiques, aucun secret n’est requis pour ces contrôles. Les autres variables Expo doivent rester cohérentes avec l’environnement utilisé.

## Recette navigateur reproductible

Les audits Lighthouse existants incluent les 80 contrôles d’accueil et 32 rendus légaux. Pour ajouter le parcours PWA isolé, construire une PWA locale avec des destinations de test :

```sh
EXPO_PUBLIC_LANDING_ORIGIN=http://127.0.0.1:4321 \
EXPO_PUBLIC_BETTER_AUTH_URL=http://127.0.0.1:9999 \
EXPO_PUBLIC_API_URL=http://127.0.0.1:9999 \
EXPO_PUBLIC_FILFINANCE_WS_URL=ws://127.0.0.1:9999/ws/news \
npm run build:pwa
```

Puis lancer dans deux terminaux :

```sh
npm --prefix apps/landing run preview -- --host 127.0.0.1 --port 4321
```

```sh
node apps/pwa/scripts/serve-static.cjs apps/pwa/dist 8053
```

Enfin :

```sh
PWA_LEGAL_QA_ORIGIN=http://127.0.0.1:8053 \
CHROME_PATH=/chemin/vers/chromium npm run lighthouse:landing
```

Le script [pwa-legal-browser-qa.cjs](../../../apps/landing/scripts/pwa-legal-browser-qa.cjs) utilise le build réel avec des réponses de compte, profil et API simulées. Il bloque les requêtes hors des origines locales attendues, neutralise le WebSocket et ne soumet aucun compte. Il couvre l’inscription et le profil dans les deux thèmes à 320, 390, 768 et 1440 px, puis ouvre la confidentialité au clavier dans un nouvel onglet et vérifie la conservation des champs nom/email au retour. La revalidation de session au retour du focus est attendue avant de contrôler le formulaire.

**Redémarrer l’aperçu Astro après l’ajout d’une route** : un serveur déjà lancé peut conserver l’ancien manifeste de routes et retourner 404 malgré la présence du fichier HTML. La recette des documents dans Lighthouse démarre un serveur statique frais.

## Résultats locaux

| Vérification | Résultat |
| --- | --- |
| Astro | 28 fichiers vérifiés ; aucune erreur, alerte ou indication |
| Thème et scripts | 15 tests réussis |
| Documents légaux | 15 tests réussis, dont cas négatifs : cinq variables manquantes, valeur provisoire, page absente, repli sur accueil, canonical, liens, sitemap, contenu provisoire, script ajouté, configuration modifiée |
| Build landing strict | 5 pages HTML réelles ; sitemap et validation de production réussis |
| Contrôles PWA | Contrats frontend existants, 11 tests auth, 12 tests légaux et TypeScript réussis |
| Builds PWA | Exports locaux, DEV et PROD réussis ; contrôles des origines dans les artefacts DEV/PROD réussis |
| Prébuild négatif PWA | Origine vide refusée avant export |
| Navigateur accueil | 80 contrôles réussis |
| Navigateur documents | 32 rendus : 4 documents × 4 largeurs × 2 thèmes ; aucun débordement, liens et bascule au clavier validés |
| Navigateur PWA | 16 rendus : inscription et profil × 4 largeurs × 2 thèmes ; liens et couleurs réelles vérifiés ; nouvel onglet et formulaire préservé |
| Lighthouse accueil | 12 audits : 3 mobile et 3 desktop par thème ; performance, accessibilité, bonnes pratiques et SEO **100** à chaque passage |
| CI | YAML analysé ; cinq variables présentes ; publications PWA dépendantes de `landing:check` ; règles de déclenchement comparées à HEAD et inchangées |

Environnement local : macOS, Node 24.18.0, npm 11.16.0, Chromium fourni localement. Le dépôt demande Node 24.13.1 ; aucun changement de cette exigence n’est inclus. Le prébuild peut afficher l’avertissement Node sur le module TypeScript sans champ `type`, sans échec du contrôle.

Les [résultats navigateur légaux](./recette/2026-09-12/browser-qa.json), [résultats PWA simulés](./recette/2026-09-12/pwa-browser-qa.json), [builds DEV/PROD](./recette/2026-09-12/pwa-builds.json), [scores Lighthouse](./recette/2026-09-12/lighthouse-scores.json) et [empreintes SHA-256](./recette/2026-09-12/empreintes.json) identifient le périmètre de preuve. Les rapports détaillés restent dans `apps/landing/reports/` (ignorés par Git). Les empreintes décrivent le working tree, pas uniquement son commit de base.

## Captures

Captures de l’implémentation locale, distinctes des maquettes EPIC-038. La confidentialité est illustrée à 390 et 1440 px ; l’inscription et le profil utilisent un utilisateur fictif à 390 px. Les pages longues peuvent être affichées réduites dans l’aperçu Markdown.

| Surface | Clair | Sombre |
| --- | --- | --- |
| Confidentialité mobile | [Capture](./recette/2026-09-12/confidentialite-light-390.png) | [Capture](./recette/2026-09-12/confidentialite-dark-390.png) |
| Confidentialité desktop | [Capture](./recette/2026-09-12/confidentialite-light-1440.png) | [Capture](./recette/2026-09-12/confidentialite-dark-1440.png) |
| Inscription PWA | [Capture](./recette/2026-09-12/pwa-registration-light-390.png) | [Capture](./recette/2026-09-12/pwa-registration-dark-390.png) |
| Profil PWA simulé | [Capture](./recette/2026-09-12/pwa-profile-light-390.png) | [Capture](./recette/2026-09-12/pwa-profile-dark-390.png) |

![Profil connecté simulé, thème sombre](./recette/2026-09-12/pwa-profile-dark-390.png)

## Maintenance et limites

Modifier les textes dans les quatre pages Astro, puis `updatedAt` et `updatedAtIso` dans `legal.config.ts`. Lors d’un changement d’identité ou de contact, aligner les cinq valeurs dans cette configuration, l’exemple et la CI, ainsi que la notice courte de [LegalLinks.tsx](../../../apps/pwa/src/shared/legal/LegalLinks.tsx). Refaire les contrôles, puis la recette de l’environnement effectivement publié. Les historiques de recette EPIC-038 ne sont pas réécrits.

Le chemin natif utilise `Linking.openURL` avec message en cas d’échec ; aucun simulateur ou appareil iOS/Android n’a été utilisé dans cette recette. Les résultats PWA connectés reposent sur une API simulée et ne prouvent pas une session serveur réelle. Les douze audits Lighthouse portent sur l’accueil ; les pages légales bénéficient des contrôles HTML et navigateur décrits ci-dessus, sans prétendre à douze audits supplémentaires de ces documents.

Aucun pipeline distant ni déploiement n’a été déclenché. Les réponses CDN, paramètres fournisseurs, contrats, régions de traitement et réception effective des emails ne sont pas certifiés par ces tests locaux. L’intégration reprend le périmètre documentaire approuvé ; elle ne constitue pas une certification juridique.
