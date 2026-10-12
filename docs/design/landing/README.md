# Landing Fil Finance — Référentiel visuel clair et sombre

## Statut et ordre de référence

Spécification retenue le **12 septembre 2026**, pour
[EPIC-038](../../epics/EPIC-038-refonte-visuelle-landing-themes-clair-sombre.md).
La livraison documentaire initiale est suivie de la refonte Astro et des
deux thèmes, implémentés et validés localement le même jour.

Le référentiel, ses liens et l'intégrité des deux copies PNG ont été vérifiés
le 12 septembre 2026 dans NEWS-222. La
recette frontend et ses huit captures (preuve interne conservée dans GitLab, exclue du dépôt de présentation)
clôturent NEWS-223 à NEWS-226. Aucun déploiement n'est attesté.

Les valeurs et comportements écrits ci-dessous font foi. Les maquettes
ImageGen donnent la composition, la hiérarchie et l'ambiance ; leurs couleurs,
textures ou détails de rendu ne remplacent pas les tokens documentés. Ce ne
sont pas des captures d'une application implémentée ou testée.

Les deux références retenues comportent les quatre services et ne comportent
aucune section gamma. Les propositions antérieures ne font pas partie de
cette galerie. La commande de thème prévue ci-dessous n'est pas dessinée
dans les PNG : son ajout est un écart intentionnel.

## Galerie

### Thème clair — Avoine et noisette

![Maquette claire de la landing, avec les quatre services et sans section gamma](./maquettes/landing-clair-v1.png)

### Thème sombre — Bleu nuit et cuivre

![Maquette sombre de la landing, avec les quatre services et sans section gamma](./maquettes/landing-sombre-v1.png)

### Provenance et intégrité

Les fichiers sont copiés sans conversion, redimensionnement, retouche ni
recompression depuis les dernières sorties du générateur d'images intégré
ImageGen, produites dans la conversation de conception du 12 septembre 2026.
Ils sont réservés à la documentation : ne pas les embarquer dans le build
public ni utiliser une image de page à la place du HTML/CSS.

Répertoire local de génération d'origine :
`~/.codex/generated_images/01a094bb-a633-7f13-bda3-88d27584e1a0/`.
La galerie utilise exclusivement les copies relatives versionnées dans ce dépôt.

| Référence | Fichier source dans ce répertoire | Dimensions | Taille |
| --- | --- | --- | --- |
| [Clair v1](./maquettes/landing-clair-v1.png) | `exec-edff9f50-96c7-4a31-a07e-c462498d0b8d.png` | 887 × 1774 px | 1 440 548 octets |
| [Sombre v1](./maquettes/landing-sombre-v1.png) | `exec-ed4bdde8-74e8-4c68-964f-98418e420559.png` | 887 × 1774 px | 1 487 723 octets |

Empreintes SHA-256 des originaux et des copies attendues :

```text
15368c3e9df310635a66a2b2278b803a0128ecbf7b9a53c89a43cdb2973a6d2a  landing-clair-v1.png
1058a0db6c4cc11814160350b124957cf048f73bc5452bd7628159fe26319f3a  landing-sombre-v1.png
```

Vérification depuis la racine du dépôt :

```sh
shasum -a 256 docs/design/landing/maquettes/landing-clair-v1.png docs/design/landing/maquettes/landing-sombre-v1.png
```

## Palettes et tokens cibles

Ces tokens sont implémentés dans `apps/landing/src/styles/global.css`.
Les deux thèmes utilisent les mêmes rôles.

| Usage | Token cible | Clair | Sombre |
| --- | --- | --- | --- |
| Fond principal | `--color-background` | `#F1E5D3` | `#111827` |
| Surfaces contrastées | `--color-surface` | `#FBF5EB` | `#192535` |
| Texte principal | `--color-text` | `#382E26` | `#FBF5EB` |
| Texte secondaire | `--color-text-secondary` | `#796B5D` | `#F1E5D3` |
| Accent | `--color-accent` | `#7B5E45` | `#8B5E3C` |
| Bouton principal | `--color-button` | `#7B5E45` | `#714426` |
| Texte du bouton | `--color-button-text` | `#FBF5EB` | `#FBF5EB` |

- **Fond principal** : en-tête, introduction, présentation des quatre services,
  section d'actualités hors aperçu, pourtour du calendrier et pied de page.
- **Surface** : bandeau du flux audio, section entière du résumé audio du jour,
  aperçu du fil, cellules du calendrier et intérieur du bouton de connexion.
