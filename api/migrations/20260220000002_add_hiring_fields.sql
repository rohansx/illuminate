-- migrate:up
ALTER TABLE repositories ADD COLUMN is_hiring BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE repositories ADD COLUMN hiring_url TEXT NOT NULL DEFAULT '';

CREATE INDEX idx_repositories_hiring ON repositories(is_hiring) WHERE is_hiring = TRUE;

-- migrate:down
DROP INDEX IF EXISTS idx_repositories_hiring;
ALTER TABLE repositories DROP COLUMN IF EXISTS hiring_url;
ALTER TABLE repositories DROP COLUMN IF EXISTS is_hiring;
