# NEWS-152 - Rotation logs deux jours et viewer admin

## Objectif

Mettre en place une retention de logs sur deux jours et une page admin permettant de consulter les logs backend recents.

La cible de retention est stricte:

- conserver J
- conserver J-1
- supprimer J-2 et plus

## Scope backend

- Configurer une sortie fichier journaliere pour les logs backend.
- Conserver seulement deux jours de fichiers de logs.
- Ajouter un endpoint protege `GET /api/admin/logs/days`.
- Ajouter un endpoint protege `GET /api/admin/logs?day=YYYY-MM-DD&limit=...&cursor=...`.
- Borner la taille de lecture et paginer les lignes.
- Refuser tout chemin arbitraire ou toute date hors retention.
- Documenter les variables `NEWS_LOG_*` si une configuration est exposee.

## Scope frontend

- Ajouter une page admin "Logs".
- Lister les deux jours disponibles.
- Afficher les logs pagines avec recherche/filtre niveau si le backend l'expose.
- Prevoir un bouton refresh.
- Afficher clairement les erreurs de permission ou d'indisponibilite.

## Criteres d'acceptation

- Les fichiers J-2+ sont supprimes automatiquement.
- Un moderator peut lire les logs J et J-1 depuis l'admin.
- Un listener ne peut pas lire les logs.
- Le viewer ne lit jamais un chemin arbitraire.
- Les logs volumineux ne bloquent pas le serveur.

## Implementation actuelle

- `NEWS_LOG_FILE_ENABLED=true` active une sortie fichier en plus de stdout.
- `NEWS_LOG_DIR` pointe vers le repertoire des fichiers `YYYY-MM-DD.log`.
- `NEWS_LOG_RETENTION_DAYS=2` est obligatoire pour conserver uniquement J et J-1.
- `NEWS_LOG_ADMIN_MAX_LINES` borne la pagination de `/api/admin/logs`.
- La page admin `Logs` liste les jours disponibles, filtre par niveau/texte et charge la suite par curseur.

## Hors scope

- Centralisation Loki/ELK.
- Export long terme des logs.
- Edition ou suppression manuelle depuis le frontend.