- **Accent** : marque audio, pictogrammes, ondes audio et séparateurs décoratifs.
  Les séparateurs fins peuvent utiliser l'accent à 25 % d'opacité ; aucune
  information ne doit dépendre de cette seule séparation.
- **Bouton** : accès principal au flux et disques portant les triangles de
  lecture ; les triangles et libellés utilisent le texte de bouton.
- Les deux bandes audio vont d'un bord à l'autre du viewport. Les titres,
  pictogrammes et commandes restent dans le conteneur centré, sans étirement.

La surface sombre retenue est **`#192535`**. `#101B2B` appartenait à une
proposition antérieure et ne doit pas être réintroduit pour ce rôle.

### Contrastes et états interactifs

Les rapports suivants sont calculés sur les couleurs sRGB documentées, pas
mesurés sur les pixels des maquettes. Le contraste entre fond et surface est
environ 1,15:1 dans les deux thèmes ; ce rapport décoratif n'est pas un seuil
de lisibilité pour le texte.

| Premier plan / fond | Contraste approximatif | Usage |
| --- | --- | --- |
| `#796B5D` / `#F1E5D3` | 4,15:1 | Insuffisant pour du petit texte courant |
| `#796B5D` / `#FBF5EB` | 4,75:1 | Texte secondaire sur surface crème |
| `#FBF5EB` / `#7B5E45` | 5,49:1 | Libellé du bouton clair |
| `#FBF5EB` / `#714426` | 7,57:1 | Libellé du bouton sombre |
| `#8B5E3C` / `#111827` | 3,18:1 | Accent décoratif sombre |
| `#8B5E3C` / `#192535` | 2,77:1 | Décoration uniquement, pas une information autonome |
| `#F1E5D3` / `#192535` | 12,44:1 | Texte secondaire sombre sur surface |

Règles d'application :

- Utiliser le brun principal pour les petits textes sur fond avoine. Réserver
  le taupe aux surfaces crème, ou aux grands textes qui satisfont le seuil
  applicable ; ne pas éclaircir silencieusement le fond pour contourner la règle.
- En sombre, utiliser crème ou avoine pour les petits libellés, liens et
  informations. Les icônes cuivre purement décoratives accompagnent des textes
  explicites ; les pictogrammes qui portent seuls une action utilisent une
  couleur de texte contrastée.
- Souligner les liens textuels ; le survol renforce le soulignement plutôt
  que de diminuer l'opacité ou le contraste du libellé.
- Prévoir un focus visible de 3 px, avec décalage de 3 px, utilisant la couleur
  du texte principal sur le fond environnant. Aucun état ne dépend uniquement
  de la couleur, y compris le choix du thème.
- Vérifier 4,5:1 pour le texte courant, 3:1 pour les grands textes et les
  éléments graphiques nécessaires à l'utilisation des commandes.

## Composition et responsive

Conserver les familles locales déjà utilisées par la landing :
`Georgia, "Times New Roman", Times, serif` pour les titres et la marque ;
`Arial, Helvetica, sans-serif` pour les descriptions, menus, listes et tableaux.
La seconde ligne du H1 est en italique. Les PNG ne permettent pas d'identifier
une police exacte ; aucune police externe n'est nécessaire.

L'icône de marque est l'onde audio à cinq traits verticaux arrondis, de hauteurs
croissantes puis décroissantes. Réutiliser `AudioMark.astro` à gauche de la
marque dans l'en-tête et dans le pied de page. Les autres pictogrammes et les
ondes sont des SVG, pas des captures des maquettes.

- Conteneur centré de largeur maximale 1180 px ; gouttières de 24 px à partir
  de 768 px et de 16 px en dessous. Seuls les fonds des bandes audio s'étendent
  à toute la largeur.
- Les quatre services occupent quatre colonnes à partir de 1024 px, deux
  colonnes entre 640 et 1023 px, puis une colonne en dessous.
- Les sections éditoriales et leurs aperçus sont côte à côte à partir de
  1024 px, puis empilés avec le texte avant l'aperçu.
- Navigation complète à partir de 1024 px ; menu mobile natif en dessous.
  La commande de thème reste visible hors du menu replié. Les contrôles
  interactifs possèdent une cible d'au moins 44 × 44 px.
- Le calendrier garde ses sept colonnes et sa sémantique de tableau. Sur
  mobile, permettre un défilement horizontal dans un conteneur nommé et
  accessible au clavier, sans faire défiler horizontalement la page entière.
