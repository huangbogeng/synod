CREATE TABLE server_state (
    singleton_id            INTEGER PRIMARY KEY NOT NULL CHECK (singleton_id = 1),
    bootstrap_principal_id  TEXT REFERENCES principals(id) ON DELETE RESTRICT
) STRICT;

INSERT INTO server_state(singleton_id) VALUES (1);

CREATE TABLE principal_tokens (
    id             TEXT PRIMARY KEY NOT NULL,
    principal_id   TEXT NOT NULL REFERENCES principals(id) ON DELETE CASCADE,
    label          TEXT NOT NULL,
    token_hash     BLOB NOT NULL UNIQUE,
    expires_at     TEXT,
    revoked_at     TEXT,
    created_at     TEXT NOT NULL
) STRICT;

CREATE INDEX principal_tokens_principal_idx ON principal_tokens(principal_id);
