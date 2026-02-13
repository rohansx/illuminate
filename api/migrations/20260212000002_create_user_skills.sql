-- migrate:up
CREATE TABLE user_skills (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    language    VARCHAR(100) NOT NULL,
    proficiency REAL NOT NULL DEFAULT 0.0,
    source      VARCHAR(50) NOT NULL DEFAULT 'github',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, language)
);

CREATE INDEX idx_user_skills_user_id ON user_skills(user_id);

-- migrate:down
DROP TABLE IF EXISTS user_skills;
