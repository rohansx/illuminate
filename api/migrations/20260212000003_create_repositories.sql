-- migrate:up
CREATE TABLE repositories (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    github_id        BIGINT UNIQUE NOT NULL,
    owner            VARCHAR(255) NOT NULL,
    name             VARCHAR(255) NOT NULL,
    description      TEXT NOT NULL DEFAULT '',
    stars            INTEGER NOT NULL DEFAULT 0,
    primary_language VARCHAR(100) NOT NULL DEFAULT '',
    topics           TEXT[] NOT NULL DEFAULT '{}',
    has_contributing BOOLEAN NOT NULL DEFAULT FALSE,
    health_score     REAL NOT NULL DEFAULT 0.0,
    last_commit_at   TIMESTAMPTZ,
    indexed_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(owner, name)
);

CREATE INDEX idx_repositories_primary_language ON repositories(primary_language);
CREATE INDEX idx_repositories_health_score ON repositories(health_score);

-- migrate:down
DROP TABLE IF EXISTS repositories;
