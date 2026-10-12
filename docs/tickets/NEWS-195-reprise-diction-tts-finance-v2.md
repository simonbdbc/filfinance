# NEWS-195 - Reprise diction TTS finance v2

## Objectif

Reprendre la diction TTS finance sans changer la voix Piper de production.

La voix reste:

```env
NEWS_TTS_VOICE=fr_FR-upmc-medium-jessica
```

## Constat

`NEWS-128` a livre une premiere normalisation utile, mais insuffisante: la diction reste trop mecanique sur certains titres macro et le systeme ne separe pas encore assez clairement les termes affiches des formes vocales metier.

## Scope

- Revoir le normaliseur envoye a Piper pour rendre les phrases plus naturelles.
- Ajouter des fixtures de diction finance plus strictes.
- Ameliorer les acronymes macro, banques centrales, horaires, devises, paires FX, points de base, pourcentages, decimales et tickers.
- Brancher le glossaire metier pour appliquer `spoken_term` uniquement au texte parle.
- Ajouter le mode glossaire `pattern`, avec captures `<number>` et `<string>` sans ponctuation terminale, pour traiter les horaires operateur comme `<number>h<number>`, les motifs directionnels comme `ECB's <string>` et l'espace avant deux-points via `<string>:` sans polluer le texte affiche.
- Segmenter les corps longs envoyes a Piper uniquement sur ponctuation (`.`, `,`, `;`, `?`, `!`) puis concatener les WAV PCM avant HLS, afin de reduire les timeouts par segment sans decoupage par espaces ni limite dure. Les points et virgules entre deux chiffres, comme `1.62` ou `5,2`, ne coupent pas le segment. Les tirets ne coupent jamais le segment.
- Garder `translated_title`, `translated_body` et `narration_text` persistants lisibles pour l'utilisateur.

## Critere d'acceptation

- `NEWS_TTS_VOICE` reste `fr_FR-upmc-medium-jessica`.
- Une entree `ECB -> BCE -> Banque Centrale Europeenne` affiche `BCE` mais envoie `Banque Centrale Europeenne` a Piper.
- Une entree motif `<number>h<number>` avec `spoken_term=<number> heure <number>` lit `16h01` comme `seize heure zero un` et `09h30` comme `zero neuf heure trente`.
- Une entree motif `ECB's <string>` avec `translated_term=<string> de la BCE` transforme `ECB's Nagel:` en `Nagel de la BCE:`; combinee avec `<string>:` -> `<string> :`, elle produit `Nagel de la BCE :`.
- Un corps long est synthetise par segments de ponctuation avec le timeout Piper existant par segment; un segment sans ponctuation qui depasse le timeout garde l'erreur TTS actuelle.
- La segmentation conserve `1.62`, `5,2`, `EUR-USD`, `Fed - BCE` et `2026-06-09` dans leurs segments d'origine.
- Les fixtures TTS couvrent `NFP`, `EUR/USD`, `+1.5%`, `US 10Y`, `Fed`, `ECB/BCE` et les tickers.
- Le texte affiche n'est pas pollue par les formes longues vocales.
- Les compteurs runtime traduction exposent les entrees glossaire chargees, les hits et la derniere erreur glossaire.

## Hors scope

- Changement de voix par defaut.
- Fine-tuning Piper.
- Evaluation comparative de nouvelles voix.
- Pools workers traduction/TTS, qui restent portes par `NEWS-130`.
