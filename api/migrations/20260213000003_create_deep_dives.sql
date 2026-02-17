-- migrate:up
CREATE TABLE deep_dives (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    issue_id          UUID NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    user_id           UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    issue_indexed_at  TIMESTAMPTZ NOT NULL,
    project_overview  TEXT NOT NULL DEFAULT '',
    issue_context     TEXT NOT NULL DEFAULT '',
    suggested_approach TEXT NOT NULL DEFAULT '',
    questions_to_ask  TEXT NOT NULL DEFAULT '',
    red_flags         TEXT NOT NULL DEFAULT '',
    model_used        VARCHAR(100) NOT NULL DEFAULT '',
    prompt_tokens     INTEGER NOT NULL DEFAULT 0,
    completion_tokens INTEGER NOT NULL DEFAULT 0,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(issue_id, user_id)
);

CREATE INDEX idx_deep_dives_issue_id ON deep_dives(issue_id);
CREATE INDEX idx_deep_dives_user_id ON deep_dives(user_id);

-- migrate:down
DROP TABLE IF EXISTS deep_dives;
