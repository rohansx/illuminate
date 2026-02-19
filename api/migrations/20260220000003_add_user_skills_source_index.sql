-- migrate:up
CREATE INDEX idx_user_skills_source ON user_skills(user_id, source);

-- migrate:down
DROP INDEX IF EXISTS idx_user_skills_source;
