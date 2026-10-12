# EPIC-039 — Refonte PWA avec gluestack-ui v5, UniWind et Tailwind CSS 4

## Statut

**Terminé localement — 17 septembre 2026.** NEWS-227 à NEWS-237 sont livrés.
Toute la PWA utilise **gluestack-ui v5 + UniWind + Tailwind CSS 4** ; NativeWind,
css-interop, Tailwind 3 et les composants/adaptateurs remplacés sont retirés.
Les maquettes, palettes et typographies restent les références communes avec la landing.

[Recette locale, captures, résultats et empreintes](../design/pwa/recette/2026-09-16/README.md).
Les preuves utilisent des API et sessions simulées et un média HLS local réellement
lu. Elles ne constituent pas une preuve de production ou de livraison native.
Aucun commit, push ou déploiement n’a été effectué.

## Objectif métier

Uniformiser toute la PWA avec la landing en conservant ses palettes claire/sombre
et ses polices système. La direction retenue « La table de session » privilégie
la lecture des actualités, la préparation du calendrier et la continuité audio.
La refonte visuelle s'accompagne d'une refonte structurelle en composants gluestack
réutilisables, afin que les pages utilisateur, authentification et administration
partagent les mêmes règles, états et comportements accessibles.

La recette porte sur la **PWA Web sur mobile et ordinateur**. Les composants
gardent leur compatibilité Expo, sans livraison native iOS/Android dans cet EPIC.

## Références et livrables

- [Référentiel PWA : palettes, typographie, architecture, routes et galerie](../design/pwa/README.md).
- [Provenance et empreintes des maquettes](../design/pwa/maquettes/manifest.json).
- [Vérification de la livraison documentaire](../design/pwa/recette-documentaire-2026-09-16.md).
- [Charte de la landing](../design/landing/README.md) et
  [EPIC-038](./EPIC-038-refonte-visuelle-landing-themes-clair-sombre.md).
- [Architecture existante](../architecture.md) et [contrat des tickets](../ROADMAP.md).

Cinq planches actives : direction générale, Actualités, Calendrier, Profil et
Administration. Les cinq originaux ImageGen sont conservés à l'identique en
**1644 × 957 px**, avec nom source, date, taille et SHA-256. Les originaux de la
direction et de Calendrier sont historiques ; leurs versions corrigées montrent
lundi à vendredi. Chaque planche associe ordinateur clair et mobile sombre.
Les règles écrites priment sur les approximations de l'image ; aucune image n'est
une preuve de recette ni une ressource à embarquer dans l'application.

## État de départ

Expo Router, organisation par fonctionnalités, NativeWind 4 et un provider de
thème sont déjà présents. `shared/theme/tokens.ts` et `appDesignTheme.ts` portent
des valeurs visuelles à unifier au début de l’EPIC ; les composants partagés et compositions admin
existants doivent être migrés progressivement. gluestack n’était pas intégré avant cette livraison.

Le calendrier métier utilise déjà cinq jours ouvrés. L'authentification, les
services audio, les liens légaux et les autorisations admin sont des contrats
existants à conserver. L'inventaire du référentiel couvre **16 routes et 3 layouts**,
dont toutes les routes admin et les modes internes de connexion/inscription/
récupération de mot de passe.

Préserver les modifications locales préexistantes du lecteur de logs admin :
`apps/pwa/src/features/admin/lib/rustLogger/api.ts`, `headers.ts` dans le même
dossier, et `apps/pwa/tests/rustLoggerHeaders.test.mjs`. Leur intégration visuelle
future ne doit pas rétablir une ancienne version de leur comportement.

## Périmètre visuel

| Usage | Clair | Sombre |
| --- | --- | --- |
| Fond principal | `#F1E5D3` | `#111827` |
| Surfaces | `#FBF5EB` | `#192535` |
| Texte principal | `#382E26` | `#FBF5EB` |
| Texte secondaire | `#796B5D` | `#F1E5D3` |
| Accent décoratif | `#7B5E45` | `#8B5E3C` |
| Bouton principal | `#7B5E45` | `#714426` |
| Texte du bouton | `#FBF5EB` | `#FBF5EB` |

Le référentiel définit aussi sélection, bordures, focus, survol, désactivation,
succès et erreur. Petit texte sur avoine en brun principal (taupe : environ
4,15:1) ; cuivre sombre décoratif, libellés interactifs contrastés. Une source
volontairement désactivée reçoit un état neutre, jamais une erreur rouge.
Tous les états comportent du texte ou une icône explicite.

