# NEWS-114 - Side menu et pages role-based profil/tests/admin

## Statut

Implementation livree le 2 mai 2026.

## Objectif

Ajouter une navigation applicative claire au client `filfinance`:

- un side menu;
- une page `Profil`;
- une page `Tests` accessible aux roles `moderator` et `admin`;
- une page `Admin` accessible au role `admin` uniquement.

Le but est d'exploiter la factorisation frontend deja livree par `EPIC-013`, sans recreer un ecran monolithique.

## Scope

- Ajouter un shell applicatif avec navigation laterale.
- Garder un comportement responsive compatible web desktop et mobile.
- Afficher les entrees de menu selon le role du profil courant.
- Bloquer cote client l'acces direct a une page non autorisee.
- Deplacer ou reutiliser les composants existants de profil/session dans la page `Profil`.
- Deplacer ou reutiliser les composants existants de `test-runs` dans la page `Tests`.
- Ajouter une page `Admin` minimale, reservee au role `admin`, prete a recevoir les futurs outils d'administration.
- Conserver la page live/news comme vue principale.

## Hors scope

- Migration des roles vers Supabase Postgres: portee par `EPIC-017`.
- CRUD complet d'administration des utilisateurs.
- Gestion avancee des permissions.
- Refonte visuelle globale de l'application.
- Migration obligatoire vers `expo-router`.

## Approche technique

Le premier lot doit rester incremental:

- utiliser un etat de vue interne si cela suffit au shell actuel;
- conserver `FilFinanceApp` comme orchestrateur haut niveau, mais extraire le shell et les pages dans `src/app` ou `src/features`;
- centraliser les regles de role dans un helper frontend nomme, par exemple `canAccessTests` et `canAccessAdmin`;
- ne jamais considerer ces checks frontend comme une securite suffisante: les routes backend restent protegees par le middleware Supabase.

Structure cible indicative:

```text
filfinance-client/src/app/
  AppShell.tsx
  AppNavigation.tsx
  FilFinanceApp.tsx

filfinance-client/src/features/profile/
  ProfilePage.tsx

filfinance-client/src/features/test-runs/
  TestsPage.tsx

filfinance-client/src/features/admin/
  AdminPage.tsx

filfinance-client/src/shared/auth/
  roles.ts
```

La structure exacte peut etre ajustee pendant l'implementation si le code existant indique un meilleur emplacement.

## Implementation

- `FilFinanceApp` reste l'orchestrateur des sockets, de l'audio et du chargement du profil.
- `AppShell` porte le layout principal et une sidebar gauche masquee par defaut.
- Le bouton icone place en haut a droite ouvre/ferme la sidebar dans un style mobile app.
- Les icones `menu.svg` et `close.svg` sont des assets SVG locaux du client.
- Un fond overlay ferme la sidebar au clic, et une navigation referme automatiquement le panneau.
- Metro force l'ecriture filesystem de NativeWind pour eviter le watcher fast-refresh CSS incompatible avec la forme d'evenement attendue par Metro.
- `AppNavigation` affiche uniquement les entrees autorisees par le role applicatif courant.
- Les regles `moderator/admin` sont centralisees dans `src/shared/auth/roles.ts`.
- La vue `Profil` reutilise la carte de session et expose le profil applicatif charge depuis `/api/me`.
- La vue `Tests` reutilise les composants existants de test-run sans ouvrir le menu aux roles `listener`.
- La vue `Admin` est un espace minimal reserve aux roles `admin`, pret pour les prochains tickets.

## Criteres d'acceptation

- Le side menu permet de naviguer entre `Live`, `Profil`, `Tests` et `Admin`.
- La page `Profil` est visible pour tout utilisateur authentifie.
- La page `Tests` est visible et accessible seulement pour `moderator` et `admin`.
- La page `Admin` est visible et accessible seulement pour `admin`.
- Un `listener` ne voit pas les entrees `Tests` et `Admin`.
- Un acces direct a une vue interdite affiche un etat non autorise ou redirige vers une page autorisee.
- Les composants `test-runs` existants continuent d'utiliser les protections backend existantes.
- Aucun scroll horizontal n'est reintroduit sur mobile.
- `npm run typecheck` passe.
- `npm run build:web` passe.

## Tests attendus

- Smoke role `listener`: menu sans `Tests` ni `Admin`.
- Smoke role `moderator`: menu avec `Tests`, sans `Admin`.
- Smoke role `admin`: menu avec `Tests` et `Admin`.
- Smoke responsive: `320`, `375`, `414`, `768`, `1024` px sans overflow horizontal.
- Verification que `/ws/test-runs` reste refuse si le backend ne confirme pas le role suffisant.

## Validation locale

- `npm run typecheck`
- `npm run build:web`
- Preview statique du build `dist` sur `http://127.0.0.1:4173/`, avec `GET /` en HTTP 200.
- `npm run web -- --port 19007 --non-interactive`, serveur Expo stable jusqu'a l'arret manuel.

## Dependances

- `EPIC-013`: architecture frontend feature-based.
- `EPIC-014`: session Supabase et profil courant charge dans le client.
- `EPIC-017`: source de verite cible des roles dans `public.user_profiles`.
