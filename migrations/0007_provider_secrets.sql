CREATE TABLE provider_secrets (
    provider_id  TEXT PRIMARY KEY NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
    secret       TEXT NOT NULL,
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL,
    CHECK (length(secret) BETWEEN 1 AND 8192)
) STRICT;
