# Refonte App OS Sobre FilFinance

## Objectifs

Cette refonte transforme FilFinance en interface homogene **App OS sobre**. Le design part du systeme Admin OS valide et l'etend aux pages admin, flux, historique actualités, fil d'actualités, calendrier hebdo, profil, auth et etats transverses.

Objectifs principaux:

- Donner un cockpit global lisible sur `/admin`.
- Donner des vues tri-pane aux surfaces de configuration, glossaire et logs.
- Donner une vue workflow board au resume quotidien.
- Harmoniser feed, profil, auth et flux status avec les memes surfaces, rayons et bordures.
- Ajuster legerement le shell global sans changer sa structure ni ses entrees.
- Factoriser l'UI admin sans deplacer les routes ni modifier les contrats API/WebSocket.
- Extraire les derivations metier dans des hooks view-model et des services purs.
- Renforcer le contraste clair et sombre via une couche design partagee, sans modifier les contrats theme clair/sombre.

## Contraintes Strictes

La refonte ne doit pas modifier:

- les routes publiques;
- la section theme et le switch clair/sombre, ancree en bas de sidebar;
- les contrats backend API ou WebSocket;
- les guards `canAccessAdmin` et `canAccessAppView`;
- les restrictions par role, notamment `/admin/daily-summary` reserve au role `admin`.

Le theme clair/sombre existant est conserve. La couche `useAppDesignTheme` derive des tokens existants et sert de systeme visuel global: surfaces plus blanches et bordures plus nettes en clair, surfaces sombres plus differenciees avec bordures et selections renforcees en dark, rayons compacts 10/12. `useAdminDashboardTheme` reste un wrapper compatible pour le kit admin.

## Directions UI

Direction 1 est appliquee au systeme global admin:

- cockpit `/admin`;
- command bar;
- KPI strip;
- health strip;
- timeline operations;
- panneau d'actions.

Direction 2 est appliquee aux surfaces de registre et console:

- `/admin/config-sources`;
- `/admin/glossary`;
- `/admin/logs`.

Elle prend la forme d'un vrai tri-pane desktop: rail gauche compact, zone centrale dominante et inspector lateral. Sur ecrans etroits, les trois zones passent en pile verticale sans changer les actions.

Direction 3 est appliquee au workflow:

- `/admin/daily-summary`;
- workflow board en colonnes `Collecte`, `Qualification`, `Synthese IA`, `Generation audio`, `Publication`.

Direction calendrier appliquee aux surfaces calendrier:

- `/calendar`;
- `/admin/calendar-history`;
- bloc `FILTRES` unifie avec jours en premier, bouton filtres pleine largeur et chevron partage;
- sous-cartes de filtres importance, type et localisation;
- filtres partages entre prochains evenements et evenements fusionnes.

## Routes Couvertes

### `/admin`

Fonctions conservees:

- etat des sources;
- runtime FinancialJuice;
- runtime X;
- refresh des statuts;
- toggles runtime existants;
- raccourcis vers les surfaces admin;
- restriction des details source au role admin.

Refonte appliquee:

- `AdminPageShell`;
- `AdminCommandBar`;
- `AdminKpiStrip`;
- `AdminTriPane`;
- `AdminHealthStrip`;
- `AdminDataGrid`;
- `AdminEventTimeline`;
- `AdminActionTile`.

Le libelle de navigation courant est `Supervision admin`.

### `/admin/config-sources`

Fonctions conservees:

- activation runtime X;
- comptes X;
- mots-cles;
- refresh;
- statut token bearer;
- statut persistance;
- disabled states lies au token et a la persistance.

Refonte appliquee:

- tri-pane responsive;
- rail source avec switch runtime;
- grilles editables comptes/mots-cles;
- inspector runtime avec champs techniques.

### `/admin/flux-status`

Fonctions conservees:

- etat connexion live;
- runtime live;
- metadonnees audio;
- tests;
- actions existantes de reconnexion et audio.

Refonte appliquee:

- header admin commun;
- KPI strip admin commun;
- reglages de connexion, actualite live, stats runtime et test-runs harmonises avec les panels/items Admin OS.

### `/admin/glossary`

Fonctions conservees:

- CRUD creation/update;
- desactivation;
- priorite;
- notes;
- termes affiches;
- termes audio;
- match mode;
- casse;
- refresh;
- update glossaire pour retraduire les news du jour courant;
- disabled state pendant sauvegarde.

Refonte appliquee:

- tri-pane responsive;
- formulaire dans le rail;
- registre dense via `AdminDataGrid`;
- inspector lateral;
- badges statut/match/priorite/casse.

### `/admin/blacklist`

Fonctions conservees:

- CRUD des regles blacklist;
- activation/desactivation;
- audit des actions recentes.

Evolution appliquee:

- selection du champ filtre: `Titre`, `Contenu` ou `Titre + contenu`;
- libelles operateur bases sur `contenu`;
- aide explicite pour `Contenu`: la news live garde son titre, mais le contenu est retire avant les traitements downstream;
- audit affiche `news rejetee` ou `contenu retire`.

### `/admin/history`

Fonctions conservees:

- diagnostic quotidien;
- jour courant;
- jour precedent;
- badges de niveau detectes depuis `news_level`;
- source;
- traduction;
- metadonnees;
- replay audio;
- affichage J-1.

Refonte appliquee:

