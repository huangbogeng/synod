PRAGMA foreign_keys = ON;

CREATE TABLE principals (
    id          TEXT PRIMARY KEY NOT NULL,
    kind        TEXT NOT NULL CHECK (kind IN ('human', 'ai', 'caller', 'system')),
    handle      TEXT NOT NULL COLLATE NOCASE UNIQUE,
    display_name TEXT NOT NULL,
    active      INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    created_at  TEXT NOT NULL
) STRICT;

CREATE TABLE topics (
    id                    TEXT PRIMARY KEY NOT NULL,
    topic_key             TEXT NOT NULL COLLATE NOCASE UNIQUE,
    title                 TEXT NOT NULL,
    description           TEXT NOT NULL DEFAULT '',
    revision              INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    next_issue_number     INTEGER NOT NULL DEFAULT 1 CHECK (next_issue_number > 0),
    next_proposal_number  INTEGER NOT NULL DEFAULT 1 CHECK (next_proposal_number > 0),
    created_at            TEXT NOT NULL,
    updated_at            TEXT NOT NULL
) STRICT;

CREATE TABLE topic_memberships (
    topic_id       TEXT NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
    principal_id   TEXT NOT NULL REFERENCES principals(id) ON DELETE RESTRICT,
    role           TEXT NOT NULL CHECK (role IN ('read', 'contribute', 'write')),
    created_at     TEXT NOT NULL,
    PRIMARY KEY (topic_id, principal_id)
) STRICT;

CREATE TABLE topic_items (
    id                    TEXT PRIMARY KEY NOT NULL,
    topic_id              TEXT NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
    number                INTEGER NOT NULL CHECK (number > 0),
    kind                  TEXT NOT NULL CHECK (kind IN ('issue', 'proposal')),
    title                 TEXT NOT NULL,
    author_principal_id   TEXT NOT NULL REFERENCES principals(id) ON DELETE RESTRICT,
    revision              INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    created_at            TEXT NOT NULL,
    updated_at            TEXT NOT NULL,
    UNIQUE (topic_id, kind, number)
) STRICT;

CREATE INDEX topic_items_topic_updated_idx ON topic_items(topic_id, updated_at DESC);

CREATE TABLE issues (
    item_id                 TEXT PRIMARY KEY NOT NULL REFERENCES topic_items(id) ON DELETE CASCADE,
    type_key                TEXT NOT NULL,
    state                   TEXT NOT NULL CHECK (state IN ('open', 'closed')),
    body                    TEXT NOT NULL DEFAULT '',
    parent_issue_item_id    TEXT REFERENCES issues(item_id) ON DELETE RESTRICT,
    closed_by_principal_id  TEXT REFERENCES principals(id) ON DELETE RESTRICT,
    closed_at               TEXT
) STRICT;

CREATE TABLE proposals (
    item_id                    TEXT PRIMARY KEY NOT NULL REFERENCES topic_items(id) ON DELETE CASCADE,
    state                      TEXT NOT NULL CHECK (state IN ('draft', 'open', 'merged', 'closed')),
    body                       TEXT NOT NULL DEFAULT '',
    current_proposal_revision  INTEGER NOT NULL DEFAULT 1 CHECK (current_proposal_revision > 0),
    base_topic_revision_id     TEXT,
    merged_by_principal_id     TEXT REFERENCES principals(id) ON DELETE RESTRICT,
    merged_topic_revision_id   TEXT,
    merged_at                  TEXT,
    CHECK (
        (state = 'merged' AND merged_by_principal_id IS NOT NULL AND merged_at IS NOT NULL)
        OR
        (state <> 'merged' AND merged_by_principal_id IS NULL AND merged_at IS NULL)
    )
) STRICT;

CREATE TABLE comments (
    id                    TEXT PRIMARY KEY NOT NULL,
    item_id               TEXT NOT NULL REFERENCES topic_items(id) ON DELETE CASCADE,
    author_principal_id   TEXT NOT NULL REFERENCES principals(id) ON DELETE RESTRICT,
    kind                  TEXT NOT NULL CHECK (kind IN ('discussion', 'direction', 'evidence', 'progress', 'result')),
    body                  TEXT NOT NULL,
    revision              INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    reply_to_comment_id   TEXT REFERENCES comments(id) ON DELETE RESTRICT,
    source_run_id         TEXT,
    tombstoned_at         TEXT,
    created_at            TEXT NOT NULL,
    updated_at            TEXT NOT NULL
) STRICT;

CREATE INDEX comments_item_created_idx ON comments(item_id, created_at);

CREATE TABLE comment_revisions (
    comment_id            TEXT NOT NULL REFERENCES comments(id) ON DELETE CASCADE,
    revision              INTEGER NOT NULL CHECK (revision > 0),
    body                  TEXT NOT NULL,
    editor_principal_id   TEXT NOT NULL REFERENCES principals(id) ON DELETE RESTRICT,
    created_at            TEXT NOT NULL,
    PRIMARY KEY (comment_id, revision)
) STRICT;

CREATE TABLE activity_events (
    id                    TEXT PRIMARY KEY NOT NULL,
    topic_id              TEXT NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
    sequence              INTEGER NOT NULL CHECK (sequence > 0),
    item_id               TEXT REFERENCES topic_items(id) ON DELETE CASCADE,
    event_type            TEXT NOT NULL,
    actor_principal_id    TEXT REFERENCES principals(id) ON DELETE RESTRICT,
    subject_type          TEXT NOT NULL,
    subject_id            TEXT NOT NULL,
    payload               TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(payload)),
    created_at            TEXT NOT NULL,
    UNIQUE (topic_id, sequence)
) STRICT;

CREATE INDEX activity_events_item_sequence_idx ON activity_events(item_id, sequence);

CREATE TABLE jobs (
    id              TEXT PRIMARY KEY NOT NULL,
    kind            TEXT NOT NULL,
    payload         TEXT NOT NULL CHECK (json_valid(payload)),
    state           TEXT NOT NULL CHECK (state IN ('queued', 'leased', 'completed')),
    available_at    TEXT NOT NULL,
    lease_token     TEXT,
    lease_expires_at TEXT,
    attempts        INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    outcome         TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    CHECK (
        (state = 'leased' AND lease_token IS NOT NULL AND lease_expires_at IS NOT NULL)
        OR state <> 'leased'
    )
) STRICT;

CREATE INDEX jobs_claim_idx ON jobs(state, available_at);
