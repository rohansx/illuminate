-- migrate:up
-- Add tags array to repositories
ALTER TABLE repositories ADD COLUMN tags TEXT[] DEFAULT '{}';

-- Add difficulty level (beginner, intermediate, advanced)
ALTER TABLE repositories ADD COLUMN difficulty_level VARCHAR(20) DEFAULT 'intermediate';

-- Add activity status
ALTER TABLE repositories ADD COLUMN activity_status VARCHAR(20) DEFAULT 'active';

-- Create categories table
CREATE TABLE categories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(50) NOT NULL UNIQUE,
    slug VARCHAR(50) NOT NULL UNIQUE,
    description TEXT,
    icon VARCHAR(50),
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Create repo_categories junction table
CREATE TABLE repo_categories (
    repo_id UUID NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    category_id UUID NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    PRIMARY KEY (repo_id, category_id),
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Insert default categories
INSERT INTO categories (name, slug, description, icon) VALUES
    ('Web Development', 'web', 'Web applications, frameworks, and tools', 'globe'),
    ('Mobile', 'mobile', 'iOS, Android, React Native, Flutter', 'smartphone'),
    ('DevOps & Infrastructure', 'devops', 'CI/CD, containers, orchestration', 'server'),
    ('AI & Machine Learning', 'ai-ml', 'ML frameworks, NLP, computer vision', 'brain'),
    ('CLI Tools', 'cli', 'Command-line utilities and tools', 'terminal'),
    ('Libraries & Frameworks', 'libraries', 'Reusable code libraries', 'package'),
    ('Data & Databases', 'data', 'Databases, data processing, analytics', 'database'),
    ('Security', 'security', 'Security tools and frameworks', 'shield'),
    ('Testing', 'testing', 'Testing frameworks and tools', 'check-circle'),
    ('Developer Tools', 'devtools', 'IDEs, editors, productivity tools', 'tool');

-- Create indexes
CREATE INDEX idx_repositories_tags ON repositories USING GIN(tags);
CREATE INDEX idx_repositories_difficulty ON repositories(difficulty_level);
CREATE INDEX idx_repositories_activity ON repositories(activity_status);
CREATE INDEX idx_repo_categories_repo ON repo_categories(repo_id);
CREATE INDEX idx_repo_categories_category ON repo_categories(category_id);

-- migrate:down
DROP INDEX IF EXISTS idx_repo_categories_category;
DROP INDEX IF EXISTS idx_repo_categories_repo;
DROP INDEX IF EXISTS idx_repositories_activity;
DROP INDEX IF EXISTS idx_repositories_difficulty;
DROP INDEX IF EXISTS idx_repositories_tags;
DROP TABLE IF EXISTS repo_categories;
DROP TABLE IF EXISTS categories;
ALTER TABLE repositories DROP COLUMN IF EXISTS activity_status;
ALTER TABLE repositories DROP COLUMN IF EXISTS difficulty_level;
ALTER TABLE repositories DROP COLUMN IF EXISTS tags;