- shell admin commun;
- KPI strip admin commun;
- panel diagnostic commun;
- metadonnees en haut de chaque card;
- blocs anglais, francais et TTS complets;
- badge utilisateur en deuxieme ligne a gauche: `active -> Élevée`, `news-general -> Général` ou `none -> Faible`.

Les badges gardent le terme source et le libelle utilisateur pour expliciter la transformation: `active` vient du champ source `news_level` quand il contient le mot `active`, puis l'interface le mappe vers le niveau utilisateur `Élevée`, comme dans le fil d'actualités. `news-general` est mappe vers `Général`; les valeurs absentes ou non reconnues sont affichees `none -> Faible`.

### `/admin/calendar-history`

Fonctions conservees:

- selection de jour;
- filtres calendrier partages;
- evenements fusionnes;
- details complets par evenement;
- audios lies;
- alertes liees.

Refonte appliquee:

- shell admin commun;
- bloc `FILTRES` commun avec jours puis bouton filtres;
- retrait du bloc prochains evenements;
- ordre evenement: importance finale/source/IA/confiance, titre/details, id/date/horaire/localisation/type.

### `/admin/daily-summary`

Fonctions conservees:

- refresh;
- relance forcee;
- reconnexion WebSocket;
- suivi WebSocket;
- etapes;
- artifacts;
- news utilisees;
- textes finaux;
- erreurs run/summary;
- role `admin` requis.

Refonte appliquee:

- workflow board en cinq colonnes;
- inspector run;
- health strip summary/run/artifacts;
- log ribbon des derniers signaux;
- panneaux artifacts, news utilisees et textes finaux.

### `/feed`, `/calendar`, `/profile`, `/login`

Fonctions conservees:

- feed J/J-1, replay et stop audio, badges sources;
- bouton global de resume audio avec icones SVG et libelles d'etat: `Résumé audio du jour`, `Résumé en préparation`, `Résumé indisponible`, `Arrêter la préparation`, `Arrêter la lecture`;
- calendrier hebdo, synthese, prochains evenements et evenements fusionnes;
- session, deconnexion, infos profil et preferences sans ID technique de profil;
- libelles profil accentues: `Voix TTS préférée`, `Thème`, `Mise à jour`, `Création`;
- connexion, creation de compte, erreurs et notices.

Refonte appliquee:

- primitives partagees App OS pour cards, headers, list items, boutons, badges et inputs;
- contraste clair global renforce;
- contraste sombre renforce sans changement de palette;
- feed avec titre FR visible, corps FR replie au clic et aucun texte anglais source;
- calendrier avec statut audio de synthese en haut a droite du bloc synthese et filtres communs.

### `/admin/logs`

Fonctions conservees:

- filtres jour/niveau/recherche;
- live tail;
- pause/reprise;
- reconnexion;
- clear view;
- terminal;
- niveaux;
- taille fichier;
- heartbeat.

Refonte appliquee:

- operator console;
- toolbar de commandes;
- rail filtres;
- zone terminal dominante;
- inspector session;
- terminal live conserve avec auto-scroll controle.

## Architecture

Le kit reusable est situe dans:

```text
src/features/admin/components/dashboard/
```

Les types UI sont situes dans:

```text
src/features/admin/types/dashboard.ts
```

La couche design globale est situee dans:

```text
src/shared/theme/appDesignTheme.ts
```

La couche theme admin compatible est situee dans:

```text
src/features/admin/components/dashboard/theme.ts
```

Les formatters et derivations pures sont situes dans:

```text
src/features/admin/services/dashboardFormatters.ts
```

Les view-model hooks sont situes dans:

```text
src/features/admin/hooks/useAdminOverviewViewModel.ts
src/features/admin/hooks/useAdminSourceSettingsViewModel.ts
src/features/admin/hooks/useAdminGlossaryViewModel.ts
src/features/admin/hooks/useAdminDailySummaryViewModel.ts
src/features/admin/hooks/useAdminLogsViewModel.ts
```

Les routes `app/(app)/admin/*` restent des wrappers fins. Les ecrans composent les hooks, services et composants dashboard sans modifier les contrats backend.

## Anti-Regression

Elements interdits:

- changer les routes publiques;
- changer les contrats API/WebSocket;
- changer les guards ou roles;
- changer les libelles, l'ordre ou les sections du side menu hors demande produit explicite;
- refondre structurellement le shell global;
- rendre visible plus de logs ou artifacts que l'existant;
- retirer les disabled states des actions mutantes;
- importer profondement une autre feature.

Checklist smoke test:

- Le side menu reste sectionne et l'admin reste plat.
- Le theme clair est plus contraste partout via `useAppDesignTheme`.
- Le theme sombre gagne en contraste sur les surfaces, bordures, headers, selections et textes secondaires sans changer de palette globale.
- `Fil d'actualités`, `Calendrier hebdo`, `Profil` et `Login` gardent leurs fonctions.
- `/admin` charge le cockpit et les actions.
- `/admin/config-sources` permet refresh, activation X, comptes et mots-cles.
- `/admin/flux-status` conserve connexion, audio metadata et tests.
- `/admin/glossary` conserve creation, edition et desactivation.
- `/admin/history` conserve J/J-1, news elevees, metadonnees et replay audio.
- `/admin/daily-summary` conserve refresh, relance, WebSocket, etapes, artifacts, news et textes finaux.
- `/admin/logs` conserve filtres, live tail, pause/reprise, reconnexion, clear view et terminal.

## Validation Technique

Commandes de validation obligatoires:

```bash
npm run typecheck
npm run qa:frontend
npm run build:web
```
