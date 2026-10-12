# NEWS-207 - Documentation couts, rollback et promotion Bunny

## Objectif

Documenter les couts Bunny, le rollback HLS dev/prod et la promotion production vers
`https://media.filfinance.fr`.

## Scope

- Mettre a jour `docs/ENVIRONMENT-FILES.md`.
- Mettre a jour le runbook dev ou GitLab pertinent.
- Documenter `FILFINANCE_DEV_HLS_BASE_URL`.
- Documenter `FILFINANCE_PROD_HLS_BASE_URL`.
- Documenter `EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL=https://media.filfinance.fr` sur
  `main`.
- Documenter `FILFINANCE_MEDIA_LOAD_TEST_ALLOWED_HOSTS`.
- Documenter le rollback:
  - vider `FILFINANCE_DEV_HLS_BASE_URL`;
  - vider `FILFINANCE_PROD_HLS_BASE_URL`;
  - vider `EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL` cote dev ou prod si necessaire;
  - repasser sur les URLs HLS directes backend;
  - ne pas reactiver `/hls/audio/...`.
- Documenter l'ordre de grandeur de couts dev et le minimum mensuel Bunny.
- Documenter la Pull Zone prod `filfinance-prod-hls`, le CNAME DNS-only
  `media.filfinance.fr` et les prerequis de go/no-go.

## Hors scope

- Choix contractuel long terme.
- Automatisation complete du provisioning Bunny.

## Approche technique

- Garder les docs production explicites: Bunny prod passe par `media.filfinance.fr`,
  pas par `live.filfinance.fr`.
- Separater les variables dev, publiques client et variables de scripts de charge.
- Indiquer que tout palier de charge prod doit rester progressif avec monitoring Bunny,
  API health, `/api/admin/health` et logs slow requests.
- Referencer les sources Bunny utilisees pour les couts et les APIs.

## Criteres d'acceptation

- Les docs expliquent comment configurer Bunny en dev.
- Les docs expliquent comment configurer Bunny en production.
- Les docs expliquent comment rollback sans Cloudflare HLS.
- Les docs indiquent que `FILFINANCE_ALLOW_CLOUDFLARE_MEDIA_LOAD_TEST` ne sert pas pour
  Bunny.
- Les couts mentionnent le minimum mensuel Bunny et le prix CDN Europe/Amerique du Nord.
- La production pointe explicitement `EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL` vers
  `https://media.filfinance.fr`.
- Les sources Bunny officielles sont referencees.

## Tests attendus

- `docs:check` passe.
- Recherche documentaire:
  - aucune recommandation prod ne pointe `EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL` vers
    `https://live.filfinance.fr`;
  - aucune recommandation ne reactive `/hls/audio/...`;
  - les variables `FILFINANCE_DEV_HLS_BASE_URL`, `FILFINANCE_PROD_HLS_BASE_URL` et
    `FILFINANCE_MEDIA_LOAD_TEST_ALLOWED_HOSTS` sont documentees.

## Dependances

- `EPIC-035`
- `NEWS-203`
- `NEWS-204`
- `NEWS-205`
- `NEWS-206`
