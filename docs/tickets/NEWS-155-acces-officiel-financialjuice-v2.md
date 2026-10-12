# NEWS-155 - Acces officiel FinancialJuice v2

## Objectif

Preparer une mise en production v2 propre en demandant a `FinancialJuice` un acces officiel, autorise et stable pour l'ingestion serveur `news`/`calendar`.

## Scope

- Contacter `FinancialJuice` via leur page officielle de contact.
- Demander les options disponibles pour une integration backend.
- Clarifier les droits d'usage, les limites et les conditions de redistribution interne.
- Documenter la decision d'architecture apres reponse.

## Hors scope

- Pas d'implementation technique avant retour clair.
- Pas de contournement Cloudflare.
- Pas de scraping supplementaire non autorise.
- Pas de changement de schema SQL dans ce ticket.

## Demandes a formuler

- API officielle temps reel `news`/`calendar`.
- Flux serveur autorise.
- RSS exploitable legalement cote backend.
- Webhook eventuel.
- IP allowlist ou acces partenaire.
- Autorisation ecrite pour ingestion backend.

## Message propose

```text
Subject: Request for authorized FinancialJuice data access

Hello FinancialJuice team,

I am building Fil Finance, a software product that consumes real-time financial news and economic calendar data to provide translated news, metadata, and audio playback to authenticated users.

I would like to avoid any unsupported scraping or Cloudflare challenge workaround. Could you confirm whether FinancialJuice provides an official or authorized integration option for backend/server-side consumption of your news and calendar data?

The options I am looking for are, for example:

- official API access;
- licensed RSS or feed access;
- webhook or streaming endpoint;
- server-to-server access with authentication;
- IP allowlisting or partner access;
- written permission for backend ingestion under your terms.

If one of these options is available, could you share the relevant documentation, pricing, usage rights, rate limits, authentication method, and redistribution restrictions for an authenticated end-user product?

Best regards,
Simon
Fil Finance
```

## Version courte si le formulaire impose peu de caracteres

```text
Hello FinancialJuice team,

I am building Fil Finance and would like to use FinancialJuice news and calendar data through an authorized backend/server-side integration, without unsupported scraping or Cloudflare challenge workarounds.

Do you provide an official API, licensed RSS/feed, webhook, server-to-server access, IP allowlisting, or partner access? If yes, could you share documentation, pricing, usage rights, rate limits, and authentication details?

Best regards,
Simon
```

## Canal

- `https://www.financialjuice.com/contactus.aspx`

## Criteres d'acceptation

- La demande est envoyee.
- La reponse est archivee dans une note interne du projet.
- La roadmap est mise a jour selon la reponse:
  - API officielle disponible;
  - RSS licencie disponible;
  - IP allowlist disponible;
  - partenariat necessaire;
  - aucune option officielle disponible.
- Le choix final remplace ou confirme le collector navigateur MVP.

## Tests attendus

Pas de tests automatises dans ce ticket.

La validation attendue est documentaire:

- URL/contact utilise;
- date d'envoi;
- contenu envoye;
- reponse recue;
- decision produit/architecture.

## Dependances

- `EPIC-025`
- Stabilisation MVP v1
- Decision go/no-go production v2
