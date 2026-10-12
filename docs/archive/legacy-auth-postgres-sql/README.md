# SQL Supabase Postgres - Fil Finance

Ce dossier documente le schema Postgres cible heberge dans Supabase.

## Role dans l'architecture hybride

- Supabase Auth porte l'identite et la session.
- Supabase Postgres porte les profils, preferences et le role utilisateur unique dans `public.user_profiles.role`.
- Supabase Postgres porte aussi les parametres admin persistants du collecteur X dans `public.x_source_settings`.
- Supabase Postgres peut porter le glossaire metier `public.translation_glossary_entries` si le glossaire doit survivre au conteneur backend.
- SQLite local reste responsable du pipeline news/audio: ingestion, traductions, calendrier, replay et debug.
- SQLite local porte aussi le resume IA journalier: qualifications, summaries, scheduler et traces admin des runs. Ces tables ne sont pas migrees vers Supabase pour garder le processus n8n/OpenRouter proche des donnees news locales.

Le client Expo ne lit pas directement `public.user_profiles`. Il continue de passer par le backend Rust via `GET /api/me` et `PATCH /api/me`.

## Contenu

- `schema.sql`: schema Postgres consolide cible.
- `migrations/0001_create_user_profiles.sql`: creation de la table profils et garde-fous RLS.
- `migrations/0002_add_user_theme_preference.sql`: ajout de la preference `theme` (`light` / `dark`).
- `migrations/0003_create_x_source_settings.sql`: parametres admin persistants du collecteur X.
- `migrations/0004_rename_user_roles_listener_moderator.sql`: renommage `reader` -> `listener` et `operator` -> `moderator`.
- `migrations/0005_create_translation_glossary_entries.sql`: glossaire metier traduction/TTS.
- `migrations/0006_seed_translation_glossary_patterns.sql`: motif idempotent `ECB's <string>` -> `<string> de la BCE`, par exemple `ECB's Nagel` -> `Nagel de la BCE`; la capture `<string>` exclut la ponctuation terminale pour laisser `<string>:` gerer l'espace avant deux-points separement.
- `migrations/0007_add_translation_glossary_scope.sql`: portee `full_pipeline` / `spoken_only` des regles diction/TTS.
- `migrations/0008_seed_spoken_time_glossary_patterns.sql`: motifs vocaux idempotents `14h00 -> quatorze heures` et `12h01 -> douze heures une` en `spoken_only`.

## Politique d'acces

Les tables `public.user_profiles`, `public.x_source_settings` et `public.translation_glossary_entries` ont le Row Level Security active des le depart.

Le premier lot recommande un acces serveur uniquement:

- le backend Rust verifie le JWT Supabase;
- le backend lit/ecrit le profil avec une connexion Postgres privee cote serveur;
- aucune `service_role key` ne doit etre exposee au client Expo;
- aucune policy publique de lecture/ecriture client n'est creee tant que le client ne lit pas directement la table.

Si un acces direct client devient necessaire plus tard, il faudra ajouter des policies explicites limitees a `auth.uid() = supabase_user_id`.

## Roles utilisateur

Roles autorises:

- `listener` (`auditeur`)
- `moderator` (`modérateur`)
- `admin`

Tout profil cree automatiquement demarre en `listener`. Les promotions vers `moderator` ou `admin` doivent passer par une operation serveur ou admin controlee.
