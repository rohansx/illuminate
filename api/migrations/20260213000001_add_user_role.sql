-- migrate:up
ALTER TABLE users ADD COLUMN role VARCHAR(20) NOT NULL DEFAULT 'user';

-- migrate:down
ALTER TABLE users DROP COLUMN role;
