CREATE TABLE notifications (
    id                     TEXT PRIMARY KEY NOT NULL,
    dispatch_id            TEXT NOT NULL REFERENCES dispatches(id) ON DELETE CASCADE,
    recipient_principal_id TEXT NOT NULL REFERENCES principals(id) ON DELETE RESTRICT,
    kind                   TEXT NOT NULL CHECK (kind IN ('mention')),
    read_at                TEXT,
    created_at             TEXT NOT NULL,
    UNIQUE (dispatch_id, recipient_principal_id)
) STRICT;

CREATE INDEX notifications_recipient_created_idx
    ON notifications(recipient_principal_id, created_at DESC);

CREATE TABLE conversations (
    id              TEXT PRIMARY KEY NOT NULL,
    topic_id        TEXT NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
    item_id         TEXT NOT NULL REFERENCES topic_items(id) ON DELETE CASCADE,
    ai_principal_id TEXT NOT NULL REFERENCES ai_profiles(principal_id) ON DELETE RESTRICT,
    context_epoch   INTEGER NOT NULL DEFAULT 1 CHECK (context_epoch > 0),
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    UNIQUE (item_id, ai_principal_id)
) STRICT;

CREATE TABLE runs (
    id                      TEXT PRIMARY KEY NOT NULL,
    dispatch_id             TEXT NOT NULL REFERENCES dispatches(id) ON DELETE CASCADE,
    topic_id                TEXT NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
    item_id                 TEXT NOT NULL REFERENCES topic_items(id) ON DELETE CASCADE,
    ai_principal_id         TEXT NOT NULL REFERENCES ai_profiles(principal_id) ON DELETE RESTRICT,
    conversation_id         TEXT NOT NULL REFERENCES conversations(id) ON DELETE RESTRICT,
    identity_prompt_version INTEGER NOT NULL CHECK (identity_prompt_version > 0),
    model_id                TEXT NOT NULL REFERENCES models(id) ON DELETE RESTRICT,
    context_snapshot_id     TEXT,
    status                  TEXT NOT NULL CHECK (status IN ('queued', 'in_progress', 'completed')),
    conclusion              TEXT CHECK (conclusion IN ('success', 'failure', 'cancelled', 'timed_out', 'skipped', 'neutral')),
    retry_of_run_id         TEXT REFERENCES runs(id) ON DELETE RESTRICT,
    created_at              TEXT NOT NULL,
    started_at              TEXT,
    completed_at            TEXT,
    UNIQUE (dispatch_id, ai_principal_id),
    CHECK (
        (status = 'completed' AND conclusion IS NOT NULL AND completed_at IS NOT NULL)
        OR
        (status <> 'completed' AND conclusion IS NULL AND completed_at IS NULL)
    )
) STRICT;

CREATE INDEX runs_conversation_created_idx ON runs(conversation_id, created_at);
CREATE INDEX runs_status_created_idx ON runs(status, created_at);

CREATE TABLE dispatch_targets (
    id               TEXT PRIMARY KEY NOT NULL,
    dispatch_id      TEXT NOT NULL REFERENCES dispatches(id) ON DELETE CASCADE,
    principal_id     TEXT REFERENCES principals(id) ON DELETE RESTRICT,
    target_handle    TEXT NOT NULL COLLATE NOCASE,
    principal_kind   TEXT CHECK (principal_kind IN ('human', 'ai', 'caller', 'system')),
    outcome          TEXT NOT NULL CHECK (outcome IN ('notified', 'queued', 'skipped')),
    notification_id  TEXT REFERENCES notifications(id) ON DELETE RESTRICT,
    run_id           TEXT REFERENCES runs(id) ON DELETE RESTRICT,
    skip_reason      TEXT,
    target_order     INTEGER NOT NULL CHECK (target_order >= 0),
    CHECK (
        (outcome = 'notified' AND principal_kind = 'human' AND notification_id IS NOT NULL AND run_id IS NULL AND skip_reason IS NULL)
        OR
        (outcome = 'queued' AND principal_kind = 'ai' AND notification_id IS NULL AND run_id IS NOT NULL AND skip_reason IS NULL)
        OR
        (outcome = 'skipped' AND notification_id IS NULL AND run_id IS NULL AND skip_reason IS NOT NULL)
    ),
    UNIQUE (dispatch_id, target_order)
) STRICT;

CREATE UNIQUE INDEX dispatch_targets_principal_unique
    ON dispatch_targets(dispatch_id, principal_id)
    WHERE principal_id IS NOT NULL;

CREATE UNIQUE INDEX dispatch_targets_unresolved_unique
    ON dispatch_targets(dispatch_id, target_handle)
    WHERE principal_id IS NULL;

CREATE TABLE dispatch_target_sources (
    target_id       TEXT NOT NULL REFERENCES dispatch_targets(id) ON DELETE CASCADE,
    mention_handle  TEXT NOT NULL COLLATE NOCASE,
    source_kind     TEXT NOT NULL CHECK (source_kind IN ('direct', 'team')),
    PRIMARY KEY (target_id, mention_handle)
) STRICT;
