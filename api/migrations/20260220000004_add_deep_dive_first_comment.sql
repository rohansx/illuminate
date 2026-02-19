-- migrate:up
ALTER TABLE deep_dives ADD COLUMN first_comment TEXT NOT NULL DEFAULT '';

-- migrate:down
ALTER TABLE deep_dives DROP COLUMN IF EXISTS first_comment;
