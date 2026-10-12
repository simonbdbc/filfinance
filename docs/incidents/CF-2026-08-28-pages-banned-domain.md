# Incident Cloudflare Pages - `banned domain`

## Objet et perimetre

Ce dossier prepare une demande de revue manuelle de la politique anti-abus qui
bloque l'association normale des domaines PWA de la topologie Pages approuvee :
`app.filfinance.fr` vers `filfinance-app-prod` et
`app-dev.filfinance.fr` vers `filfinance-app-dev`. Il ne constitue ni une
autorisation d'envoi, ni une raison de contourner la protection. L'operateur doit
confirmer explicitement l'envoi au moment de contacter Cloudflare.

Le 31 aout 2026, apres confirmation de l'operateur, les canaux officiels ont
ete verifies : `Blocked Content` ne presente aucun blocage actif ou en attente,
et le portail Support du plan Free n'accepte pas de dossier technique Pages.
Le formulaire public de signalement d'abus ne convient pas : il creerait une
plainte contre un site puis la transmettrait a son hebergeur/proprietaire. Aucun
dossier de revue n'a donc ete soumis et aucun faux signalement n'a ete cree.

Ne jamais ajouter a ce document un token API, une cle globale, un secret GitLab,
un cookie, un mot de passe, une cle SSH ou une capture affichant leur valeur.

## Topologie cible et garde-fous

| Surface | Projet Pages | Branche de production | Domaine |
| --- | --- | --- | --- |
| Landing PROD | `filfinance-prod` | `main` | `filfinance.fr` |
| PWA PROD | `filfinance-app-prod` | `main` | `app.filfinance.fr` |
| Landing DEV | `filfinance-dev` | `dev` | `dev.filfinance.fr` |
| PWA DEV | `filfinance-app-dev` | `dev` | `app-dev.filfinance.fr` |

GitLab publie par Direct Upload. `filfinance-prod` conserve son lien Git
historique avec les deploys automatiques desactives; les trois autres projets
n'ont pas d'integration Git. DEV reste entierement en pause : aucun deploiement,
DNS, backend, Worker ou smoke DEV n'est autorise.

Les projets legacy `fil-finance` et `filfinance-pwa` ne sont pas des cibles de
repli. Ils restent conserves jusqu'a l'activation des quatre domaines, aux
deploiements verifies et aux smokes explicitement autorises; ils pourront alors
etre archives puis supprimes. `filfinance-prod` ne doit jamais etre supprime.

## Preuves historiques au 28 aout 2026

- La zone `filfinance.fr` et les projets Pages concernes sont administres dans
  le meme compte Cloudflare.
- Le projet historique `filfinance-pwa` est en Direct Upload : la section
  `Git repository` propose encore l'action `Connect`. Sa branche de production
  est `main`.
- Son hostname historique `pwa-dev.filfinance.fr` affiche exactement :

  > The hostname is part of a banned domain. This web property cannot be added
  > to Cloudflare at this time. If you are an Enterprise customer, please
  > contact your Customer Success Manager. Otherwise, please email
  > abusereply@cloudflare.com with the name of the web property and a detailed
  > explanation of your association with this web property.

- L'ancien hostname `app-dev.filfinance.fr`, attache au projet historique
  `fil-finance`, affiche le meme refus. Ces deux observations sont conservees
  comme indices de classement commun; elles ne changent pas les projets cibles
  ni le hostname PWA DEV canonique.
- Sur le projet `filfinance-prod`, `dev.filfinance.fr` et `filfinance.fr` sont
  `Active` avec `SSL enabled`. Cela prouve qu'une autre propriete de la meme
  zone est deja acceptee par Pages.
