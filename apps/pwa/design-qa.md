# Revue visuelle — EPIC-039

Revue locale des 16–17 septembre 2026, fondée sur les références de
[`docs/design/pwa`](../../docs/design/pwa/README.md). Les captures viennent de
l’export Web réel ; API et sessions sont simulées et nommées dans la recette.

## Comparaison effectuée

La planche Actualités confirmée et le rendu clair 1440 px ont été inspectés côte
à côte. Les vues sombres 320 px et le Profil clair ont également été inspectés.
La structure conserve sidebar, titre Georgia, lecteur commun, outils du fil,
liste/détail, panneau mobile et navigation inférieure. Actualités n’intègre aucun
calendrier. Les couleurs CSS documentées priment sur les dégradés de la maquette.

Écarts intentionnels : pas de volume ni d’onde fictive ; textes/dates et statuts
issus des données ; résumé indisponible lorsqu’aucun média n’est disponible ;
entrée Administration selon les droits ; trois contrôles du fil qui peuvent
passer sur deux lignes à 320 px afin de garder les cibles de 44 px.

## Corrections issues de la recette

- Titre et date empilés sur petit écran pour éviter la coupure du titre.
- Lecteur compact et métadonnées lisibles, sans retour à la ligne lettre par lettre.
- Retrait de la double marge dans la liste mobile.
- Supervision admin rapprochée de la planche : sources et activité à gauche,
  outils compacts à droite, état opérationnel conservé en dessous.
- Actions des workflows bornées à la largeur disponible ; colonnes de workflow
  repliées verticalement ; tableaux mobiles transformés en lignes libellées.
- Focus visible, confinement et retour du dialogue ; noms des commandes et champs
  explicites, icônes exclues du nom accessible, listes sémantiques complètes.
- Couleur des icônes alignée sur celle du texte des boutons sélectionnés.
- Source désactivée neutre ; statuts d’erreur et de succès avec texte.

## Résultat et limites

Les 150 rendus contrôlés ne débordent pas horizontalement. Les seuils 1024 et
1280 px sont testés des deux côtés. Trente audits axe-core ne remontent aucune
violation ; les icônes de police, indéterminées pour l’algorithme de contraste,
utilisent les mêmes tokens textuels contrastés. Les ratios calculés et résultats
bruts sont conservés dans le dossier de recette.

La lecture directe et le résumé HLS local continuent pendant les changements de
route et de thème. Les tests ne prouvent ni la disponibilité de la production,
ni le rendu natif, ni une certification d’accessibilité. Le zoom est couvert par
le reflow équivalent à 200 %, complété par l’inspection visuelle et de l’arbre
accessible ; aucune session VoiceOver complète n’est revendiquée.

Preuves et galerie : [recette locale](../../docs/design/pwa/recette/2026-09-16/README.md).

Complément du 17 septembre : [défilement vertical et zones internes](../../docs/design/pwa/recette/2026-09-17-scroll/README.md).
`npm run qa:scroll` contrôle le déplacement réel, la fin visible des pages, le
clavier, le tactile, les panneaux et les logs. La seule absence de débordement
horizontal ne suffit pas à valider le défilement.
