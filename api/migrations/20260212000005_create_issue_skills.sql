-- migrate:up
CREATE TABLE issue_skills (
    id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    issue_id  UUID NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    language  VARCHAR(100) NOT NULL,
    framework VARCHAR(100) NOT NULL DEFAULT '',
    UNIQUE(issue_id, language, framework)
);

CREATE INDEX idx_issue_skills_issue_id ON issue_skills(issue_id);
CREATE INDEX idx_issue_skills_language ON issue_skills(language);

-- migrate:down
DROP TABLE IF EXISTS issue_skills;
