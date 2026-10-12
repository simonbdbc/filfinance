# NEWS-121 - Fix build Cloudflare Pages NativeWind cache

## Statut

Implementation livree le 3 mai 2026.

## Objectif

Corriger l'echec Cloudflare Pages:

```text
Failed to get the SHA-1 for:
node_modules/react-native-css-interop/.cache/web.css
```

## Cause

`react-native-css-interop` genere `web.css` dans son dossier `.cache` pendant le build Metro. En environnement frais Cloudflare Pages, Metro peut construire sa file map avant que ce fichier n'existe. Le resolver pointe ensuite vers un fichier que Metro n'a pas indexe, ce qui bloque le calcul SHA-1.

Le probleme ne se reproduit pas forcement en local si un build precedent a deja cree `node_modules/react-native-css-interop/.cache/web.css`.

## Implementation

- Ajout du script `filfinance-client/scripts/prepare-nativewind-web-cache.js`.
- Ajout du lifecycle npm `prebuild:web`.
- Le script cree `node_modules/react-native-css-interop/.cache/web.css` avant `expo export`.
- La valeur `engines.npm` accepte maintenant `>=11.8.0 <12` pour correspondre au runtime Cloudflare Pages observe.

## Criteres d'acceptation

- Un build web fonctionne meme si `web.css` est absent avant lancement.
- `npm run build:web` lance automatiquement `prebuild:web`.
- Le warning `EBADENGINE` npm Cloudflare ne doit plus apparaitre pour npm `11.8.x`.

## Validation locale

- `web.css` de cache deplace hors du dossier `.cache` avant test.
- `npm run typecheck`: OK.
- `npm run build:web`: OK, avec execution de `prebuild:web`.
