# Kit Composants Dashboard Admin

## Emplacement

Le kit est expose depuis:

```text
src/features/admin/components/dashboard/index.tsx
```

Il est aussi exporte par l'index public de la feature admin:

```text
src/features/admin/index.ts
```

Les features externes doivent importer depuis `@features/admin`, jamais depuis `@features/admin/components/dashboard`.

## Types

Types principaux:

- `AdminTone`: ton visuel `critical`, `dark`, `info`, `muted`, `trace`.
- `AdminDashboardTheme`: alias compatible de `AppDesignTheme`.
- `AdminMetric`: tuile de metrique.
- `AdminHealthItem`: item de health strip.
- `AdminCommandAction`: action de command bar.
- `AdminTimelineItem`: evenement de timeline.
- `AdminDataGridColumn<Row>`: colonne de grille.
- `AdminInspectorField`: champ d'inspector.
- `AdminWorkflowStage`: colonne de workflow.
- `AdminWorkflowStep`: ligne d'etape.
- `AdminTriPaneProps`: contrat rail, zone principale et inspector.

## Theme App OS

La couche globale est exposee par:

```text
src/shared/theme/appDesignTheme.ts
```

Elle est consommee par les primitives partagees (`Card`, `Badge`, `ActionButton`, `PageHeaderCard`, `StatTile`, `WebSelect`) et par le kit admin.

### `useAdminDashboardTheme`

Responsabilite:

- deleguer a `useAppDesignTheme`;
- conserver une compatibilite pour les composants admin existants;
- renforcer le contraste du theme clair dans l'ensemble de l'app;
- renforcer le contraste du theme sombre sur surfaces, bordures, selections et textes secondaires;
- fournir couleurs et rayons communs aux panels, grilles, rails, inspectors et terminaux.

Convention:

- ne pas modifier les contrats theme globaux pour ajuster une surface;
- utiliser `dashboardTheme.colors.border` et `dashboardTheme.colors.gridHeader` pour les grilles;
- utiliser `dashboardTheme.radii.control`, `panel` et `section` au lieu de nouveaux rayons inline.

## Composants

### `AdminPageShell`

Responsabilite:

- wrapper de page admin;
- affiche `PageHeaderCard`;
- accepte une command bar optionnelle;
- ne gere pas les droits ni les appels API.

Usage:

```tsx
<AdminPageShell title="Admin" description="Cockpit" commandBar={bar}>
  {content}
</AdminPageShell>
```

### `AdminCommandBar`

Responsabilite:

- presenter le contexte de pilotage;
- afficher actions rapides;
- afficher meta badges;
- rester responsive avec wrap.

### `AdminKpiStrip` et `AdminMetricTile`

Responsabilite:

- presenter les metriques principales;
- utiliser les tons standards;
- eviter de porter de logique metier inline.

Les valeurs doivent venir d'un hook view-model quand elles derivent d'un snapshot.

## Surfaces Metier Branchees

- `AdminBlacklistScreen`: gere les mots/phrases de rejet avant ingestion durable, expose les derniers hits blacklist et suit le style du glossaire.
- `AdminHistoryScreen`: reste la page Debug historique et affiche aussi les audits de contenus FinancialJuice deja tronques upstream.
- Les audits de troncature ne sont pas affiches dans la page Blacklist, car ils concernent la qualite de source/historique plutot que les regles operateur.

### `AdminPanel`

Responsabilite:

- panneau metier compact avec bordure, surface App OS et `CardHeader`;
- surface principale pour grilles, inspectors, timelines et controles.

Ne pas imbriquer inutilement un `AdminPanel` dans un autre `AdminPanel`.

### `AdminListItem`

Responsabilite:

- remplacer les usages admin directs de `CardListItem`;
- fournir une ligne dense avec fond, bordure et rayon App OS;
- accepter un `style` optionnel pour les etats critiques ou informatifs.

### `AdminSection`

Responsabilite:

- section non carte pour regrouper des composants deja encadres;
- utile pour eviter les cartes imbriquees.

### `AdminStatusBadge`

Responsabilite:

- normaliser les badges de statut;
- mapper `AdminTone` vers les variantes `Badge` existantes.

Convention:

- `info`: OK, actif, pret;
- `trace`: suivi, en cours, attention non bloquante;
- `critical`: erreur ou etat bloquant;
- `muted`: absent, inactif, non applicable;
- `dark`: action ou etat principal.

### `AdminHealthStrip`

Responsabilite:

- afficher plusieurs etats operationnels en ligne;
- utile pour cockpit, inspector et workflow.

### `AdminEventTimeline`

Responsabilite:

- afficher des evenements operationnels recents;
- ne charge pas les donnees;
- accepte deja des items pre-formates.

### `AdminDataGrid`

Responsabilite:

