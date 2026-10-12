# NEWS-223 — Thèmes clair/sombre et mémorisation de la landing

## Statut

**Terminé localement le 12 septembre 2026.** Tokens, script unique dans le
`head`, commande accessible et mémorisation sont implémentés. Les 15 tests
de `check:landing` couvrent le thème et le rejet des scripts inattendus.
La recette navigateur vérifie aussi les pannes de stockage, l'absence de JS
et le thème sombre avant l'insertion du contenu. Voir les
[preuves de recette](../design/landing/recette/2026-09-12/README.md).

EPIC : [EPIC-038](../epics/EPIC-038-refonte-visuelle-landing-themes-clair-sombre.md).
Source de vérité : [palettes et comportement](../design/landing/README.md).

## Objectif métier

Permettre au visiteur de choisir un thème clair ou sombre, conserver son choix
sur ce navigateur et retrouver immédiatement la même présentation à la visite
suivante, avec le clair par défaut.

## Périmètre

- Introduire les sept tokens sémantiques des deux palettes documentées dans
  les styles de la landing et supprimer leur dépendance à des couleurs fixes
  dans les éléments conservés.
- Ajouter la commande accessible dans l'en-tête, visible aussi sur mobile.
- Appliquer le thème enregistré avant le premier affichage, puis gérer la
  bascule et sa mémorisation avec un script local minimal.
- Adapter la validation de production à ce seul script applicatif autorisé.
- Mettre à jour les mentions techniques « sans JavaScript applicatif » pour
  décrire cette exception uniquement quand elle est implémentée.

## Hors périmètre

Mode système, framework client, dépendance JavaScript supplémentaire,
préférence de compte, synchronisation PWA, cookies, API et refonte des sections
prise en charge par NEWS-224.

## Approche technique

- Conserver le build Astro statique et centraliser les palettes dans les
  variables CSS décrites par le référentiel.
- Utiliser `data-theme="light"` ou `data-theme="dark"` sur la racine HTML,
  avec CSS clair par défaut. Aligner `color-scheme` et `meta[name="theme-color"]`
  sur le fond principal du thème effectif.
- Lire `localStorage["filfinance.landing.theme"]` avant le premier affichage.
  Accepter seulement `light` et `dark`. Préférence absente, invalide ou lecture
  impossible : clair, sans suivre `prefers-color-scheme`.
- Regrouper l'initialisation et la gestion de commande dans un script de thème
  local, sans chargement distant. Intercepter les erreurs de stockage ; une
  écriture refusée ne doit pas annuler la bascule dans la page courante.
- Bouton au nom stable « Thème sombre », `aria-pressed` reflétant le thème
  effectif. Révéler la commande après installation de son comportement ; sans
  JavaScript, elle reste masquée et la landing demeure utilisable en clair.
- Dans `scripts/validate-production-build.mjs`, conserver JSON-LD et autoriser
  uniquement le script de thème attendu, identifié et comparé à sa source
  maîtrisée ou à son empreinte. Un attribut déclaratif seul ne suffit pas.
  Rejeter les scripts supplémentaires, les doublons et les ressources tierces
  bloquantes ; ne pas supprimer le contrôle global des scripts.
- Commenter les nouvelles règles et le script en français.

## Critères d'acceptation

- Première visite en clair, y compris avec système sombre.
- Bascule clavier/pointeur immédiate, nom accessible et état annoncé cohérents.
- Préférence valide restaurée avant le premier affichage, sans flash clair
  lors du retour d'un visiteur ayant choisi le sombre.
- Préférence invalide ou stockage indisponible : repli sûr en clair ; erreur
  d'écriture : thème courant conservé sans exception non interceptée.
- Sans JavaScript : contenu, navigation et accès PWA utilisables en clair.
- Pas d'accès au profil ni de modification de la préférence PWA.
- Le validateur accepte le seul script prévu et refuse une injection de script
  étranger, même marquée avec l'identifiant du thème.

## Tests attendus

Vérifier première visite, préférence `light`, préférence `dark`, valeur
invalide, échec de lecture, échec d'écriture et JavaScript désactivé. Couvrir
le changement d'état et sa restauration au rechargement, ainsi que la
cohérence de `data-theme`, `aria-pressed`, `color-scheme` et `theme-color`.
Vérifier le premier affichage en sombre dans le navigateur, pas seulement
l'état final du DOM. Ajouter un cas de rejet de script inattendu au contrôle
de production et exécuter `npm run check:landing`.

## Dépendances

- [NEWS-222](./NEWS-222-referentiel-visuel-landing.md).
- Débloque [NEWS-224](./NEWS-224-sections-landing-quatre-services.md).
