-- migrate:up
CREATE TABLE issues (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    github_id       BIGINT NOT NULL,
    repo_id         UUID NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    number          INTEGER NOT NULL,
    title           TEXT NOT NULL,
    body            TEXT NOT NULL DEFAULT '',
    summary         TEXT NOT NULL DEFAULT '',
    labels          TEXT[] NOT NULL DEFAULT '{}',
    difficulty      SMALLINT NOT NULL DEFAULT 0,
    time_estimate   VARCHAR(50) NOT NULL DEFAULT '',
    status          VARCHAR(20) NOT NULL DEFAULT 'open',
    comment_count   INTEGER NOT NULL DEFAULT 0,
    freshness_score REAL NOT NULL DEFAULT 0.0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    indexed_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(github_id, repo_id)
);

CREATE INDEX idx_issues_repo_id ON issues(repo_id);
CREATE INDEX idx_issues_status ON issues(status);
CREATE INDEX idx_issues_freshness ON issues(freshness_score DESC);
CREATE INDEX idx_issues_labels ON issues USING GIN(labels);

-- migrate:down
DROP TABLE IF EXISTS issues;
