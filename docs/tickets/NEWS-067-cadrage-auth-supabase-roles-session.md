# NEWS-067 - Cadrage auth Supabase, roles et contrats de session

## Statut

Fait le 1 mai 2026.

## Objectif

Fixer le contrat d'authentification `EPIC-014` avant de proteger les routes HTTP et les WebSockets.

## Decisions

- fournisseur d'identite: `Supabase Auth`
- session cliente: geree par `@supabase/supabase-js`
- stockage metier news: conserve en SQLite dans cet EPIC
- cle publique client: `EXPO_PUBLIC_SUPABASE_PUBLISHABLE_KEY`
- compatibilite client acceptee: `EXPO_PUBLIC_SUPABASE_ANON_KEY` reste lisible si un projet Supabase n'a pas encore migre vers les nouvelles cles publishable
- backend: verification du JWT Supabase avant exposition des surfaces privees
- WebSocket: ouverture possible sans header custom navigateur, mais aucune diffusion avant validation explicite d'un jeton envoye en premier message

## Surfaces publiques

- `GET /health`

## Surfaces authentifiees

- `GET /api/sources`
- `GET /api/news`
- `GET /api/news/{id}`
- `GET /api/translations`
- `GET /api/calendar-day/today`
- `GET /ws/news`
- `GET /ws/audio`

## Surfaces operateur

- `GET /api/runtime/translation`
- `GET /api/runtime/tts`
- `GET /ws/test-runs`
- declenchement d'un `test_run`
- changement de voix TTS serveur si le backend decide de le classer comme action sensible

## Roles

Role minimal cote application:

- `listener`: lecture des flux news, audio et historique
- `moderator`: `listener` plus runtime et test-runs
- `admin`: `moderator` plus futures actions de gestion

Source recommandee:

- cible `EPIC-017`: table Supabase Postgres `public.user_profiles.role`
- etat `NEWS-071`: table locale SQLite `user_profiles.role`
- creation automatique en `listener` quand une session Supabase valide n'a pas encore de profil
- changement de role via operation controlee sur `user_profiles`

## Contrat JWT attendu

Le backend doit au minimum verifier:

- signature du token via la configuration Supabase
- `iss` attendu du projet Supabase
- `aud` compatible avec une session utilisateur
- `exp` non expire
- `sub` present et exploitable comme identifiant utilisateur
- role applicatif relu dans `user_profiles` apres verification du token, jamais depuis les claims JWT

## Contrat WebSocket cible

Le navigateur ne permet pas d'ajouter librement un header `Authorization` standard lors de l'ouverture d'un `WebSocket`.

Contrat retenu:

1. le client ouvre le WebSocket
2. le client envoie immediatement un message JSON `auth`
3. le serveur verifie le token
4. le serveur n'abonne le socket au flux et n'envoie les messages metier qu'apres validation
5. le serveur ferme le canal si le message `auth` est absent, invalide ou insuffisant pour la surface demandee

## Hors scope

- migration des tables news vers Supabase Postgres
- gestion fine des organisations ou equipes
- stockage durable avance des preferences utilisateur hors preference TTS minimale
