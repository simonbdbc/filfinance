# NEWS-128 - Normalisation TTS finance

## Objectif

Ameliorer la comprehension audio sans modifier le texte affiche.

Le normaliseur doit produire un `narration_text` parle et stable pour Piper, tandis que `translated_title` et `translated_body` restent des textes de lecture visuelle.

## Priorite

Priorite 3.

C'est le meilleur gain qualite/risque cote TTS apres la remise en place OPUS-MT Docker.

## Scope

- Creer ou renforcer un normaliseur dedie au texte parle.
- Traiter les acronymes finance.
- Traiter les horaires.
- Traiter les devises et paires FX.
- Traiter les variations en points de base.
- Traiter les pourcentages.
- Traiter les tickers quand ils sont prononces lettre par lettre.
- Ajouter des tests unitaires Rust sur fixtures finance.

## Exemples attendus

- `25 bps` -> `25 points de base`
- `EUR/USD` -> `euro dollar`
- `14:30` -> `quatorze heures trente`
- `Fed` -> `la Fed`
- `ECB` ou `BCE` -> prononciation explicite si Piper lit mal l'acronyme
- `US 10Y` -> `Treasury américain à 10 ans`

## Livraison 2026-05-03

- Le helper TTS normalise uniquement le texte parle envoye a Piper; les champs visuels `translated_title` et `translated_body` restent inchanges.
- Les horaires `HH:MM`, les pourcentages signes, les paires FX et les devises majeures sont rendus sous forme plus naturelle.
- Les expressions multi-tokens `+25 bps` et `US 10Y` sont traitees avant le traitement token par token.
- Les banques centrales courantes (`Fed`, `ECB`, `BCE`, `BoE`, `BoJ`, `PBoC`) ont une formulation dediee, avec protection contre `la la Fed`.
- Les tickers/sigles restants continuent d'etre epeles lettre par lettre.

## Reprise 2026-06-02

Cette v1 est consideree livree mais insuffisante pour la qualite produit attendue.

La reprise est portee par [`NEWS-195`](./NEWS-195-reprise-diction-tts-finance-v2.md):

- conserver `NEWS_TTS_VOICE=fr_FR-upmc-medium-jessica`;
- rendre la diction moins mecanique sur les fixtures macro;
- brancher le glossaire metier pour separer `FR affiche` et `FR vocal`;
- garder les formes longues vocales hors des champs affiches et persistants.

## Critere d'acceptation

- Le texte affiche reste inchangé.
- Le texte parle est plus naturel sur les fixtures finance.
- Les cas numeriques critiques restent exacts.
- Les tests couvrent les acronymes, devises, horaires, `bps`, pourcentages et tickers.

## Hors scope

- Fine-tuning Piper.
- Changement de voix par defaut.
- Correction generale de la traduction OPUS-MT.