- Préserver l'ordre de lecture, des espacements généreux et des retours de
  ligne naturels sur mobile. Aucune hauteur fixe ne doit couper les textes.
- Les ondes audio sont statiques et décoratives ; aucune animation ne simule
  une lecture. Respecter la préférence de réduction des mouvements pour les
  éventuelles transitions et le défilement des ancres.

## Structure et textes retenus

### 1. En-tête

Marque : **fil finance**, précédée de l'icône audio.

Navigation : **Flux audio**, **Actualités**, **Résumé audio**, **Calendrier**.
Action de connexion : **Se connecter**. La commande du thème est décrite plus bas.

### 2. Introduction

Surtitre : **L’ACTUALITÉ FINANCIÈRE, EN AUDIO**.

H1, première ligne : **Écoutez les marchés.**

H1, seconde ligne en italique : **Restez dans votre session.**

Description :

> Un flux de news financières en français pour accompagner votre trading.
> Un calendrier hebdomadaire pour préparer vos rendez-vous.

Accès principal : **Écouter le flux**.

### 3. Présentation des quatre services

Titre : **Ce que propose Fil Finance**.

| Pictogramme | Titre | Description |
| --- | --- | --- |
| Onde audio | Flux audio de news en français | Suivez les marchés à l’écoute. |
| Journal | Fil d’actualités financières | Retrouvez les titres et leurs détails. |
| Casque | Résumé audio du jour en français | Réécoutez l’essentiel de la journée. |
| Calendrier | Calendrier économique hebdomadaire | Préparez les rendez-vous de la semaine. |

### 4. Flux audio

Bandeau contrasté pleine largeur, contenant un accès circulaire au flux,
son libellé, une onde décorative et le texte d'accompagnement :

- **Flux audio de news**.
- **Français · Aperçu illustratif**.
- **Votre fil pendant la session**.

Ce flux accompagne la session ; il ne doit pas être présenté comme le résumé
journalier de la section suivante.

### 5. Fil d'actualités financières

Titre : **L’actualité, au fil des marchés.**

Description :

> Consultez les actualités financières en français et ouvrez les détails qui vous intéressent.

Lien : **Consulter le fil →**.

Aperçu : titre **Fil d’actualités**, mention **Aperçu illustratif**.

| Heure illustrative | Catégorie | Sujet illustratif |
| --- | --- | --- |
| 14:30 | Économie | Indicateurs économiques |
| 14:00 | Banques centrales | Décisions et prises de parole |
| 13:45 | Marchés | Actualités des marchés |

Ces lignes sont des exemples éditoriaux fixes, pas des annonces réelles.

### 6. Résumé audio du jour

Section contrastée pleine largeur.

Titre : **L’essentiel du jour, en audio.**

Description :

> Retrouvez les principales actualités de la journée dans un résumé audio en français.

Libellé de l'aperçu : **Résumé audio du jour**.

Mention : **Français · Aperçu illustratif**.

Lien : **Écouter le résumé →**.

Le disque de lecture et l'onde sont une présentation du service. Ne pas ajouter
de durée, progression, état « en direct » ou disponibilité réelle inventés.

### 7. Calendrier économique hebdomadaire

Titre : **La semaine se prépare.**

Description :

> Les rendez-vous économiques, réunis dans une vue hebdomadaire.

Titre du tableau : **Calendrier hebdomadaire**.

Mention : **Exemple de semaine**.

Période illustrative fixe : **7 – 13 septembre 2026**. Chaque en-tête de jour
est suivi de **septembre**.

| Lun 7 | Mar 8 | Mer 9 | Jeu 10 | Ven 11 | Sam 12 | Dim 13 |
| --- | --- | --- | --- | --- | --- | --- |
| Conjoncture | Inflation | Banques centrales | Inflation | Banques centrales | — | — |
| Banques centrales | Conjoncture | — | Conjoncture | — | — | — |
| — | — | — | — | — | — | — |

Conserver la mention d'exemple et cette période explicite, sans la présenter
comme la semaine courante ni mettre à jour seulement ses dates automatiquement.

### 8. Pied de page

Icône audio puis marque **fil finance**.

Texte : **© 2026 Fil Finance. Tous droits réservés.**

Liens : **Flux audio**, **Actualités**, **Résumé audio**, **Calendrier**,
**Se connecter**.

Le pied de page suit le calendrier. Aucun bloc gamma, « À venir », photographie,
graphique de trading, carnet d'ordres ou ancien bloc explicatif n'est à insérer.

## Navigation et aperçus