- afficher un registre dense responsive;
- accepter des colonnes typees;
- gerer loading et empty state;
- garder les actions dans les cellules.
- afficher headers contrastes, lignes alternees et selection optionnelle.

Convention:

- les colonnes doivent avoir un `minWidth`;
- les cellules longues doivent utiliser `SubtleText` ou `Text` avec break-word;
- les callbacks d'actions restent dans l'ecran ou le hook d'action dedie.
- `selectedRowKey` sert a marquer une ligne active sans changer les donnees.

### `AdminFilterRail`

Responsabilite:

- presenter filtres et selections de contexte;
- utiliser `WebSelect`, `TextInput`, toggles ou autres primitives existantes.

### `AdminInspectorPanel`

Responsabilite:

- afficher les champs techniques ou l'etat courant;
- recevoir `AdminInspectorField[]`;
- peut contenir des blocs complementaires comme erreur ou health strip.

### `AdminTriPane`

Responsabilite:

- composer un rail gauche, une zone centrale dominante et un inspector droit;
- passer automatiquement en pile verticale quand `isWide` est faux;
- accepter `railWidth` et `inspectorWidth` pour cadrer les dashboards metier.

Usage:

```tsx
<AdminTriPane
  isWide={isWideLayout}
  rail={<AdminFilterRail>{filters}</AdminFilterRail>}
  main={<AdminDataGrid rows={rows} columns={columns} keyExtractor={getKey} />}
  inspector={<AdminInspectorPanel fields={fields} />}
/>
```

### `AdminWorkflowBoard`, `AdminStageColumn`, `AdminRunStepRow`

Responsabilite:

- afficher les workflows en colonnes;
- grouper les etapes par stage;
- ne pas connaitre le domaine daily-summary.

La classification des etapes daily-summary est dans `useAdminDailySummaryViewModel`.

### `AdminTerminalPanel`

Responsabilite:

- fournir une surface terminal standard;
- accepter des callbacks de scroll pour conserver l'auto-scroll existant;
- ne parse pas les lignes.

### `AdminLogToolbar`

Responsabilite:

- command bar specialisee console operateur;
- regroupe filtres/session/actions sans remplacer les controles metier.

### `AdminEmptyState` et `AdminErrorState`

Responsabilite:

- etats vides et erreurs reutilisables;
- messages courts;
- pas de logique de retry implicite.

## View-Model Hooks

Les hooks view-model transforment les donnees deja chargees:

- `useAdminOverviewViewModel`;
- `useAdminSourceSettingsViewModel`;
- `useAdminGlossaryViewModel`;
- `useAdminDailySummaryViewModel`;
- `useAdminLogsViewModel`.

Ils ne doivent pas:

- changer les contrats API;
- ouvrir de WebSocket;
- muter le backend;
- lire le theme;
- rendre de JSX lourd.

Ils peuvent:

- compter;
- trier;
- classifier;
- mapper des tons;
- construire des champs inspector;
- construire des colonnes workflow ou timeline.

## Formatters

Les formatters communs sont dans:

```text
src/features/admin/services/dashboardFormatters.ts
```

Ils couvrent:

- labels booleens;
- labels actif/disponible;
- tons runtime/connexion/status;
- format bytes;
- format artifact JSON;
- format terminal;
- normalisation comptes X et mots-cles.

## Responsive

Conventions:

- utiliser `useWindowDimensions` dans les ecrans;
- passer `isWide` a `AdminTriPane`;
- laisser les grilles wrap via `minWidth`;
- ne pas forcer d'overflow horizontal;
- garder les textes longs en `break-words`.

## Accessibilite Et Etats

Chaque page admin doit avoir:

- un header;
- une command bar ou un contexte d'action clair;
- des metriques ou un statut principal;
- une zone principale metier;
- des etats loading, empty et error;
- les disabled states existants sur actions mutantes.

## Anti-Regression

Avant de modifier une page admin:

- verifier qu'elle garde son guard actuel;
- verifier que ses callbacks mutateurs sont toujours branches;
- verifier que les erreurs API/WebSocket restent visibles;
- verifier qu'aucun import profond entre features n'est ajoute;
- verifier que feed/profil/auth gardent leurs fonctions.
- verifier que le shell global ne change pas de structure.

Smoke test par composant:

- `AdminDataGrid`: loading, empty, lignes longues, actions.
- `AdminTriPane`: desktop large, desktop etroit, mobile web, ordre rail/main/inspector.
- `AdminWorkflowBoard`: colonnes presentes meme sans etape.
- `AdminTerminalPanel`: scroll, texte long, niveau couleur.
- `AdminInspectorPanel`: valeurs longues, badges, erreurs.
- `useAppDesignTheme`: contraste clair global et contraste sombre renforce sans changement de palette.
