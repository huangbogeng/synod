CREATE TABLE context_snapshots (
    id                     TEXT PRIMARY KEY NOT NULL,
    run_id                 TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE UNIQUE,
    manifest_json          TEXT NOT NULL CHECK (json_valid(manifest_json)),
    input_json             TEXT NOT NULL CHECK (json_valid(input_json)),
    estimated_input_tokens INTEGER NOT NULL CHECK (estimated_input_tokens >= 0),
    created_at             TEXT NOT NULL
) STRICT;

CREATE TABLE conversation_items (
    id              TEXT PRIMARY KEY NOT NULL,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    sequence        INTEGER NOT NULL CHECK (sequence > 0),
    kind            TEXT NOT NULL CHECK (kind IN ('trigger', 'model_message', 'error')),
    role            TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
    content         TEXT NOT NULL,
    run_id          TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    created_at      TEXT NOT NULL,
    UNIQUE (conversation_id, sequence),
    UNIQUE (run_id, kind)
) STRICT;

CREATE TABLE provider_attempts (
    id                  TEXT PRIMARY KEY NOT NULL,
    run_id              TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    sequence            INTEGER NOT NULL CHECK (sequence > 0),
    provider_id         TEXT NOT NULL REFERENCES providers(id) ON DELETE RESTRICT,
    model_id            TEXT NOT NULL REFERENCES models(id) ON DELETE RESTRICT,
    status              TEXT NOT NULL CHECK (status IN ('in_progress', 'completed')),
    conclusion          TEXT CHECK (conclusion IN ('success', 'failure', 'timed_out')),
    provider_request_id TEXT,
    usage_json          TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(usage_json)),
    error_message       TEXT,
    started_at          TEXT NOT NULL,
    completed_at        TEXT,
    UNIQUE (run_id, sequence),
    CHECK (
        (status = 'completed' AND conclusion IS NOT NULL AND completed_at IS NOT NULL)
        OR
        (status = 'in_progress' AND conclusion IS NULL AND completed_at IS NULL)
    )
) STRICT;

CREATE UNIQUE INDEX runs_one_in_progress_per_conversation
    ON runs(conversation_id)
    WHERE status = 'in_progress';

CREATE UNIQUE INDEX comments_source_run_unique
    ON comments(source_run_id)
    WHERE source_run_id IS NOT NULL;

CREATE INDEX provider_attempts_run_sequence_idx
    ON provider_attempts(run_id, sequence);