Les ancres internes cibles sont `#top`, `#contenu`, `#offre`, `#flux-audio`,
`#actualites`, `#resume-audio` et `#calendrier`. Les liens du menu et du pied
de page utilisent les quatre ancres des services ; la marque retourne à `#top`.

| Action | Destination |
| --- | --- |
| Écouter le flux, dans l'introduction ou le bandeau | `externalDestinations.home` |
| Consulter le fil | `externalDestinations.home` |
| Écouter le résumé, lien ou disque de lecture | `externalDestinations.home` |
| Se connecter | `externalDestinations.login` |

Ces destinations restent issues de la configuration existante. En production,
elles valent `https://app.filfinance.fr/` et `https://app.filfinance.fr/login`.
Les origines propres aux environnements restent configurables ; aucun nouveau
chemin PWA ou deep link n'est inventé dans cet EPIC.

Les actions ouvrent l'application dans le même onglet. Les disques de lecture
sont des liens, avec un nom accessible tel que « Écouter le flux dans
l’application » ou « Écouter le résumé dans l’application », pas des boutons
de lecture inopérants. Leurs libellés accessibles conservent les mots visibles.
Les exemples textuels ne prétendent pas être des données actuelles.

## Fonctionnement du thème

### Choix et persistance

- Première visite : **clair**, même si le système préfère le sombre.
- Stockage local : clé **`filfinance.landing.theme`**, valeurs **`light`** et
  **`dark`** uniquement. Pas de mode système, de cookie, de synchronisation
  de compte ou de transfert de préférence vers la PWA.
- Au chargement, lire la préférence avant le premier affichage et appliquer
  le thème à la racine HTML via `data-theme`. Aligner la propriété CSS
  `color-scheme` et la métadonnée `theme-color` sur le thème effectif.
- Une préférence absente ou invalide donne le clair. Si la lecture du stockage
  échoue, démarrer en clair sans erreur non interceptée.
- Une bascule actualise immédiatement l'affichage ; une erreur d'écriture du
  stockage n'annule pas le choix pour la visite courante. Après rechargement,
  seule une préférence effectivement enregistrée peut être restaurée.
- Sans JavaScript, la page et ses liens restent utilisables en clair. Masquer
  la commande tant que son comportement n'est pas disponible.

### Commande accessible

Prévoir dans l'en-tête un bouton bascule au libellé stable **Thème sombre**,
avec `aria-pressed="false"` en clair et `aria-pressed="true"` en sombre.
En présentation compacte, un pictogramme est accompagné du même nom accessible
et d'une infobulle ; l'état ne repose pas seulement sur sa couleur.
La commande reste disponible hors du menu mobile replié, utilisable au clavier,
avec une cible de 44 × 44 px et un focus visible.

### Contrainte technique

La landing reste un build Astro statique. Le seul script applicatif autorisé
est celui du thème, comprenant l'application précoce de la préférence et la
gestion de la commande, sans framework client ni bibliothèque supplémentaire.

Le validateur compare le script de thème intégré dans le `head` à la source
locale `apps/landing/src/scripts/theme.js`. Il contrôle son identifiant,
son unicité et ses attributs ; un script supplémentaire, altéré ou externe
est refusé. JSON-LD reste autorisé, les ressources tierces bloquantes restent
interdites. Le README et l'architecture décrivent désormais cette seule
exception à l'absence de JavaScript applicatif.

## Validation et suivi

- [NEWS-222 — Référentiel documentaire](../../tickets/NEWS-222-referentiel-visuel-landing.md).
- [NEWS-223 — Thèmes et mémorisation](../../tickets/NEWS-223-themes-landing-clair-sombre.md).
- [NEWS-224 — Sections et aperçus](../../tickets/NEWS-224-sections-landing-quatre-services.md).
- [NEWS-225 — Responsive, navigation et accessibilité](../../tickets/NEWS-225-landing-responsive-navigation-accessibilite.md).
- [NEWS-226 — Recette des deux thèmes](../../tickets/NEWS-226-recette-landing-themes.md).

La recette du 12 septembre 2026 couvre 320, 390, 768 et 1440 px dans les deux
thèmes : voir le [compte rendu](../../../apps/landing/design-qa.md) et les
preuves datées (preuve interne conservée dans GitLab, exclue du dépôt de présentation). Les résultats QA historiques
restent séparés de cette validation et ne sont pas réutilisés comme preuve.
Les copies, empreintes et liens documentaires peuvent être validés séparément,
sans annoncer une validation de l'implémentation ni d'un déploiement.