- Le pipeline GitLab
  [`2799510106`](https://gitlab.com/simonbdbc/filfinance/-/pipelines/2799510106)
  du SHA `116bf5a3b557dba0b60149e6256af93588b3bd8c` a passe les dix
  validations. Le job
  [`release:dev-lock` `16168017679`](https://gitlab.com/simonbdbc/filfinance/-/jobs/16168017679)
  a ensuite echoue dans `check_cloudflare_release_preflight.sh`, avant tout
  deploiement, car le projet n'expose pas le domaine cible accepte attendu.
- Aucun renommage, hostname alternatif, cookie, proxy, rotation d'IP,
  suppression/recreation de projet ou autre contournement n'a ete tente.

Chronologie UTC utile a la revue :

| Horodatage | Preuve |
| --- | --- |
| `2026-08-28T09:59:04Z` | Premier refus reproductible, pipeline `2799273585`, job `16166355951`. |
| `2026-08-28T11:27:33Z` | Meme refus sur le SHA final documente, pipeline `2799510106`, job `16168017679`. |
| `2026-08-28` | Lecture passive du dashboard : les deux hostnames restent `banned domain`, tandis que `dev.filfinance.fr` reste actif avec SSL. |

## Message de suivi prepare (non envoye)

Le repondeur de `abusereply@cloudflare.com` precise qu'il ne recueille pas de
nouveaux signalements. Il peut seulement servir au suivi d'un dossier Trust &
Safety existant avec sa reference. Le dashboard expose un rapport historique du
18 juin 2026, dont les deux mitigations sont deja `Removed`, mais il ne propose
aucune action `Request review` pour la restriction Pages actuelle. Ne pas
envoyer le texte ci-dessous comme un nouveau signalement tant que Cloudflare ne
fournit pas un canal de suivi associe a cette restriction.

Objet :

```text
Request for review of Pages "banned domain" false positive for app.filfinance.fr and app-dev.filfinance.fr
```

Corps :

```text
Hello Cloudflare Abuse Review team,

I administer the Cloudflare zone filfinance.fr and the Pages projects for the
Fil Finance application in the same Cloudflare account.

Our approved Pages topology is:
- filfinance-prod (main) -> filfinance.fr
- filfinance-app-prod (main) -> app.filfinance.fr
- filfinance-dev (dev) -> dev.filfinance.fr
- filfinance-app-dev (dev) -> app-dev.filfinance.fr

When we attach the PWA custom domains app.filfinance.fr and
app-dev.filfinance.fr to their respective Pages projects, the Pages dashboard
reports:
"The hostname is part of a banned domain. This web property cannot be added to
Cloudflare at this time."

Historical attempts for pwa-dev.filfinance.fr on filfinance-pwa and
app-dev.filfinance.fr on the historical fil-finance project show the same
message. By contrast, dev.filfinance.fr and filfinance.fr are already active
with SSL on filfinance-prod in this same account and zone.

The two PWA domains are first-party application surfaces. app-dev.filfinance.fr
is required for the development PWA to use Better Auth securely with
api-dev.filfinance.fr. GitLab is the only deployment publisher: the new Pages
projects use Direct Upload with no connected Git repository or automatic Git
deployments. The domains are not intended for resale, URL shortening,
user-created subdomains, or redirect traffic.

Please manually review this classification and allow app.filfinance.fr to be
associated with filfinance-app-prod and app-dev.filfinance.fr to be associated
with filfinance-app-dev. We will not try alternate hostnames or bypass the
restriction, and we will complete the normal Pages domain and DNS validation
after your review.

I can provide dashboard screenshots showing the error, the active domains in
the same zone, the project settings, and both failing release preflights at
2026-08-28T09:59:04Z and 2026-08-28T11:27:33Z.

Thank you,
[account owner name]
```

Remplacer seulement la signature avant l'envoi. Envoyer depuis l'adresse liee
au compte Cloudflare. Ne pas copier l'identifiant de compte dans un canal public;
le fournir uniquement dans un dossier prive Cloudflare s'il est demande.

## Pieces a joindre

Captures a produire juste avant l'envoi, en masquant l'adresse du compte si le
canal n'est pas prive :

1. `filfinance-app-prod` > `Custom domains`, avec `app.filfinance.fr` et le
   message complet;
2. `filfinance-app-dev` > `Custom domains`, avec `app-dev.filfinance.fr` et le
   meme message;
3. les captures historiques `filfinance-pwa` / `pwa-dev.filfinance.fr` et
   `fil-finance` / `app-dev.filfinance.fr`, comme correlation uniquement;
4. `filfinance-prod` > `Custom domains`, avec `filfinance.fr` actif et SSL
   active;
5. les reglages Direct Upload sans Git integration des trois nouveaux projets,
   sans valeurs de secrets;
6. le job GitLab `16168017679`, sans variables ni secrets.

## Apres une levee explicite par Cloudflare

Ne modifier aucun DNS tant que Cloudflare n'a pas confirme la revue. Apres une
levee explicite, proceder d'abord a la seule bascule PROD; DEV reste en pause
jusqu'a une decision distincte. Ensuite :

1. relancer une seule fois la validation du domaine existant via l'operation
   Pages `PATCH` officielle, sans supprimer puis recreer le domaine;
2. interroger le domaine jusqu'a obtenir `status=active`,
   `validation_data.status=active` et `verification_data.status=active`;
3. deriver la cible depuis `result.subdomain`, puis exiger exactement un CNAME
   proxifie `app.filfinance.fr -> filfinance-app-prod.pages.dev`; arreter pour
   revue manuelle en cas de doublon ou de type DNS concurrent;
4. relancer la preflight PROD, publier la PWA PROD puis executer les smokes
   autorises;
5. ne basculer `dev.filfinance.fr` et `app-dev.filfinance.fr` qu'apres une
   decision explicite de reprise DEV; leurs CNAME attendus seront
   `filfinance-dev.pages.dev` et `filfinance-app-dev.pages.dev`;
6. supprimer les projets legacy seulement apres les quatre domaines actifs et
   verifies, les deploiements valides et les smokes autorises.

Cloudflare exige que l'association du custom domain soit faite par Pages avant
le CNAME; un CNAME cree seul peut aboutir a une erreur `522`. References
officielles :

- [Custom domains Cloudflare Pages](https://developers.cloudflare.com/pages/configuration/custom-domains/)
- [API Pages Domains](https://developers.cloudflare.com/api/resources/pages/subresources/projects/subresources/domains/)
- [Contacter le support Cloudflare](https://developers.cloudflare.com/support/contacting-cloudflare-support/)
