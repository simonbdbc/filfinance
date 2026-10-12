# Runbook Better Auth - Fil Finance

## Architecture

- `filfinance-auth` expose Better Auth sous `/api/auth/*` et le profil sous `/api/me`.
- PostgreSQL separe les tables Better Auth (`auth`) des donnees metier (`app`).
- Expo/PWA conserve la session Better Auth dans SecureStore via `@better-auth/expo`.
- La verification email revient sur la route publique `/email-verification`; Better Auth ouvre automatiquement une session apres l'activation.
- Le client demande un JWT RS256 court via `/api/auth/token` pour les routes HTTP et WebSocket Rust.
- Rust verifie `kid`, signature RS256, `iss`, `aud`, `exp` et `sub` contre `/api/auth/jwks`.
- Le role `listener`, `moderator` ou `admin` est signe dans ce JWT; les preferences restent dans `app.profiles`.

URLs publiques cible:

| Environnement | Landing | PWA et callback Better Auth | API et Better Auth |
| --- | --- | --- | --- |
| dev | `https://dev.filfinance.fr` | `https://app.dev.filfinance.fr` | `https://api.dev.filfinance.fr` |
| prod | `https://filfinance.fr` | `https://app.filfinance.fr` | `https://api.filfinance.fr` |

Pour le dev Web, utiliser le custom domain same-site canonique
`https://app.dev.filfinance.fr`. Un hôte tiers est cross-site avec
`api.dev.filfinance.fr`; le cookie Better Auth `SameSite=Lax` ne serait alors pas
envoye de facon fiable par les requetes `fetch` du navigateur.

DEV reste en pause. La reprise exige l'association Pages normale de
`app.dev.filfinance.fr` à la Pull Zone Bunny PWA, puis sa validation
Cloudflare; tant que la politique `banned domain` n'est pas levee, ne pas
essayer une variante de hostname ou un CNAME seul.

## Configuration

La liste complete est dans [.env.server.example](../.env.server.example). Les valeurs structurantes de production sont:

```env
BETTER_AUTH_URL=https://api.filfinance.fr
BETTER_AUTH_JWT_AUDIENCE=https://api.filfinance.fr
BETTER_AUTH_REQUIRE_EMAIL_VERIFICATION=true
BETTER_AUTH_TRUSTED_ORIGINS=filfinance://,https://app.filfinance.fr
APP_WEB_URL=https://app.filfinance.fr

NEWS_AUTH_ENABLED=true
NEWS_AUTH_PROVIDER=better_auth
NEWS_AUTH_ISSUER=https://api.filfinance.fr
NEWS_AUTH_JWKS_URL=http://filfinance-auth:3000/api/auth/jwks
NEWS_AUTH_AUDIENCE=https://api.filfinance.fr
NEWS_AUTH_MODERATOR_ROLES=moderator,admin
```

Pour le dev, remplacer la cible API et la PWA dans le fichier dev :

```env
BETTER_AUTH_URL=https://api.dev.filfinance.fr
BETTER_AUTH_JWT_AUDIENCE=https://api.dev.filfinance.fr
BETTER_AUTH_TRUSTED_ORIGINS=filfinance://,https://app.dev.filfinance.fr
APP_WEB_URL=https://app.dev.filfinance.fr
NEWS_AUTH_ISSUER=https://api.dev.filfinance.fr
NEWS_AUTH_AUDIENCE=https://api.dev.filfinance.fr
```

Le preflight refuse les placeholders, les secrets courts, les URLs incoherentes,
les origines Expo de developpement en prod et un SMTP incomplet. La verification
PROD est autonome : elle ne lit aucune URL DEV.

```bash
# PROD actif
bash scripts/check_better_auth_env.sh .env.server.prod prod

# DEV archive : a executer uniquement apres une decision de remise en service.
bash scripts/check_better_auth_env.sh \
  .env.server.dev dev "$FILFINANCE_DEV_PWA_URL"
```

`APP_WEB_URL` doit valoir `https://app.filfinance.fr` en prod et
`https://app.dev.filfinance.fr` en dev (`FILFINANCE_DEV_PWA_URL`).
`BETTER_AUTH_TRUSTED_ORIGINS` contient
`filfinance://` et uniquement la PWA du meme environnement : l'origine prod en
prod, l'origine dev exacte en dev. Ajouter uniquement la landing du meme
environnement si elle utilise un endpoint Better Auth. Le preflight refuse les
origines croisees, une URL dev absente, non HTTPS ou contenant un chemin.

## Inscription et activation

- Le mot de passe accepte entre 10 et 128 caracteres, sans autre regle de complexite.
- L'inscription appelle explicitement l'envoi du lien et n'annonce le succes qu'apres acceptation SMTP.
- Web et mobile transmettent `https://<pwa>/email-verification?status=success`; le serveur remplace tout callback par la PWA HTTPS du meme environnement.
- L'email contient un lien HTML qui conserve le jeton et le callback complets.
- Better Auth confirme l'adresse, cree la session puis redirige vers la page de confirmation.
- Sur iOS et Android, le lien revient temporairement dans la PWA HTTPS. Le retour automatique vers l'app exige encore des Universal Links/App Links verifies et un echange ephemere; aucun cookie ou token sensible ne doit transiter dans `filfinance://`.
- Le travail restant et ses prerequis de signature sont suivis dans [NEWS-216](./tickets/NEWS-216-liens-auth-natifs-verifies.md). Tant que ce ticket n'est pas valide sur appareils, le parcours natif automatique reste un no-go de release mobile.
- Si une connexion est refusee avec `EMAIL_NOT_VERIFIED`, le client demande explicitement un nouveau lien avec le callback adapte a la plateforme.

