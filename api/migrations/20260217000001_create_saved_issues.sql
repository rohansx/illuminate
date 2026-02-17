-- migrate:up
CREATE TABLE saved_issues (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    issue_id   UUID NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, issue_id)
);

CREATE INDEX idx_saved_issues_user_id ON saved_issues(user_id);
CREATE INDEX idx_saved_issues_issue_id ON saved_issues(issue_id);

-- migrate:down
DROP TABLE IF EXISTS saved_issues;
