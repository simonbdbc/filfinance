# NEWS-112 - Route Traefik API Fil Finance

> **Archive historique — ne pas executer.** Cette route, le domaine provisoire,
> le frontend unique et Supabase sont supersedes. Utiliser
> [le runbook GitLab actuel](../GITLAB-RUNBOOK.md) et
> [les fichiers d'environnement actuels](../ENVIRONMENT-FILES.md).

## Objectif

Appliquer dans `home-srv/traefik-docker` la route Traefik necessaire au backend Fil Finance, sans heberger le frontend sur le serveur personnel.

## Statut

Applique le 2 mai 2026.

## Decision

Le frontend reste heberge par Cloudflare Pages:

- domaine frontend provisoire: `https://fil-finance.pages.dev`
- build: `filfinance-client`
- variables publiques Expo dans Cloudflare Pages

Le serveur personnel expose uniquement le backend Rust:

- domaine API/WebSocket: `filfinance-api.sbcenter.fr`
- service Docker Traefik: `filfinance-service@docker`
- autorisation applicative: JWT Supabase, pas Basic Auth Traefik

## Fichier ajoute dans `home-srv`

Chemin:

```text
/Users/simon/Projets/PlateformeDev/home-srv/traefik-docker/volumes/config/conf/filfinance.yaml
```

Contenu:

```yaml
# Traefik dynamic configuration file
# Backend Fil Finance: frontend is hosted by Cloudflare Pages.

http:
  routers:
    filfinance-api:
      rule: "Host(`filfinance-api.sbcenter.fr`)"
      entryPoints:
        - websecure
      tls:
        certResolver: lets_encrypt
        domains:
          - main: "filfinance-api.sbcenter.fr"
      middlewares:
        - default-headers
      service: filfinance-service@docker
```

## Points de vigilance

- Ne pas ajouter `traefik-auth` sur cette route: le client Cloudflare Pages doit pouvoir ouvrir les WebSockets et appeler l'API avec son JWT Supabase.
- Le service `filfinance-service@docker` existera uniquement quand le compose Fil Finance sera lance sur le meme reseau Docker externe `traefik_network`.
- `filfinance-api.sbcenter.fr` doit pointer vers le serveur personnel pour que Let's Encrypt puisse emettre le certificat.

## Validation

Commandes executees:

```bash
cd /Users/simon/Projets/PlateformeDev/home-srv/traefik-docker
ruby -e 'require "yaml"; YAML.load_file("volumes/config/conf/filfinance.yaml"); puts "yaml ok"'
```

Resultat:

- YAML OK
- repo `traefik-docker` propre avant ajout
- nouveau fichier non committe: `volumes/config/conf/filfinance.yaml`

## Suite

- [`NEWS-113`](./NEWS-113-bootstrap-serveur-premier-deploy.md): bootstrap du repo Fil Finance sur le serveur, creation de `.env.server`, premier deploy GitLab.
- Garder Cloudflare Pages sur `https://fil-finance.pages.dev` pour l'environnement courant.
- La cible de production finale sera `filfinance.fr`, documentee dans [PRODUCTION-FINAL-RUNBOOK](../PRODUCTION-FINAL-RUNBOOK.md).
