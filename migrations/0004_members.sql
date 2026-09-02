CREATE TABLE providers (
    id              TEXT PRIMARY KEY NOT NULL,
    name            TEXT NOT NULL COLLATE NOCASE UNIQUE,
    adapter         TEXT NOT NULL CHECK (adapter IN ('openai_responses', 'openai_compatible', 'anthropic_messages', 'google_gemini')),
    base_url        TEXT NOT NULL,
    credential_ref  TEXT NOT NULL,
    enabled         INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
) STRICT;

CREATE TABLE models (
    id              TEXT PRIMARY KEY NOT NULL,
    provider_id     TEXT NOT NULL REFERENCES providers(id) ON DELETE RESTRICT,
    model_name      TEXT NOT NULL,
    display_name    TEXT NOT NULL,
    capabilities    TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(capabilities)),
    limits_json     TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(limits_json)),
    defaults_json   TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(defaults_json)),
    enabled         INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    UNIQUE (provider_id, model_name)
) STRICT;

CREATE TABLE ai_profiles (
    principal_id            TEXT PRIMARY KEY NOT NULL REFERENCES principals(id) ON DELETE CASCADE,
    identity_prompt_version INTEGER NOT NULL CHECK (identity_prompt_version > 0),
    default_model_id        TEXT NOT NULL REFERENCES models(id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE ai_prompt_versions (
    ai_principal_id       TEXT NOT NULL REFERENCES ai_profiles(principal_id) ON DELETE CASCADE,
    version               INTEGER NOT NULL CHECK (version > 0),
    prompt                TEXT NOT NULL,
    created_by_principal_id TEXT NOT NULL REFERENCES principals(id) ON DELETE RESTRICT,
    created_at            TEXT NOT NULL,
    PRIMARY KEY (ai_principal_id, version)
) STRICT;

CREATE TABLE teams (
    id                    TEXT PRIMARY KEY NOT NULL,
    topic_id              TEXT NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
    handle                TEXT NOT NULL COLLATE NOCASE,
    display_name          TEXT NOT NULL,
    created_by_principal_id TEXT NOT NULL REFERENCES principals(id) ON DELETE RESTRICT,
    created_at            TEXT NOT NULL,
    updated_at            TEXT NOT NULL,
    UNIQUE (topic_id, handle)
) STRICT;

CREATE TABLE team_members (
    team_id       TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    principal_id  TEXT NOT NULL REFERENCES principals(id) ON DELETE RESTRICT,
    added_by_principal_id TEXT NOT NULL REFERENCES principals(id) ON DELETE RESTRICT,
    created_at    TEXT NOT NULL,
    PRIMARY KEY (team_id, principal_id)
) STRICT;

CREATE INDEX teams_topic_idx ON teams(topic_id, handle);
CREATE INDEX team_members_principal_idx ON team_members(principal_id);