Titres et marque : `Georgia, "Times New Roman", Times, serif`. Corps, tableaux et
contrôles : `Arial, Helvetica, sans-serif`. Monospace conservée pour les logs
techniques. Titres 36 px ordinateur / 30 px mobile, sections 24 px, corps 16 px,
métadonnées au moins 14 px. Espacements 4/8/12/16/24/32 px ; rayons contrôles 6 px,
surfaces 8 px ; cibles tactiles au moins 44 × 44 px. Polices système sans téléchargement
propriétaire ; assets de marque existants, dont le favicon arrondi, sans nouveau logo.

## Architecture cible

Conserver Expo Router et les frontières entre fonctionnalités. La filière
[gluestack v5 UniWind](https://gluestack.io/ui/docs/guides/more/upgrade-to-v5)
est verrouillée : `@gluestack-ui/core@5.0.15`, `@gluestack-ui/utils@5.0.6`,
`uniwind@1.3.0`, `tailwindcss@4.1.18`, `react-native-svg@15.15.3` et
`@expo/vector-icons@15.1.1`. Un premier export Web du socle a précédé la migration
des écrans. Metro utilise `withUniwindConfig`, `global.css` et `uniwind-types.d.ts`.
Aucune étape PostCSS n’est ajoutée. NativeWind, css-interop, le preset Babel,
jsxImportSource, Tailwind 3 et la préparation de son cache sont retirés.

Trois niveaux, alimentés par les mêmes tokens :

1. Primitives gluestack isolées derrière les exports UI partagés : boutons,
   textes, champs, sélecteurs, badges, dialogues, panneaux mobiles et chargement.
2. Composants Fil Finance : en-tête, section, ligne, tableau responsive, détail,
   filtres, états asynchrones, statuts et préférences. Données/états/callbacks en
   entrée, aucune API ni connaissance d'une fonctionnalité métier.
3. Compositions métier dans les fonctionnalités existantes : hooks, permissions,
   données et actions reliés aux composants communs ; routes minces.

Centraliser tokens et état de thème ; retirer progressivement les palettes
concurrentes, les variantes admin dupliquées et les styles visuels d'écran.
Les adaptateurs de transition doivent disparaître avant clôture. Conserver les
contrôles d'architecture ; commenter les futures modifications en français.

## Comportements conservés et cible responsive

- Sidebar persistante dès **1024 px** ; en dessous, navigation inférieure et menu
  complémentaire. Administration accessible uniquement au rôle autorisé.
- Liste/détail côte à côte dès **1280 px** ; en dessous, panneau avec retour,
  gestion du focus et fermeture accessible. Pas de débordement de page.
- Lecteur audio commun persistant dans le cadre connecté, absent de l'auth.
  Réutiliser les moteurs et abonnements existants, sans autoplay ajouté ni
  redémarrage à la navigation ou au changement de thème.
- Conserver `filfinance.theme`, `light`/`dark`, clair par défaut et préférence
  indépendante de la landing. Un état pilote gluestack, UniWind et les
  composants. Intercepter les erreurs de stockage, préserver les choix explicites
  face à une lecture tardive, éviter l'affichage transitoire dans le mauvais thème.
- Actualités : historique, sélection, détail, chargement et contrôles de sources
  existants ; aucune permission ni donnée illustrative nouvelle. Utiliser la
  [maquette Actualités confirmée, sans retouche](../design/pwa/maquettes/pwa-actualites-v1.png),
  sans calendrier ni aperçu hebdomadaire ; la navigation conserve l'accès à
  la page Calendrier distincte.
- Calendrier : **cinq jours ouvrés**, sélection du jour et repli week-end existants,
  filtres, événements et détail. Les maquettes montrent le 14–18 septembre 2026.
- Profil : compte en lecture seule, thème, préférence vocale affichée, déconnexion
  et informations légales ; aucun nouveau formulaire de modification de compte.
- Auth : connexion, inscription, vérification email, récupération et réinitialisation
  de mot de passe ; notice de collecte et accès aux documents légaux préservés.
- Admin : toutes les routes de configuration, supervision, historiques, calendrier,
  résumé audio et logs. Préserver les permissions, actions et lectures bornées.

## Hors périmètre

Changement d'API publique, de schéma de
données, d'authentification, de contrat audio ou de règles métier ; nouvelle offre,
gamma, graphiques de trading ou carnet d'ordres ; changement de charte de la landing ;
nouveau logo ou police propriétaire ; migration native iOS/Android ; installation
ou diffusion des PNG dans l'app ; livraison native ; commit, push et déploiement.

## Tickets et dépendances

| Ticket | Livraison | Dépendances | Statut |
| --- | --- | --- | --- |
| [NEWS-227](../tickets/NEWS-227-referentiel-visuel-pwa.md) | Référentiel, corrections et archivage des maquettes, liens | Aucune | Terminé — documentation vérifiée |
| [NEWS-228](../tickets/NEWS-228-socle-gluestack-themes-pwa.md) | gluestack, tokens, typographie et thème unifié | NEWS-227 | Terminé localement |
| [NEWS-229](../tickets/NEWS-229-composants-reutilisables-pwa.md) | Primitives et composants Fil Finance | NEWS-228 | Terminé localement |
| [NEWS-230](../tickets/NEWS-230-navigation-audio-persistant-pwa.md) | Cadre commun, navigation responsive et audio persistant | NEWS-229 | Terminé localement |
| [NEWS-231](../tickets/NEWS-231-refonte-actualites-pwa.md) | Actualités et détail | NEWS-230 | Terminé localement |
| [NEWS-232](../tickets/NEWS-232-refonte-calendrier-pwa.md) | Calendrier ouvré | NEWS-230 | Terminé localement |
| [NEWS-233](../tickets/NEWS-233-refonte-profil-pwa.md) | Profil et informations légales | NEWS-230 | Terminé localement |
| [NEWS-234](../tickets/NEWS-234-refonte-authentification-pwa.md) | Authentification | NEWS-229 | Terminé localement |
| [NEWS-235](../tickets/NEWS-235-refonte-admin-supervision-pwa.md) | Supervision, sources, blacklist et glossaire | NEWS-230 | Terminé localement |
| [NEWS-236](../tickets/NEWS-236-refonte-admin-historiques-logs-pwa.md) | Historiques, calendrier, résumé audio et logs | NEWS-235 | Terminé localement |
| [NEWS-237](../tickets/NEWS-237-recette-refonte-pwa.md) | Retrait de l'ancien UI, recette globale et preuves | NEWS-231 à NEWS-236 | Terminé localement |

Les dépendances externes sont le socle Expo/UniWind, les contrats auth/audio/
calendrier/admin existants et la charte documentée par EPIC-038. Les onze tickets
suivent le contrat du dépôt sans réécrire les bilans historiques des EPICs antérieurs.

## Critères de clôture

### Livraison documentaire — NEWS-227 uniquement

- Identifiants uniques, onze tickets complets et liens relatifs valides.
- Galerie consultable : cinq planches actives, cinq originaux exacts conservés,
  sept PNG au total, dimensions et empreintes enregistrées.
- Deux références corrigées à cinq jours ouvrés avec dates cohérentes ; composition
  vérifiée visuellement, originaux disponibles séparément.
- Inventaire exhaustif des routes/layouts et rattachement aux tickets ; palettes,
  typographie, comportements et différences avec les images explicités.
- Modifications locales du lecteur de logs préservées ; aucune implémentation
  présentée comme livrée ni maquette présentée comme une capture de recette.

### Implémentation — clôture de l’EPIC après NEWS-237

- Toutes les routes migrées, anciens composants/adaptateurs et palettes concurrentes
  retirés ; aucun import métier ou accès API ajouté aux composants partagés.
- Deux thèmes à **320, 390, 768, 1024 et 1440 px**, zoom 200 %, sans débordement
  horizontal de page ; seuil liste/détail également vérifié à 1279/1280 px.
- Clavier, focus visible et retour des panneaux, noms/états accessibles, lecteur
  d'écran, contrastes et cibles tactiles vérifiés.
- Thème restauré avant le contenu, défaut clair, stockage indisponible ; navigation
  et bascule pendant une lecture sans interruption ou abonnement supplémentaire.
- États chargé, chargement, vide, erreur, déconnecté, accès refusé et reconnexion ;
  parcours utilisateur/admin, règles du calendrier, filtres, auth et liens légaux.
- Toutes les fonctions admin conservées, notamment la lecture bornée des logs,
  les permissions et les actions de configuration/supervision.
- `npm run check:pwa`, `npm run build:pwa` et contrôles d'architecture réussis.
  Les attentes visuelles des validateurs sont adaptées sans retirer leurs contrôles
  métier. Tests comportementaux des composants réutilisables et audit des imports/tokens.
- Captures comparatives datées et preuves locales reproductibles associées à l'état
  testé. Tout déploiement ultérieur a ses preuves séparées ; aucun déploiement
  n’est requis ou inclus dans cette livraison locale.
