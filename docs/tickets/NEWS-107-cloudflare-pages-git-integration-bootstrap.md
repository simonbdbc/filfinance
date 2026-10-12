# NEWS-107 - Cloudflare Pages frontend Git integration

> **Archive historique.** La Git Integration Cloudflare est desormais interdite
> pour eviter une publication parallele. Les deux frontends utilisent Direct
> Upload depuis GitLab; voir [le runbook actuel](../GITLAB-RUNBOOK.md).

## Objectif

Mettre en place le deploiement Cloudflare Pages de `filfinance-client` avec le process le plus simple pour un developpement solo:

- push direct sur GitLab;
- build automatique Cloudflare Pages via Git integration;
- production automatique sur `main`;
- previews automatiques sur branches temporaires si elles sont utilisees.

## Statut

Livre cote repo le 2 mai 2026.

Validation live du domaine provisoire Cloudflare Pages effectuee le 2 mai 2026 sur `https://fil-finance.pages.dev/`.

Le domaine custom `filfinance.sbcenter.fr` n'est plus a configurer. La cible de production finale sera `filfinance.fr`.

## Decision

Fil Finance utilise Cloudflare Pages Git integration pour le frontend.

Raison: le projet est developpe par une seule personne, sans merge request obligatoire. La Git integration evite de gerer tout de suite les tokens Cloudflare et les jobs Wrangler dans GitLab.

Le process plus solide reste la cible de production finale:

- GitLab comme pipeline maitre;
- deploiements Pages via Wrangler Direct Upload;
- Workers/Durable Objects deployes depuis GitLab;
- smoke tests post-deploiement;
- rollback documente.

Cette cible sera reprise quand le passage production final demandera plus de controle.

## Scope livre

- pipeline GitLab qui valide le frontend et le backend;
- workflow CI declenche sur branches et tags, sans dependance aux merge requests;
- documentation du setup Cloudflare Pages Git integration;
- documentation des validations preview, production et rollback;
- fallback Cloudflare Pages `_redirects` pour servir l'app Web sur les routes profondes;
- exclusion TypeScript des dossiers d'export Web pour que `npm run typecheck` reste stable apres un `npm run build:web` local.

## Hors scope

- creation effective du projet Cloudflare Pages dans le dashboard;
- configuration effective du domaine custom;
- deploiement Workers/Durable Objects;
- migration vers Wrangler Direct Upload;
- smoke tests automatises post-deploiement.

## Configuration Cloudflare Pages

Creer un projet **Cloudflare Pages**, pas un projet Worker, connecte au repository GitLab `simonbdbc/filfinance`.

Configuration build:

- production branch: `main`
- root directory: `filfinance-client`
- build command: `npm run build:web`
- build output directory: `dist`
- Node.js version: `24.13.1`
- deploy command: laisser vide
- non-production branch deploy command: laisser vide

Variable build Cloudflare Pages:

- `NODE_VERSION=24.13.1`

Variables Pages requises, en preview et production:

- `EXPO_PUBLIC_FILFINANCE_WS_URL`
- `EXPO_PUBLIC_SUPABASE_URL`
- `EXPO_PUBLIC_SUPABASE_PUBLISHABLE_KEY`

`EXPO_PUBLIC_SUPABASE_ANON_KEY` reste supporte par compatibilite client, mais la convention recommandee pour Fil Finance est `EXPO_PUBLIC_SUPABASE_PUBLISHABLE_KEY`.

Ces variables sont publiques dans le bundle Expo Web. Elles ne doivent jamais contenir de secret serveur.

Point important: ne pas configurer `npx wrangler deploy` comme deploy command pour ce projet frontend. La Git integration Pages publie automatiquement le contenu du dossier `dist` apres le build. `wrangler deploy` declenche un deploiement Worker/Static Assets et rejette le fallback SPA `public/_redirects`.

## Diagnostic build Cloudflare

Si le log contient:

