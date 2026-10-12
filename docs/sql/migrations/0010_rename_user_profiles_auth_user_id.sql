-- Renomme l'identifiant profil pour le rendre fournisseur-agnostique.
-- Le bootstrap Rust applique la compatibilite idempotente pour les bases
-- creees avant `EPIC-036`, car le nom de colonne legacy etait specifique
-- a l'ancien fournisseur d'auth.

INSERT OR IGNORE INTO schema_migrations (name)
VALUES ('0010_rename_user_profiles_auth_user_id');
