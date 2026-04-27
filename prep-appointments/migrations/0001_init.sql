CREATE TABLE IF NOT EXISTS accounts (
    account_key TEXT PRIMARY KEY,
    payload JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS current_forms_map (
    schedule_key TEXT PRIMARY KEY,
    form_code TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS forms (
    code TEXT PRIMARY KEY,
    account_name TEXT NOT NULL,
    server_number INTEGER NOT NULL,
    archived BOOLEAN NOT NULL DEFAULT FALSE,
    archive_name TEXT,
    payload JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_forms_account_server
    ON forms (account_name, server_number);
CREATE INDEX IF NOT EXISTS idx_forms_archived
    ON forms (archived);
CREATE INDEX IF NOT EXISTS idx_forms_archive_name
    ON forms (archive_name);

CREATE TABLE IF NOT EXISTS schedules (
    account_name TEXT NOT NULL,
    server_number INTEGER NOT NULL,
    payload JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (account_name, server_number)
);

CREATE TABLE IF NOT EXISTS statistics (
    account_name TEXT NOT NULL,
    server_number INTEGER NOT NULL,
    payload JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (account_name, server_number)
);

CREATE TABLE IF NOT EXISTS feedback (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    payload JSONB NOT NULL
);

CREATE TABLE IF NOT EXISTS form_submissions (
    id BIGSERIAL PRIMARY KEY,
    form_code TEXT NOT NULL,
    player_id TEXT,
    row_data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_form_submissions_form_code
    ON form_submissions (form_code);
CREATE INDEX IF NOT EXISTS idx_form_submissions_player_id
    ON form_submissions (player_id);

CREATE TABLE IF NOT EXISTS domain_documents (
    domain TEXT NOT NULL,
    doc_key TEXT NOT NULL,
    payload JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (domain, doc_key)
);
