# NEWS-115 - Audit palette client et cadrage theming clair/sombre

## Statut

Ticket planifie le 3 mai 2026.

## Contexte

Le client utilise aujourd'hui des couleurs directement dans les classes NativeWind et quelques styles React Native inline. Cette approche a permis d'aller vite pendant la factorisation frontend, mais elle rend l'ajout d'un theme sombre couteux si chaque composant doit etre retouche manuellement.

## Audit palette actuelle

Couleurs dominantes relevees dans `filfinance-client/src`:

| Couleur | Usage actuel | Frequence approximative |
| --- | --- | ---: |
| `#111827` | texte fort, bouton primaire, nav active | 16 |
| `#1f2937` | texte principal, icones SVG, loader | 14 |
| `#6b7280` | texte secondaire | 11 |
| `#8b5e3c` | accent brun, titres courts, eyebrows | 9 |
| `#d6c3a5` | bordures inputs/boutons secondaires | 8 |
| `#881337`, `#9f1239`, `#991b1b` | erreurs/danger | 11 cumule |
| `#4b5563` | texte descriptif | 4 |
| `#fffaf3`, `#f6f1e8`, `#f6efe4` | surfaces ivoire/beige | 4 cumule |
| `#e8dcc7`, `#eadfcf`, `#f0e6d8`, `#efe5d3` | bordures/surfaces secondaires beige | 5 cumule |
| `#166534` | succes | 2 |
| `#3730a3`, `#c7d2fe`, `#eef2ff` | info | 3 cumule |
| `#2f4f4f` | badge trace | 1 |
| `rgba(17, 24, 39, 0.38)` | backdrop sidebar | 2 |
| `rgba(17, 24, 39, 0.22)` | ombre sidebar web | 1 |
| `bg-white`, `text-white`, `#ffffff` | surfaces et texte inverse | 13 cumule |

Constat visuel:

- la palette lit aujourd'hui comme une interface claire beige/ivoire avec texte charbon;
- l'accent brun est coherent avec la charte actuelle, mais renforce une dominante beige/tan;
- les couleurs statut existent, mais elles ne sont pas tokenisees;
- les icones `menu.svg` et `close.svg` codent directement `#1F2937`;
- `WebSelect` utilise encore des couleurs inline;
- la sidebar, les cards, badges, boutons, textes et inputs repetent les memes couleurs sous forme de hex.

## Risques

- Un theme sombre implemente composant par composant risque de dupliquer toute la palette.
- Les couleurs inline et SVG hardcodes ne basculeront pas automatiquement.
- Les contrastes du theme sombre peuvent diverger entre primitives UI et composants feature.
- Les preferences utilisateur existent deja cote profil; le theme doit s'integrer proprement sans creer une seconde source de verite.

## Decision de cadrage

Le theme doit etre porte par une couche frontend explicite avant d'ajouter le switch:

- centraliser les tokens `light` et `dark`;
- exposer un `ThemeProvider` ou helper equivalent au niveau `src/app`;
- migrer d'abord les primitives `shared/ui`;
- migrer ensuite les features;
- garder un fallback local si le profil utilisateur n'est pas encore charge;
- persister la preference theme dans `user_profiles.preferences` quand le backend le supporte.

## Criteres d'acceptation de l'audit

- La palette actuelle est documentee.
- Les points de hardcoding bloquants sont listes.
- Les tickets de dev du theme clair/sombre sont rattaches a `EPIC-013`.
- Le switch de theme est prevu en bas de sidebar.

