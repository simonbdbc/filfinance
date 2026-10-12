# NEWS-178 - Smokes et rollback production finale

> **Archive historique.** Les smokes actifs couvrent maintenant landing, PWA,
> Better Auth et les trois roles; voir [le runbook GitLab](../GITLAB-RUNBOOK.md)
> et [le runbook Better Auth](../AUTH-RUNBOOK.md).

## Objectif

Adapter les smokes de deploiement au modele production final:

- frontend: `https://filfinance.fr`;
- API: `https://api.filfinance.fr`;
- edge: `https://live.filfinance.fr`;
- scope frontend-only utilisable plus tard par `EPIC-029`.

## Statut

Demarre cote repo le 21 mai 2026.

Le script `scripts/smoke_deploy.sh` cible maintenant les domaines production par
defaut. Les anciennes URLs provisoires doivent etre passees explicitement si une
validation historique est encore necessaire.

## Utilisation

Smoke production complet:

```bash
bash scripts/smoke_deploy.sh full
```

Equivalent avec variables explicites:

```bash
FILFINANCE_FRONTEND_URL="https://filfinance.fr" \
FILFINANCE_API_URL="https://api.filfinance.fr" \
FILFINANCE_EDGE_URL="https://live.filfinance.fr" \
FILFINANCE_CHECK_EDGE=1 \
bash scripts/smoke_deploy.sh full
```

Smoke authentifie:

```bash
FILFINANCE_AUTH_TOKEN="<supabase-access-token>" \
FILFINANCE_CHECK_EDGE=1 \
bash scripts/smoke_deploy.sh full
```

Smoke frontend-only:

```bash
bash scripts/smoke_deploy.sh frontend
```

Ce mode verifie uniquement:

- `/`;
- `/profile`;
- presence des assets Expo dans le HTML.

## Rollback production

Frontend:

1. redeployer le dernier build stable dans le projet Pages production;
2. retirer temporairement `filfinance.fr` du projet Pages seulement si TLS/DNS bloque;
3. ne pas utiliser `fil-finance.pages.dev` comme fallback produit apres `EPIC-028`.

Backend:

```bash
cd /home/traefik/filfinance-docker
git log --oneline -10
git checkout <commit-stable>
docker compose --env-file .env.server -f compose.yaml -f compose.prod.yaml up -d --build --force-recreate
```

Edge:

1. redeployer la version Worker stable depuis Cloudflare ou GitLab;
2. verifier `https://live.filfinance.fr/health`;
3. verifier que `FILFINANCE_HLS_PROXY_ENABLED=false` reste configure;
4. verifier qu'une URL `/hls/audio/...` retourne `410 Gone`.

## Criteres d'acceptation

- le smoke public passe sur les domaines finaux;
- le smoke authentifie passe avec un token Supabase valide;
- le smoke edge peut etre active avec `FILFINANCE_CHECK_EDGE=1`;
- le mode `frontend` existe pour le futur environnement dev public;
- les rollbacks frontend/backend/edge sont documentes.

## Dependances

- [EPIC-028](../epics/EPIC-028-migration-production-filfinance-fr.md)
- [NEWS-177](./NEWS-177-pipeline-gitlab-production-main.md)
