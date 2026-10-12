# EPIC-025 - Strategie d'acces FinancialJuice

## Objectif

Stabiliser l'acquisition `FinancialJuice` sans melanger des contournements fragiles dans le backend Rust principal.

La strategie est volontairement en deux temps:

1. MVP / finalisation v1: si `FinancialJuice` doit etre consomme comme un vrai navigateur, creer un collector navigateur separe base sur `Playwright`/`Chromium`.
2. Production v2: demander directement a `FinancialJuice` un acces officiel ou autorise pour l'ingestion serveur.

## Constat

Le site `FinancialJuice` ne presente pas d'API publique officielle documentee pour l'acces temps reel `news`/`calendar`.

Le backend Rust peut etre bloque par une `Cloudflare Managed Challenge` sur `/home`, avant meme le login applicatif. Dans ce cas, un simple client HTTP `reqwest` ne peut pas executer la challenge navigateur ni produire un cookie de clearance fiable.

Les solutions de rotation d'IP, de reutilisation automatique de cookies navigateur ou de contournement de challenge ne sont pas retenues comme base produit.

## Phase 1 - MVP v1: collector navigateur isole

Pour continuer le developpement principal et finaliser la v1 de l'app, une option acceptable est de creer un composant separe qui consomme `FinancialJuice` comme un vrai navigateur.

Ce composant doit etre isole du backend Rust principal.

Architecture cible:

```text
filfinance-service/        # Backend Rust principal
  src/features/ingestion/  # Ingestion canonique, repositories, live, audio

financialjuice-browser-collector/  # Composant separe
  playwright/chromium runtime
  session navigateur dediee
  extraction news/calendar
  emission vers une entree backend stable
```

Principes:

- ne pas lire automatiquement les profils Brave/Chrome personnels;
- ne pas voler ni decrypter des cookies utilisateur;
- ne pas faire de rotation d'IP anti-blocage;
- assumer que le composant est un runtime navigateur, pas un client HTTP;
- garder un contrat de sortie simple vers le backend: JSON normalise ou appel HTTP interne;
- pouvoir desactiver ce composant sans casser le backend;
- garder `NEWS_FINANCIAL_JUICE_ENABLED=false` possible pendant les tests locaux non lies a l'acquisition.

Avantages:

- debloque le dev MVP si la source exige une verification navigateur;
- limite la dette dans un composant jetable ou remplacable;
- evite de contaminer le backend Rust feature-first avec `Playwright`, profils navigateur et logique DOM;
- conserve le pipeline canonique Rust pour normalisation, persistence, traduction, audio et diffusion.

Contraintes:

- plus fragile qu'un acces officiel;
- plus lourd en runtime;
- sensible aux changements DOM et aux challenges Cloudflare;
- a surveiller comme source externe degradable;
- ne doit pas devenir une solution de production long terme sans decision explicite.

## Phase 2 - Production v2: acces officiel FinancialJuice

Une fois le developpement principal termine et dans l'optique d'une mise en production v2, contacter `FinancialJuice` pour demander explicitement un acces autorise.

Canal:

- Contact Us: `https://www.financialjuice.com/contactus.aspx`

Demandes a formuler:

- API officielle temps reel `news`/`calendar`;
- flux serveur autorise;
- RSS exploitable legalement cote backend;
- webhook eventuel;
- IP allowlist ou acces partenaire;
- autorisation ecrite pour ingestion backend.

Action immediate: envoyer la demande d'autorisation documentee dans `NEWS-155`.

Message propose:

```text
Subject: Request for authorized FinancialJuice data access

Hello FinancialJuice team,

I am building Fil Finance, a software product that consumes real-time financial news and economic calendar data to provide translated news, metadata, and audio playback to authenticated users.

I would like to avoid any unsupported scraping or Cloudflare challenge workaround. Could you confirm whether FinancialJuice provides an official or authorized integration option for backend/server-side consumption of your news and calendar data?

If one of these options is available, could you share documentation, pricing, usage rights, rate limits, authentication method, and redistribution restrictions?

Best regards,
Simon
Fil Finance
```

## Decision actuelle

- API publique visible: non.
- Acces autorise possible: mail de demande a envoyer.
- Pour la v1, le backend Rust reste propre et defensif: detection de challenge, backoff et source desactivable.
- Si besoin MVP, le collector navigateur est un composant separe et assume, pas une extension interne du backend Rust principal.

## Tickets

- `NEWS-154`: cadrer et prototyper un collector navigateur `FinancialJuice` separe pour le MVP v1.
- `NEWS-155`: demander un acces officiel `FinancialJuice` pour la production v2.
