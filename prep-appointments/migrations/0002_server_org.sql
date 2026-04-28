-- Server Organisation: workspaces, Tyrant forms, invitations

CREATE TABLE IF NOT EXISTS server_workspaces (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    kingshot_server_number INTEGER NOT NULL,
    owner_account_key TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_server_workspaces_kingshot_server
    ON server_workspaces (kingshot_server_number);
CREATE INDEX IF NOT EXISTS idx_server_workspaces_owner
    ON server_workspaces (owner_account_key);

CREATE TABLE IF NOT EXISTS server_workspace_members (
    workspace_id TEXT NOT NULL REFERENCES server_workspaces(id) ON DELETE CASCADE,
    account_key TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('owner', 'admin')),
    PRIMARY KEY (workspace_id, account_key)
);

CREATE TABLE IF NOT EXISTS server_workspace_invites (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES server_workspaces(id) ON DELETE CASCADE,
    from_account_key TEXT NOT NULL,
    to_friend_code TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'accepted', 'rejected', 'revoked')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_server_workspace_invites_to_code
    ON server_workspace_invites (to_friend_code, status);

CREATE TABLE IF NOT EXISTS tyrant_forms (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES server_workspaces(id) ON DELETE CASCADE,
    public_code TEXT NOT NULL UNIQUE,
    config JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_tyrant_forms_workspace
    ON tyrant_forms (workspace_id);

CREATE TABLE IF NOT EXISTS tyrant_submissions (
    id BIGSERIAL PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES server_workspaces(id) ON DELETE CASCADE,
    form_id TEXT REFERENCES tyrant_forms(id) ON DELETE SET NULL,
    public_code TEXT NOT NULL,
    player_id TEXT NOT NULL,
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_tyrant_submissions_workspace
    ON tyrant_submissions (workspace_id);
CREATE INDEX IF NOT EXISTS idx_tyrant_submissions_code
    ON tyrant_submissions (public_code);
CREATE INDEX IF NOT EXISTS idx_tyrant_submissions_player
    ON tyrant_submissions (player_id);
