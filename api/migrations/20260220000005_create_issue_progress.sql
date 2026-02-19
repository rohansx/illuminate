-- migrate:up
CREATE TABLE issue_progress (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    issue_id UUID NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    status VARCHAR(20) NOT NULL DEFAULT 'interested',
    notes TEXT[] NOT NULL DEFAULT '{}',
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, issue_id)
);

CREATE INDEX idx_issue_progress_user ON issue_progress(user_id);

-- migrate:down
DROP TABLE IF EXISTS issue_progress;
