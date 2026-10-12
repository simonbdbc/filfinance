# NEWS-116 - Tokens theme clair/sombre NativeWind

## Statut

Implementation livree le 3 mai 2026.

## Objectif

Preparer le client a un theme clair/sombre sans dupliquer les couleurs dans chaque composant.

## Scope

- Creer une definition centralisee des tokens de theme:
  - `background`;
  - `surface`;
  - `surfaceElevated`;
  - `border`;
  - `textPrimary`;
  - `textSecondary`;
  - `accent`;
  - `accentText`;
  - `danger`;
  - `dangerSurface`;
  - `success`;
  - `info`;
  - `overlay`;
  - `shadow`.
- Ajouter un provider ou helper de theme cote `src/app`.
- Migrer les primitives `src/shared/ui` vers ces tokens:
  - `ScreenShell`;
  - `Card`;
  - `Button`;
  - `Badge`;
  - `Typography`;
  - `StatTile`;
  - `WebSelect`.
- Remplacer les couleurs SVG hardcodees par une strategie compatible theme:
  - soit icones SVG distinctes light/dark;
  - soit rendu par `Image` avec asset adapte au theme;
  - soit composant icone RN si le projet adopte une lib d'icones plus tard.

## Hors scope

- Switch utilisateur visible.
- Persistance backend de la preference.
- Refonte visuelle globale.

## Criteres d'acceptation

- Les primitives UI ne contiennent plus de hex metier disperses.
- Le theme clair reproduit le rendu actuel a ecart minimal.
- Le theme sombre dispose de tokens explicites et contrastes.
- `npm run typecheck` passe.
- `npm run build:web` passe.

## Implementation

- Ajout de `src/shared/theme/theme.tsx`:
  - tokens `light`;
  - tokens `dark` avec une palette sombre neutre type Codex dark;
  - surfaces sombres sans blanc, le blanc et quasi-blanc etant reserves aux textes et icones;
  - `ThemeProvider`;
  - `useTheme`;
  - `useThemeTokens`.
- Ajout de `src/shared/theme/classColor.ts` pour conserver la priorite des classes de couleur explicites passees par les features.
- `App.tsx` enveloppe maintenant le client dans `ThemeProvider`.
- Migration des primitives `src/shared/ui` vers les tokens:
  - `ScreenShell`;
  - `Card`;
  - `Button`;
  - `Badge`;
  - `Typography`;
  - `StatTile`;
  - `WebSelect`.
- Migration du shell sidebar et de la navigation vers les tokens pour preparer le switch de `NEWS-117`.
- Ajout des variantes SVG sombres:
  - `menu-dark.svg`;
  - `close-dark.svg`.

## Validation locale

- `npm run typecheck`
- `npm run build:web`
- `npm run web -- --port 19007 --non-interactive`, serveur Expo stable jusqu'a l'arret manuel.
