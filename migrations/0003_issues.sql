ALTER TABLE topics ADD COLUMN next_event_sequence INTEGER NOT NULL DEFAULT 2
    CHECK (next_event_sequence > 0);

CREATE TABLE issue_types (
    type_key      TEXT PRIMARY KEY NOT NULL,
    display_name  TEXT NOT NULL,
    description   TEXT NOT NULL
) STRICT;

INSERT INTO issue_types(type_key, display_name, description) VALUES
    ('task', 'Task', 'A concrete piece of work with an expected outcome.'),
    ('bug', 'Bug', 'Observed behavior that differs from the expected behavior.'),
    ('feature', 'Feature', 'A proposed capability or user-facing behavior.'),
    ('code_audit', 'Code audit', 'A scoped review of code correctness, security, or quality.'),
    ('research', 'Research', 'A question or hypothesis requiring evidence and analysis.'),
    ('experiment', 'Experiment', 'A defined protocol for testing a hypothesis.'),
    ('decision', 'Decision', 'A choice between explicit options and constraints.');

CREATE TABLE dispatches (
    id                    TEXT PRIMARY KEY NOT NULL,
    topic_id              TEXT NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
    source_type           TEXT NOT NULL CHECK (source_type IN ('issue', 'proposal', 'comment')),
    source_id             TEXT NOT NULL,
    source_revision       INTEGER NOT NULL CHECK (source_revision > 0),
    author_principal_id   TEXT NOT NULL REFERENCES principals(id) ON DELETE RESTRICT,
    status                TEXT NOT NULL CHECK (status IN ('pending', 'dispatched', 'partially_dispatched', 'rejected')),
    created_at            TEXT NOT NULL,
    UNIQUE (source_type, source_id, source_revision)
) STRICT;

CREATE INDEX dispatches_topic_created_idx ON dispatches(topic_id, created_at);

CREATE TABLE dispatch_mentions (
    dispatch_id    TEXT NOT NULL REFERENCES dispatches(id) ON DELETE CASCADE,
    handle         TEXT NOT NULL COLLATE NOCASE,
    mention_order  INTEGER NOT NULL CHECK (mention_order >= 0),
    PRIMARY KEY (dispatch_id, handle),
    UNIQUE (dispatch_id, mention_order)
) STRICT;
