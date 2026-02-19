-- migrate:up
ALTER TABLE users ADD COLUMN email VARCHAR(255) NOT NULL DEFAULT '';

-- migrate:down
ALTER TABLE users DROP COLUMN IF EXISTS email;