```text
Executing user deploy command: npx wrangler deploy
Invalid _redirects configuration:
Line 1: Infinite loop detected in this rule
```

Alors Cloudflare n'est pas en mode Pages Git integration simple. Corriger le projet Cloudflare:

1. supprimer le deploy command `npx wrangler deploy`;
2. supprimer le non-production deploy command s'il existe;
3. verifier que le type de projet est bien Pages, pas Worker;
4. garder uniquement:
   - root directory: `filfinance-client`
   - build command: `npm run build:web`
   - output directory: `dist`

Ne pas supprimer `filfinance-client/public/_redirects`: cette regle est le fallback SPA attendu par Cloudflare Pages.

## Workflow de developpement solo

Process nominal:

```bash
git status --short
git add .
git commit -m "type(scope): message"
git push
```

Apres push sur `main`:

- GitLab CI lance les checks docs, Rust et client;
- Cloudflare Pages lance son build Git integration;
- Cloudflare publie la production Pages si le build reussit.

Les branches temporaires restent possibles pour obtenir une preview Cloudflare, mais elles ne sont pas obligatoires.

## Preview par branche

Utiliser une branche temporaire seulement quand une preview isolee est utile.

Procedure:

1. creer une branche courte, par exemple `preview/news-107-pages`;
2. pousser la branche sur GitLab;
3. verifier que Cloudflare Pages declenche un deploiement preview;
4. verifier que les variables preview sont distinctes de la production si necessaire;
5. noter dans le ticket l'URL preview, le commit SHA et le statut build.

Note operationnelle: ne pas utiliser le bouton `Retry` d'un ancien deploiement pour valider un nouveau commit. Pousser un nouveau SHA quand le contenu change.

## Production sur `main`

Procedure:

1. valider localement les changements sensibles;
2. commit puis push sur `main`;
3. verifier le pipeline GitLab;
4. verifier le build Cloudflare Pages;
5. executer les smoke tests rapides.

Smoke tests minimaux:

- `GET /` retourne 200;
- le HTML charge les assets Expo depuis `/_expo/static/...`;
- une route profonde retourne bien l'app Web grace a `public/_redirects`;
- l'ecran auth charge sans erreur de configuration Supabase;
- une indisponibilite temporaire du backend live ne casse pas le chargement de l'app statique.

## Domaine custom et HTTPS

Domaine frontend cible:

- domaine provisoire Cloudflare Pages: `https://fil-finance.pages.dev/`
- production finale: `filfinance.fr`

Procedure cible:

1. garder l'URL `pages.dev` comme domaine de travail;
2. ne pas configurer `filfinance.sbcenter.fr`;
3. suivre [PRODUCTION-FINAL-RUNBOOK](../PRODUCTION-FINAL-RUNBOOK.md) pour basculer vers `filfinance.fr`.

## Rollback

Procedure simple:

1. identifier le dernier deploiement stable dans Cloudflare Pages;
2. redeployer ce build depuis Cloudflare;
3. verifier les smoke tests rapides;
4. ouvrir un ticket d'incident si la cause racine n'est pas immediate.

Pour un correctif rapide, pousser ensuite un nouveau commit sur `main`. `Retry` relance le meme commit; il ne valide pas un nouveau changement.

## Validation locale

Commandes executees le 2 mai 2026:

```bash
cd filfinance-client
npm run typecheck
npm run build:web
npm run typecheck
```

Resultat:

- `npm run typecheck`: OK avec Node `24.13.1`
- `npm run build:web`: OK avec Node `24.13.1`
- `npm run typecheck` apres generation de `dist`: OK

## Dependances

- [EPIC-020 - GitLab, Cloudflare Pages et serveur personnel](../epics/EPIC-020-gitlab-pipeline-cloudflare.md)
- [NEWS-106 - Pipeline GitLab CI de validation](./NEWS-106-pipeline-gitlab-ci-validation.md)
