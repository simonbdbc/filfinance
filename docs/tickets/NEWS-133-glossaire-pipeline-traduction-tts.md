# NEWS-133 - Glossaire pipeline traduction et TTS

## Objectif

Brancher le glossaire metier dans le pipeline `EN -> FR -> vocal` sans melanger texte affiche et texte lu par Piper.

Le besoin central:

```text
ECB -> BCE -> Banque Centrale Europeenne
```

`BCE` doit pouvoir etre affiche dans le client, pendant que Piper recoit `Banque Centrale Europeenne`.

## Priorite

Priorite 1.

Ce ticket depend de `NEWS-132`.

## Principe pipeline

Le glossaire agit en trois temps:

1. avant OPUS-MT: proteger les termes source par placeholders pour eviter que le modele les reformule ou les supprime;
2. apres OPUS-MT: restaurer ou imposer le terme FR affiche;
3. avant TTS: remplacer le terme FR affiche par sa forme vocale.

Exemple:

```text
Source EN: ECB signals no rush to cut rates
Avant OPUS-MT: __GLOSSARY_1__ signals no rush to cut rates
Apres OPUS-MT: BCE ne signale aucune urgence a reduire les taux
Avant TTS: Banque Centrale Europeenne ne signale aucune urgence a reduire les taux
```

## Scope

- Charger en memoire les entrees actives du glossaire.
- Appliquer les entrees par priorite et longueur decroissante pour eviter les remplacements partiels.
- Proteger les termes EN avant appel OPUS-MT.
- Restaurer les termes FR affiches apres traduction.
- Appliquer `spoken_term` uniquement dans la normalisation TTS.
- Exposer une metrique runtime:
  - nombre d'entrees chargees;
  - derniere erreur de chargement;
  - nombre de hits glossaire sur la fenetre recente.
- Ajouter des fixtures finance:
  - `ECB -> BCE -> Banque Centrale Europeenne`;
  - `Fed -> Fed -> Reserve federale americaine`;
  - `BoE -> BoE -> Banque d'Angleterre`;
  - `NFP -> NFP -> rapport emploi americain`;
  - `EUR/USD -> EUR/USD -> euro dollar`.

## Critere d'acceptation

- Le texte affiche n'est pas pollue par les formes longues vocales.
- Piper recoit la forme vocale quand `spoken_term` existe.
- OPUS-MT ne peut pas casser les tokens metier proteges.
- Les entrees desactivees ne sont pas appliquees.
- Les tests couvrent le matching case-sensitive et case-insensitive.
- Les tests couvrent les collisions, par exemple `ECB` et `ECB officials`.

## Livraison 2026-06-02

- Le worker traduction charge les entrees actives du glossaire avant de construire le draft.
- Les termes source sont remplaces par placeholders avant provider puis restaures en `translated_term`.
- Les formes vocales `spoken_term` sont appliquees uniquement dans le chemin TTS avant Piper.
- `/api/runtime/translation` expose les compteurs glossaire: entrees chargees, hits et derniere erreur.
- Des fixtures couvrent `ECB -> BCE -> Banque Centrale Europeenne` sans polluer `translated_title`.

## Mise a jour 2026-06-12

- Le mode `pattern` supporte maintenant `<string>` en plus de `<number>`.
- Le seed idempotent `ECB's <string>` -> `<string> de la BCE` couvre `ECB's Nagel` -> `Nagel de la BCE`.
- La capture `<string>` laisse la ponctuation terminale hors capture; avec le motif `<string>:` -> `<string> :`, `ECB's Nagel:` devient `Nagel de la BCE :` et `NASDAQ:` devient `NASDAQ :`.
- Les motifs de pur formatage sans litteral metier, comme `<string>:`, comptent pour `Update glossaire` mais ne protegent pas le texte source avant traduction; ils ne peuvent donc plus masquer un motif source plus precis de priorite plus basse comme `ECB's <string>`.
- `POST /api/glossary/reapply-translations` permet aux roles moderator/admin de retraduire les news `financial_juice` presentes dans la table retenue J/J-1 qui matchent une regle active, sans broadcast WebSocket live. La selection SQL scanne toute la table utile et reduit le volume aux candidates, puis le pipeline Rust valide les matches exacts pour conserver les modes `word_boundary` et `pattern`. Le retraitement est idempotent: il calcule les libelles FR attendus depuis la source, repare d'abord les traductions existantes (`BCE's Nagel` ou `Nagel: de la BCE` -> `Nagel de la BCE`), saute OPUS si les libelles attendus sont deja presents, et n'appelle OPUS-MT que pour les lignes encore non satisfaites. Le retraitement ne depend pas de `translation.updated_at`, afin de reparer les lignes deja traduites quand le code du glossaire change sans modification manuelle des regles. Les motifs de pur formatage mettent a jour les traductions existantes directement et incrementent `display_repaired_total`, sans appel OPUS-MT.

## Hors scope

- CRUD API.
- Page frontend.
- Import/export CSV.
- Apprentissage automatique depuis l'historique.
