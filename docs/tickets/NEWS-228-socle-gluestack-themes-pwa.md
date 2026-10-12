# NEWS-228 — Socle gluestack, tokens et thèmes unifiés de la PWA

## Statut

**Terminé localement — 17 septembre 2026.** gluestack v5 / UniWind / Tailwind CSS 4.

EPIC : [EPIC-039](../epics/EPIC-039-refonte-pwa-gluestack.md).
Référence : [charte PWA et inventaire des routes](../design/pwa/README.md).

## Objectif métier

Garantir une identité visuelle commune à la landing et à toute la PWA, avec une préférence clair/sombre fiable et une base gluestack compatible.

## Périmètre

- Intégrer gluestack compatible Expo/React Native Web avec UniWind et Tailwind CSS 4, verrouiller les versions et documenter la combinaison retenue.
- Centraliser palettes, tokens d’état, typographie, espacements et rayons du référentiel dans shared/theme/.
- Relier gluestack, UniWind et les composants au provider de thème unique ; conserver filfinance.theme et le défaut clair.

## Hors périmètre

Livraison native iOS/Android, changement de routes ou d’API, synchronisation avec la landing/profil serveur, migration visuelle complète des écrans.

## Approche technique

- Partir du package et du lockfile existants, comparer les exigences de la filière gluestack v5 UniWind et verrouiller une combinaison réellement testée ; pas de latest ou alpha imposé par simple copie d’un exemple.
- Isoler le provider et les primitives derrière shared/ui. Effectuer un build Web avec un contrôle témoin avant de lancer les migrations métier.
- Définir un pont unique de tokens plutôt que recopier les palettes. Inventorier tokens.ts, appDesignTheme.ts et usages admin, avec adaptateurs temporaires suivis jusqu’à NEWS-237.
- Restaurer light/dark avant le contenu visible via un amorçage Web compatible Expo. Intercepter lecture/écriture du stockage ; lecture absente/invalide/refusée : clair, écriture refusée : conserver le choix courant.
- Conserver la priorité d’un choix explicite sur une restauration asynchrone tardive. Ne pas donner au sous-arbre audio une clé dépendante du thème.
- Appliquer les familles système du référentiel sur Web et un mapping compatible Expo pour les autres plateformes, sans télécharger de police propriétaire.

## Critères d’acceptation

- Build Web du socle gluestack réussi avant migration des écrans, versions et décision documentées.
- Une seule source de vérité pour les palettes et un seul état light/dark ; aucune dépendance NativeWind ou react-native-css-interop restante.
- Les sept couleurs de chaque thème et les tokens sémantiques correspondent au référentiel ; texte taupe/avoine et cuivre interactif corrigés.
- Défaut clair, restauration sans flash du mauvais thème, stockage défaillant sans exception non interceptée ni blocage du rendu.
- La bascule ne remonte pas le runtime audio et ne modifie pas les préférences de la landing.

## Tests attendus

- Tester absence, valeurs light/dark et invalide, lecture rejetée/retardée, écriture rejetée, choix utilisateur pendant lecture tardive et rechargement ; observer le premier affichage Web.
- Mesurer les contrastes texte/fond, boutons, contours et focus ; vérifier les familles et tailles effectives.
- Exécuter npm run check:pwa et npm run build:pwa ; adapter seulement les anciennes attentes visuelles justifiées en conservant les assertions métier.

## Dépendances

- [NEWS-227](./NEWS-227-referentiel-visuel-pwa.md).

Les commentaires du code livré restent en français. Les preuves de recette locale restent distinctes de tout déploiement.

## Socle retenu le 16 septembre 2026

Versions exactes : core 5.0.15, utils 5.0.6, UniWind 1.3.0, Tailwind 4.1.18.
Metro reçoit `global.css` via `withUniwindConfig` ; les déclarations sont générées
dans `uniwind-types.d.ts`. Les variables CSS sont l’unique source des palettes.
Le contrôle légal reste exécuté avant l’export Web. Le validateur `qa:ui-stack`
refuse NativeWind, css-interop et Tailwind 3 dans les sources et dépendances.
Le choix initial est lu synchroniquement sur Web ; aucun appel de profil ne peut
réappliquer un thème. L’API serveur conserve son contrat existant.

## Livraison et preuves

Socle exact verrouillé, Metro UniWind, CSS unique, préférence locale et retrait du profil ; installation et export Web validés.

[Recette locale, commandes, résultats et captures](../design/pwa/recette/2026-09-16/README.md).
Aucun commit, push, déploiement ou livraison native.
