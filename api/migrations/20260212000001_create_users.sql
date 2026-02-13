-- migrate:up
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TABLE users (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    github_id        BIGINT UNIQUE NOT NULL,
    github_username  VARCHAR(255) UNIQUE NOT NULL,
    avatar_url       TEXT NOT NULL DEFAULT '',
    bio              TEXT NOT NULL DEFAULT '',
    access_token_enc BYTEA,
    comfort_level    VARCHAR(20) NOT NULL DEFAULT 'beginner',
    time_commitment  VARCHAR(50) NOT NULL DEFAULT '',
    goals            TEXT[] NOT NULL DEFAULT '{}',
    onboarding_done  BOOLEAN NOT NULL DEFAULT FALSE,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_github_id ON users(github_id);

-- migrate:down
DROP TABLE IF EXISTS users;
