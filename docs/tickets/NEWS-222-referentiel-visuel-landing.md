# NEWS-222 — Référentiel visuel de la landing

## Statut

**Terminé — documentation vérifiée le 12 septembre 2026.**

EPIC : [EPIC-038](../epics/EPIC-038-refonte-visuelle-landing-themes-clair-sombre.md).

## Objectif métier

Donner une référence unique et directement exploitable pour la refonte de la
landing, avec les deux palettes, les textes, les comportements retenus et les
deux dernières maquettes à quatre services, sans gamma.

## Périmètre

- Écrire le [référentiel visuel](../design/landing/README.md).
- Copier à l'identique les PNG clair et sombre de 887 × 1774 px dans la
  documentation ; consigner leur provenance, leur taille et leur SHA-256.
- Documenter les huit blocs, les quatre services, la pleine largeur des deux
  sections audio et l'icône de marque dans l'en-tête et le pied de page.
- Fixer le clair à la première visite, le choix mémorisé et les aperçus avec
  accès à la PWA, sans lecture ni données publiques réelles.
- Créer l'EPIC, NEWS-223 à NEWS-226 et les liens depuis README et roadmap.

## Hors périmètre

Modification du frontend, scripts applicatifs, backend, PWA, tests de runtime,
déploiement ou clôture de l'EPIC. Les modifications locales préexistantes de
la landing ne font pas partie de cette livraison documentaire.

## Approche technique

Utiliser des liens Markdown relatifs dans les documents. Conserver les images
originales sans recompression dans `docs/design/landing/maquettes/`, à l'écart
des assets servis. Transcrire les textes au lieu de demander au futur
implémenteur de les déduire des images. Distinguer les références ImageGen des
captures et preuves de recette d'une application implémentée.

## Critères d'acceptation

- Les identifiants EPIC-038 et NEWS-222 à NEWS-226 sont uniques.
- Les deux galeries pointent sur des fichiers présents et lisibles ; leurs
  dimensions et empreintes correspondent aux sources.
- Les palettes de l'EPIC et du référentiel sont cohérentes ; les contraintes
  de contraste sont écrites sans modifier les couleurs retenues.
- La référence sombre utilise `#192535` comme surface ; seules les deux
  dernières maquettes, à quatre services et sans gamma, sont retenues.
- Les liens ajoutés se résolvent ; les bilans historiques sont conservés.
- Les futurs tickets comportent objectif, périmètre, exclusions, approche,
  critères, tests et dépendances. Ils restent à faire.
- Aucun fichier de la landing n'est changé par cette livraison documentaire.

## Tests attendus

Vérifier les liens et ancres documentaires, l'unicité des nouveaux identifiants,
le décodage des PNG, leurs dimensions, leurs tailles et leurs empreintes.
Comparer les copies aux sources octet pour octet. Relire les textes et
contrôler le diff pour séparer les ajouts documentaires des changements locaux
préexistants. Aucun build applicatif n'est requis pour cette livraison.

## Dépendances

Aucune dépendance d'implémentation. Les deux maquettes de référence et les
choix du plan accepté sont disponibles. Débloque
[NEWS-223](./NEWS-223-themes-landing-clair-sombre.md).

## Preuves de livraison documentaire — 12 septembre 2026

- EPIC-038 et les cinq tickets NEWS-222 à NEWS-226 possèdent chacun un fichier
  unique ; les cinq tickets contiennent toutes les rubriques du contrat local.
- Les 43 liens des nouveaux documents et des ajouts au README/à la roadmap
  ciblent des fichiers présents. La galerie référence seulement les deux
  dernières maquettes à quatre services, sans gamma.
- Les deux PNG sont des copies identiques aux sources : 887 × 1774 px chacun,
  1 440 548 et 1 487 723 octets. SHA-256 conformes au référentiel ; signatures,
  intégrité des blocs PNG et décompression des données vérifiées.
- Les sept couples de couleurs sont identiques dans l'EPIC et le référentiel.
- Les empreintes des 42 fichiers de la landing contrôlés avant et après cette
  livraison sont identiques, y compris les modifications locales préexistantes.
- Aucun build, audit frontend ou déploiement exécuté pour cette livraison.
  NEWS-223 à NEWS-226 restent à faire et EPIC-038 reste planifié.
