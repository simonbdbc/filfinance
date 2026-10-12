# NEWS-202 - Validation et runbook Trust & Safety HLS

## Objectif

Documenter la preuve de coupure HLS Cloudflare et les elements a transmettre a Cloudflare
Trust & Safety.

## Scope

- Verifier que `/hls/audio/...` retourne `410 Gone` sur le Worker deploye.
- Verifier que le frontend de production n'embarque pas de base HLS `live.filfinance.fr`.
- Archiver les sorties de tests Worker et scripts.
- Preparer le message d'appel Trust & Safety.

## Criteres d'acceptation

- Le runbook prod mentionne la mitigation du 18 juin 2026 et la coupure du data plane HLS.
- Les preuves techniques listent:
  - configuration Worker `FILFINANCE_HLS_PROXY_ENABLED=false`;
  - resultat HTTP `410` sur une URL `/hls/audio/...`;
  - absence de base HLS Cloudflare dans les variables Pages;
  - garde de charge active.
- Le message d'appel indique que les tests HLS Cloudflare sont stoppes et que la diffusion
  media sera deplacee vers une cible conforme.

## Dependances

- `EPIC-034`
- `NEWS-199`
- `NEWS-200`
- `NEWS-201`
