# NEWS-234 — Refonte des écrans et états d’authentification

## Statut

**Terminé localement — 17 septembre 2026.** gluestack v5 / UniWind / Tailwind CSS 4.

EPIC : [EPIC-039](../epics/EPIC-039-refonte-pwa-gluestack.md).
Référence : [charte PWA et inventaire des routes](../design/pwa/README.md).

## Objectif métier

Assurer une entrée dans la PWA visuellement cohérente, accessible et fiable pour tous les parcours de compte existants.

## Périmètre

- Migrer le cadre (auth), /login et ses modes connexion, inscription, mot de passe oublié et réinitialisation.
- Migrer /email-verification et les états de récupération de session, succès/erreur/attente associés.
- Conserver notice avant inscription, liens légaux hors session, valeurs saisies et destinations existantes.

## Hors périmètre

Nouvelles routes d’inscription/récupération, modification Better Auth ou serveur, cases d’acceptation, changement de contrat d’authentification, lecteur audio sur auth.

## Approche technique

- Réutiliser AuthGate et ses modes ; migrer présentation et contrôles via les exports UI partagés sans réécrire les appels auth.
- Préserver labels, autocomplétion, messages, focus d’erreur et prévention des doubles soumissions ; ne pas perdre les saisies en changeant le thème.
- Conserver la notice de collecte distincte des CGU et les quatre liens légaux dans le cadre commun, même déconnecté.
- Préserver l’ouverture des liens Web en nouvel onglet et le gestionnaire externe existant sur natif ; aucune preuve native revendiquée dans ce ticket.
- Conserver redirections et traitement des liens/token ; ne jamais mettre de secret réel dans les captures de recette.

## Critères d’acceptation

- Tous les modes existants restent utilisables, avec champs et messages accessibles dans les deux thèmes.
- Aucun lecteur ni abonnement audio créé sur les écrans auth ; thème conforme à l’état commun.
- Consulter les informations légales préserve le formulaire Web ; aucune nouvelle case d’acceptation ajoutée.
- Les redirections, vérifications et erreurs de récupération conservent leur comportement actuel.

## Tests attendus

- Parcourir connexion, inscription, vérification, oubli et réinitialisation avec jeux de test ; couvrir chargement, erreur, succès et récupération de session.
- Vérifier focus, ordre clavier, validation, double soumission, changement de thème, stockage refusé et ouverture légale sans perte de saisie.
- Deux thèmes aux cinq largeurs et zoom 200 % ; check:pwa dont qa:auth/qa:legal, build:pwa.

## Dépendances

- [NEWS-229](./NEWS-229-composants-reutilisables-pwa.md).

Les commentaires du code livré restent en français. Les preuves de recette locale restent distinctes de tout déploiement.

## Livraison et preuves

Tous les modes auth migrés ; validations, formulaires et liens légaux préservés. Le nettoyage du jeton est confirmé après montage Expo.

[Recette locale, commandes, résultats et captures](../design/pwa/recette/2026-09-16/README.md).
Aucun commit, push, déploiement ou livraison native.
