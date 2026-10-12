# EPIC-023 - Administration runtime backend Fil Finance

## Objectif

Ajouter des surfaces admin frontend pour piloter et inspecter certains parametres runtime du backend Rust sans modifier les fichiers `.env` a la main.

Cet EPIC reste separe de la refonte architecture frontend `EPIC-022`: il porte des capacites d'exploitation produit, pas une restructuration technique globale.

## Decisions

- Les pages sont reservees aux comptes `moderator`/`admin`.
- Les actions passent par le backend Rust, jamais par un acces direct aux fichiers serveur depuis le client.
- Les changements runtime doivent etre explicites, traces et visibles dans les endpoints d'observabilite existants.
- Les logs visibles en admin doivent etre bornes a deux jours pour eviter une surface trop large et garder le stockage disque previsible.

## Tickets

- [NEWS-151 - Page admin profil de traduction](../tickets/NEWS-151-admin-profil-traduction.md)
- [NEWS-152 - Rotation logs deux jours et viewer admin](../tickets/NEWS-152-admin-logs-rotation-2-jours.md)

## Definition of Done

- Les pages admin existent dans le client web.
- Les routes backend sont protegees par le role operateur.
- Les changements de profil de traduction sont visibles dans le snapshot runtime traduction.
- La rotation de logs conserve seulement J et J-1.
- Le viewer logs ne permet pas de lire de fichiers arbitraires.
- Les tests backend et frontend couvrent les permissions et les contrats JSON.
