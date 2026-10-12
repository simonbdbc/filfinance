# NEWS-118 - QA theme sombre, contrastes et non-regression responsive

## Statut

Validation technique effectuee le 3 mai 2026.

La validation visuelle responsive est realisee en live, hors harness de test navigateur versionne.

## Objectif

Valider que le theme sombre est utilisable, lisible et sans regression responsive avant de considerer le lot theme comme termine.

## Scope

- Verifier les contrastes des textes principaux, secondaires, erreurs, succes et badges.
- Verifier les ecrans:
  - Live;
  - Profil;
  - Tests;
  - Admin;
  - AuthGate.
- Verifier la sidebar ouverte et fermee.
- Verifier les largeurs `320`, `375`, `414`, `768`, `1024`.
- Verifier que les SVG menu/close restent visibles dans les deux themes.
- Verifier que les inputs web et selects conservent bordure, fond et texte lisibles.

## Criteres d'acceptation

- Aucun overflow horizontal sur les largeurs ciblees.
- Les textes restent lisibles en clair et sombre.
- Les etats danger/success/info ne perdent pas leur intention visuelle.
- Les notes de validation live sont ajoutees au ticket si une regression est observee.
- `npm run typecheck` passe.
- `npm run build:web` passe.

## Validation effectuee

### Gates techniques

- `npm run typecheck`: OK.
- `npm run build:web`: OK.
- `npm run web -- --port 19007 --non-interactive`: OK, serveur Expo stable jusqu'a l'arret manuel.
- Export web: les assets SVG `menu`, `close`, `menu-dark`, `close-dark` sont bien exportes.
- Scan statique: aucun hex ou `rgba(...)` ne reste dans les composants applicatifs.

Les couleurs applicatives sont maintenant portees par `src/shared/theme/theme.tsx`, avec exceptions attendues pour les assets SVG d'icones.

### Contrastes calcules

Ratios WCAG calcules depuis les tokens. Tous les couples controles sont au-dessus de `4.5:1`.

| Theme | Couple | Ratio |
| --- | --- | ---: |
| clair | `textPrimary` / `surface` | `17.08` |
| clair | `textSecondary` / `surface` | `7.28` |
| clair | `textMuted` / `surface` | `4.66` |
| clair | `accent` / `surface` | `5.37` |
| clair | `dangerText` / `dangerSurface` | `7.56` |
| clair | `dangerText` / `dangerBadgeSurface` | `6.80` |
| clair | `successText` / `surface` | `6.87` |
| clair | `infoText` / `infoSurface` | `8.88` |
| clair | `traceText` / `traceSurface` | `8.93` |
| clair | `primaryText` / `primaryBackground` | `17.74` |
| sombre | `textPrimary` / `surface` | `16.44` |
| sombre | `textSecondary` / `surface` | `12.09` |
| sombre | `textMuted` / `surface` | `7.11` |
| sombre | `accent` / `surface` | `7.11` |
| sombre | `dangerText` / `dangerSurface` | `9.14` |
| sombre | `dangerText` / `dangerBadgeSurface` | `8.40` |
| sombre | `successText` / `surface` | `12.77` |
| sombre | `infoText` / `infoSurface` | `9.14` |
| sombre | `traceText` / `traceSurface` | `13.46` |
| sombre | `primaryText` / `primaryBackground` | `12.28` |

Note: le theme sombre utilise maintenant une palette neutre type Codex dark: fond presque noir, surfaces gris neutres, accent gris moyen, sans dominante brun/espresso. Le blanc et quasi-blanc sont reserves aux textes et aux icones, pas aux surfaces ou boutons actifs.

## Validation live

La mesure runtime `scrollWidth == clientWidth` sur `320`, `375`, `414`, `768`, `1024` est volontairement validee en live par l'utilisateur.

Checklist live:

1. Ouvrir `http://localhost:19007`.
2. Tester les viewports `320`, `375`, `414`, `768`, `1024`.
3. Ouvrir/fermer la sidebar en theme clair et sombre.
4. Basculer le switch theme en bas de sidebar.
5. Confirmer:
   - aucun scroll horizontal;
   - SVG menu/close visibles dans les deux themes;
   - inputs/selects lisibles;
   - textes danger/success/info lisibles.
