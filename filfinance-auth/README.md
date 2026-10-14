# FilFinance Auth

`filfinance-auth` est le service Node.js/TypeScript qui porte la nouvelle cible
authentification FilFinance:

- Better Auth expose `/api/auth/*`;
- PostgreSQL conserve les schemas `auth` et `app`;
- Expo/PWA utilise le plugin Better Auth Expo avec SecureStore;
- le backend Rust reste le data plane interne news/audio et verifie les JWT courts emis par Better Auth.

## Variables

```env
NODE_ENV=production
PORT=3000
BETTER_AUTH_URL=https://api.filfinance.fr
BETTER_AUTH_SECRET=replace-with-openssl-rand-base64-32
BETTER_AUTH_JWT_AUDIENCE=https://api.filfinance.fr
BETTER_AUTH_REQUIRE_EMAIL_VERIFICATION=true
BETTER_AUTH_TRUSTED_ORIGINS=filfinance://,https://app.filfinance.fr
DATABASE_URL=postgresql://filfinance_app:password@filfinance-postgres:5432/filfinance
AUTH_DATABASE_URL=postgresql://filfinance_auth:password@filfinance-postgres:5432/filfinance
APP_WEB_URL=https://app.filfinance.fr
EXPO_SCHEME=filfinance
ADMIN_EMAIL_TO_PROMOTE=
SMTP_HOST=smtp-relay.brevo.com
SMTP_PORT=587
SMTP_SECURE=false
SMTP_REQUIRE_TLS=true
SMTP_USER=replace-with-brevo-smtp-user
SMTP_PASSWORD=replace-with-brevo-smtp-key
SMTP_FROM_EMAIL=noreply@filfinance.fr
SMTP_FROM_NAME=FilFinance
```

## Commandes

Depuis `filfinance-auth/`, avec Node >=24.13.1 et une configuration privée
pointant exclusivement sur une base jetable pour la recette :

```bash
npm ci
npm test
npm run check
npm run auth:generate
npm run db:migrate
npm start
```

Ne pas exécuter les migrations ou la génération sur une instance de production
pour cette recette. `npm start` exige le build réalisé par `npm run check` ;
arrêter le processus avec Ctrl+C conserve les données PostgreSQL. Les valeurs
`replace-with-*` et les mots de passe d’exemple doivent être remplacés par des
valeurs neuves propres à l’environnement isolé.

`auth:generate` regenere le SQL Better Auth dans `db/auth-migrations`. Le runtime
applique les migrations versionnees `auth` et `app` sous verrous PostgreSQL. Avec
`AUTH_DATABASE_URL` et `DATABASE_URL` pointant vers des bases de test, le smoke
local complet se lance avec `bash scripts/run_integration_smoke.sh`; il orchestre
le SMTP de capture et le service, valide activation puis reset mono-usage avec
revocation de session, et supprime les fichiers temporaires.

Les mots de passe acceptent entre 10 et 128 caracteres. Les liens de verification
reviennent sur `${APP_WEB_URL}/email-verification`; `APP_WEB_URL` doit donc etre une
origine HTTPS presente dans `BETTER_AUTH_TRUSTED_ORIGINS`.

En production, l'envoi Better Auth utilise uniquement Brevo en STARTTLS sur le
port 587. `SMTP_USER` et `SMTP_PASSWORD` ne doivent exister que dans le fichier
serveur privé ; leur exposition via les préfixes d’environnement publics reste interdite.

L'inscription demande explicitement l'envoi du lien apres creation et n'annonce le
succes qu'apres acceptation SMTP. Les callbacks d'activation et de reinitialisation
sont toujours reecrits vers la PWA HTTPS. Ils ne transportent jamais de cookie de
session ou de token de reset dans le custom scheme `filfinance://`.

En dev, `APP_WEB_URL` et la seconde origine de
`BETTER_AUTH_TRUSTED_ORIGINS` deviennent tous deux l'URL PWA dev exacte. Chaque
backend autorise uniquement sa propre PWA; ne pas reunir les origines prod et dev
dans une meme configuration.


Les scripts de production et préflights distants restent dans GitLab privé.