## Validation locale

```bash
cd filfinance-auth
npm ci
npm test
npm run check
bash scripts/run_integration_smoke.sh
```

Le script suppose PostgreSQL de test accessible via `AUTH_DATABASE_URL` et
`DATABASE_URL`. Il construit le service, lance un SMTP de capture exclusivement
sur loopback, execute le parcours puis supprime le message temporaire. Le smoke
couvre inscription, envoi explicite, activation, restauration de session, JWT,
JWKS, creation du profil, preferences, reset mono-usage, revocation de l'ancienne
session, rejet d'un champ `role` et deconnexion.

## Validation deployee

La gate de release exige trois comptes dedies, deja verifies, avec les roles exacts
`listener`, `moderator` et `admin`. Les six identifiants sont stockes comme variables
GitLab masquees et scopees par environnement:

```text
FILFINANCE_SMOKE_LISTENER_EMAIL
FILFINANCE_SMOKE_LISTENER_PASSWORD
FILFINANCE_SMOKE_MODERATOR_EMAIL
FILFINANCE_SMOKE_MODERATOR_PASSWORD
FILFINANCE_SMOKE_ADMIN_EMAIL
FILFINANCE_SMOKE_ADMIN_PASSWORD
```

Le runner acquiert des JWT temporaires sans les journaliser, puis exige les trois
roles. Pour une execution manuelle, exporter ces variables et lancer:

```bash
FILFINANCE_FRONTEND_URL=https://filfinance.fr \
FILFINANCE_PWA_URL=https://app.filfinance.fr \
FILFINANCE_API_URL=https://api.filfinance.fr \
FILFINANCE_EDGE_URL=https://live.filfinance.fr \
FILFINANCE_CHECK_EDGE=1 \
bash scripts/run_deployed_smoke_suite.sh
```

La suite teste landing/PWA/fallback, session/JWT, API listener, refus listener,
routes moderator, route admin et WebSockets listener/moderator/admin. Les comptes
doivent rester reserves aux smokes; renouveler les JWT apres tout changement de role.

## Diagnostic

`401`:

- verifier que le client a encore une session Better Auth;
- renouveler le JWT via `/api/auth/token`;
- comparer `BETTER_AUTH_URL`, `BETTER_AUTH_JWT_AUDIENCE`, `NEWS_AUTH_ISSUER` et `NEWS_AUTH_AUDIENCE`;
- verifier `/api/auth/jwks` et l'horloge des conteneurs.

`403`:

- verifier le champ `role` dans `auth."user"`;
- renouveler le JWT apres une promotion, car un ancien JWT conserve son ancien role;
- verifier `NEWS_AUTH_MODERATOR_ROLES`.

Email non recu:

- verifier `SMTP_HOST`, `SMTP_PORT`, `SMTP_SECURE`, `SMTP_REQUIRE_TLS`, `SMTP_USER`, `SMTP_PASSWORD`, `SMTP_FROM_EMAIL` et `SMTP_FROM_NAME`; en production, le relais est Brevo sur 587 avec `SMTP_REQUIRE_TLS=true` et `SMTP_SECURE=false` pour interdire tout downgrade STARTTLS;
- lire les logs `filfinance-auth` sans afficher les secrets;
- ne pas desactiver la verification email en production pour contourner un incident SMTP.

La verification affiche du JSON:

- `{"status":true,"user":null}` indique que `/api/auth/verify-email` a ete appele sans callback exploitable;
- verifier que l'email contient le parametre `callbackURL` complet et que `APP_WEB_URL` est correct;
- les liens sans callback ou avec `callbackURL=/` sont rediriges automatiquement vers la page web de confirmation.

Lien expire ou invalide:

- la page `/email-verification` traduit `TOKEN_EXPIRED`, `INVALID_TOKEN`, `USER_NOT_FOUND` et `INVALID_USER`;
- revenir a la connexion avec les bons identifiants pour demander un nouveau lien si l'adresse n'est pas encore verifiee.

## Rotation

- `BETTER_AUTH_SECRET`: rotation planifiee avec invalidation des sessions et nouvelle connexion.
- JWT RS256: Better Auth conserve les cles dans `auth.jwks`; Rust rafraichit le JWKS au TTL ou immediatement sur un `kid` inconnu.
- Mots de passe PostgreSQL/SMTP: modifier le secret serveur, recreer les conteneurs, puis executer les smokes.

## Administration

`ADMIN_EMAIL_TO_PROMOTE` ne promeut qu'un compte existant avec email verifie. Le bootstrap est tente au demarrage puis apres les flux Better Auth jusqu'a la promotion; il est protege par un verrou PostgreSQL et trace dans `app.auth_bootstrap`. Retirer la variable apres confirmation.
