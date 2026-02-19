-- migrate:up
CREATE TABLE contributions (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id          UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    github_pr_id     BIGINT NOT NULL,
    repo_owner       VARCHAR(255) NOT NULL,
    repo_name        VARCHAR(255) NOT NULL,
    pr_number        INT NOT NULL,
    pr_title         TEXT NOT NULL,
    pr_url           TEXT NOT NULL,
    pr_state         VARCHAR(20) NOT NULL DEFAULT 'merged',
    language         VARCHAR(100) NOT NULL DEFAULT '',
    labels           TEXT[] NOT NULL DEFAULT '{}',
    merged_at        TIMESTAMPTZ,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    synced_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, github_pr_id)
);

CREATE INDEX idx_contributions_user_id ON contributions(user_id);
CREATE INDEX idx_contributions_merged_at ON contributions(merged_at);
CREATE INDEX idx_contributions_repo ON contributions(repo_owner, repo_name);

ALTER TABLE users ADD COLUMN contributions_synced_at TIMESTAMPTZ;

-- migrate:down
ALTER TABLE users DROP COLUMN IF EXISTS contributions_synced_at;
DROP TABLE IF EXISTS contributions;
