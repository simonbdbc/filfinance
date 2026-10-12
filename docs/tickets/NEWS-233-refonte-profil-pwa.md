# NEWS-233 — Refonte du profil, des préférences et des informations légales

## Statut

**Terminé localement — 17 septembre 2026.** gluestack v5 / UniWind / Tailwind CSS 4.

EPIC : [EPIC-039](../epics/EPIC-039-refonte-pwa-gluestack.md).
Référence : [charte PWA et inventaire des routes](../design/pwa/README.md).

## Objectif métier

Rendre les informations du compte et les préférences compréhensibles dans une page cohérente avec le reste de la PWA.

## Périmètre

- Migrer /profile : compte et informations existantes, préférence vocale affichée, thème et déconnexion.
- Présenter la rubrique Informations légales et ses quatre liens via les helpers existants.
- Réutiliser PreferenceGroup, sections, lignes et états de chargement/erreur communs.

## Hors périmètre

Nouveau formulaire de modification du compte ou de voix, nouvelle API, synchronisation du thème au profil serveur, copie des textes légaux dans la PWA.

## Approche technique

- Afficher les données réelles du profil en lecture seule, avec libellés existants ; conserver les informations actuelles même si le PNG en montre moins.
- Brancher le choix clair/sombre sur l’unique provider sans état local concurrent ; annoncer le choix sélectionné.
- Conserver les URL depuis EXPO_PUBLIC_LANDING_ORIGIN et l’ouverture Web en nouvel onglet ou native externe via le helper existant, avec gestion de l’échec.
- Conserver la déconnexion existante et ses transitions ; la refonte n’ajoute aucun pouvoir de gestion de compte.

## Critères d’acceptation

- Profil réel affiché sans nouveau formulaire ; préférence vocale informative et non un contrôle fictif.
- Thème cohérent avec la navigation, restauré au rechargement, bascule utilisable malgré une écriture de stockage refusée.
- Quatre liens légaux corrects selon l’environnement ; échec d’ouverture compréhensible.
- Déconnexion, reconnexion et états de profil indisponible restent fonctionnels sans rupture audio due à une simple consultation du profil.

## Tests attendus

- Compte complet, données optionnelles absentes, chargement/erreur ; vérification du choix de thème et de la déconnexion.
- Tests des quatre destinations configurées et d’ouverture ; navigateur avec compte utilisateur/admin, clavier et deux thèmes aux cinq largeurs.
- Exécuter check:pwa dont qa:profile et qa:legal, build:pwa et produire les captures de recette.

## Dépendances

- [NEWS-230](./NEWS-230-navigation-audio-persistant-pwa.md).

Les commentaires du code livré restent en français. Les preuves de recette locale restent distinctes de tout déploiement.

## Livraison et preuves

Compte en lecture seule, thème local, voix, déconnexion et liens légaux livrés ; aucune écriture du thème au serveur.

[Recette locale, commandes, résultats et captures](../design/pwa/recette/2026-09-16/README.md).
Aucun commit, push, déploiement ou livraison native.
