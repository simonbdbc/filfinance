# EPIC-011 - Refonte acquisition FinancialJuice

## Objectif

Remplacer le chemin RSS primaire par le meme chemin que le site connecte `FinancialJuice`: login HTTP sur `/home`, bootstrap JS, backlog via `FJService.asmx/GetPreviousNews` et live via SignalR 2.x.

## Valeur livree

Le service ne depend plus du flux RSS rate-limited pour remonter les news `FinancialJuice` en temps reel. Il suit maintenant le meme canal que l'application web officielle, avec une auth de session explicite, un backlog startup `J/J-1` et un live SignalR.

## Statut

En cours avance au 6 mai 2026.

## Etat reel au 6 mai 2026

Ce qui est deja livre:

- configuration `financial_juice` site auth
- validation de config avec obligation `NEWS_FINANCIAL_JUICE_EMAIL` et `NEWS_FINANCIAL_JUICE_PASSWORD` si le site est actif
- bootstrap `/home` avec extraction de `__VIEWSTATE`, `__VIEWSTATEGENERATOR`, `MainURLData`, `info`, `feedtoken` et `FeedIds`
- login WebForms HTTP et cache de session cote collecteur
- backlog `GetPreviousNews` avec pagination startup `J/J-1`
- live SignalR 2.x via Azure SignalR (`RedirectUrl` + `AccessToken`) avec `ftoken` propage jusqu'au transport
- parsing des frames `NewsHub.sendUpdates` et reinjection dans le pipeline canonique
- RSS historique supprime du backend; il n'existe plus de fallback RSS ni de variables d'environnement dediees a cet ancien flux.
- ancien flux calendrier legacy retire; le calendrier produit vit dans `calendar_day_*` via n8n
- couverture de tests unitaires sur parsing `/home`, payload `.asmx` et normalisation news

Ce qui reste a consolider:

- validation longue duree sur un service lance en continu avec vraies news live
- documentation d'exploitation et scenario de repli operatoire si l'auth FinancialJuice change

## Definition of Done

- le chemin primaire `FinancialJuice` passe par session web auth + API interne + SignalR
- le RSS n'est plus present dans le backend
- le bootstrap startup recharge `J` et `J-1` puis s'arrete des que la fenetre est depassee
- le live SignalR passe dans le meme pipeline canonique que Telegram, traduction, TTS et `/ws/news`
- le calendrier est acquis, stocke et expose separement, sans polluer le pipeline news
- les tests couvrent le parsing et les points de contrat critiques

## Tickets lies

- `NEWS-051` Bootstrap session `/home` et login WebForms
- `NEWS-052` Backlog `GetPreviousNews` et pagination startup `J/J-1`
- `NEWS-053` Live SignalR 2.x avec `feedtoken`
- `NEWS-054` RSS legacy supprime apres stabilisation du chemin auth
- `NEWS-055` Flux calendrier legacy retire
- `NEWS-056` Fixtures, tests et documentation d'exploitation
