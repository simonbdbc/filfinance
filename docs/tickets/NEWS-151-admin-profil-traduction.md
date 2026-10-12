# NEWS-151 - Page admin profil de traduction

## Objectif

Permettre a un operateur de consulter et modifier le profil de traduction actif depuis une page admin frontend.

Le profil de traduction devient la source de verite fonctionnelle pour le compromis qualite/latence OPUS-MT:

- `fast` -> `beam-size=1`
- `balanced` -> `beam-size=2`
- `quality` -> `beam-size=4`

## Scope backend

- Ajouter un endpoint protege `GET /api/admin/translation/profile`.
- Ajouter un endpoint protege `PATCH /api/admin/translation/profile`.
- Valider strictement les valeurs `fast`, `balanced`, `quality`.
- Exposer le profil actif dans le runtime snapshot existant.
- Refuser toute valeur de profil inconnue avec une erreur `400`.
- Ne pas accepter `--beam-size` dans `NEWS_TRANSLATION_COMMAND_ARGS`.

## Scope frontend

- Ajouter une section admin "Traduction".
- Afficher le profil actif, le provider, l'etat du runtime et les derniers compteurs utiles.
- Proposer un controle segmente `fast / balanced / quality`.
- Afficher un etat de chargement, succes et erreur.
- Restreindre l'acces UI aux roles `moderator`/`admin`.

## Criteres d'acceptation

- Un moderator peut lire le profil actif depuis l'admin.
- Un moderator peut changer le profil sans redeployer le frontend.
- Un listener ne peut pas acceder aux endpoints admin.
- Le changement est visible dans `/api/runtime/translation`.
- Les tests couvrent la validation des valeurs et les permissions.

## Hors scope

- Gestion d'un glossaire de traduction.
- Multiples providers par langue.
- Workspace Cargo pour isoler la traduction.
