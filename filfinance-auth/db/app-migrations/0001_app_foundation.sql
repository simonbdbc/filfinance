CREATE TABLE IF NOT EXISTS app.profiles (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    auth_user_id text UNIQUE,
    email text NOT NULL,
    display_name text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS app.auth_identity_migration (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    source_provider text NOT NULL,
    source_user_id text NOT NULL,
    better_auth_user_id text,
    normalized_email text,
    migrated_at timestamptz,
    migration_status text NOT NULL DEFAULT 'pending',
    migration_error text,
    UNIQUE (source_provider, source_user_id)
);

CREATE TABLE IF NOT EXISTS app.auth_bootstrap (
    bootstrap_key text PRIMARY KEY,
    promoted_auth_user_id text,
    promoted_email text,
    promoted_at timestamptz NOT NULL DEFAULT now()
);
