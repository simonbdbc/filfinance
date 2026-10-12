-- Ajoute les traces admin des generations de resumes IA journaliers.

CREATE TABLE IF NOT EXISTS daily_summary_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL UNIQUE,
    local_day TEXT NOT NULL,
    summary_id INTEGER,
    status TEXT NOT NULL,
    reason TEXT NOT NULL,
    forced INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    completed_at TEXT
);

CREATE TABLE IF NOT EXISTS daily_summary_run_steps (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL,
    local_day TEXT NOT NULL,
    stage TEXT NOT NULL,
    status TEXT NOT NULL,
    message TEXT NOT NULL,
    detail TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS daily_summary_run_artifacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL,
    local_day TEXT NOT NULL,
    kind TEXT NOT NULL,
    label TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_daily_summary_runs_local_day_created_at
ON daily_summary_runs (local_day, created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_daily_summary_run_steps_run_id_created_at
ON daily_summary_run_steps (run_id, created_at ASC, id ASC);

CREATE INDEX IF NOT EXISTS idx_daily_summary_run_artifacts_run_id_created_at
ON daily_summary_run_artifacts (run_id, created_at ASC, id ASC);

INSERT OR IGNORE INTO schema_migrations (name)
VALUES ('0016_add_daily_summary_admin_runs');
