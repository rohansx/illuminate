package repository

import (
	"context"
	"fmt"

	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
	"github.com/rohansx/illuminate/api/internal/model"
)

type RepoRepo interface {
	GetByID(ctx context.Context, id uuid.UUID) (*model.Repository, error)
	GetByGitHubID(ctx context.Context, githubID int64) (*model.Repository, error)
	Upsert(ctx context.Context, repo *model.Repository) (*model.Repository, error)
	GetAll(ctx context.Context) ([]model.Repository, error)
	GetAllGitHubIDs(ctx context.Context) ([]int64, error)
	Count(ctx context.Context) (int, error)
	Delete(ctx context.Context, id uuid.UUID) error
	ListWithIssueCounts(ctx context.Context, limit, offset int) ([]model.RepoListItem, int, error)
	UpdateMetadata(ctx context.Context, id uuid.UUID, tags []string, difficulty, activity string) error
	AssignCategory(ctx context.Context, repoID, categoryID uuid.UUID) error
	RemoveCategory(ctx context.Context, repoID, categoryID uuid.UUID) error
	GetCategories(ctx context.Context) ([]model.Category, error)
}

type repoRepo struct {
	pool *pgxpool.Pool
}

func NewRepoRepo(pool *pgxpool.Pool) RepoRepo {
	return &repoRepo{pool: pool}
}

func (r *repoRepo) GetByID(ctx context.Context, id uuid.UUID) (*model.Repository, error) {
	repo := &model.Repository{}
	err := r.pool.QueryRow(ctx, `
		SELECT id, github_id, owner, name, description, stars, primary_language,
			topics, has_contributing, health_score, last_commit_at, indexed_at,
			tags, difficulty_level, activity_status
		FROM repositories WHERE id = $1`, id,
	).Scan(
		&repo.ID, &repo.GitHubID, &repo.Owner, &repo.Name, &repo.Description,
		&repo.Stars, &repo.PrimaryLanguage, &repo.Topics, &repo.HasContributing,
		&repo.HealthScore, &repo.LastCommitAt, &repo.IndexedAt,
		&repo.Tags, &repo.DifficultyLevel, &repo.ActivityStatus,
	)
	if err != nil {
		if err == pgx.ErrNoRows {
			return nil, nil
		}
		return nil, fmt.Errorf("getting repo by id: %w", err)
	}

	// Load categories
	cats, err := r.getRepoCategories(ctx, repo.ID)
	if err == nil {
		repo.Categories = cats
	}

	return repo, nil
}

func (r *repoRepo) GetByGitHubID(ctx context.Context, githubID int64) (*model.Repository, error) {
	repo := &model.Repository{}
	err := r.pool.QueryRow(ctx, `
		SELECT id, github_id, owner, name, description, stars, primary_language,
			topics, has_contributing, health_score, last_commit_at, indexed_at,
			tags, difficulty_level, activity_status
		FROM repositories WHERE github_id = $1`, githubID,
	).Scan(
		&repo.ID, &repo.GitHubID, &repo.Owner, &repo.Name, &repo.Description,
		&repo.Stars, &repo.PrimaryLanguage, &repo.Topics, &repo.HasContributing,
		&repo.HealthScore, &repo.LastCommitAt, &repo.IndexedAt,
		&repo.Tags, &repo.DifficultyLevel, &repo.ActivityStatus,
	)
	if err != nil {
		if err == pgx.ErrNoRows {
			return nil, nil
		}
		return nil, fmt.Errorf("getting repo by github id: %w", err)
	}
	cats, _ := r.getRepoCategories(ctx, repo.ID)
	repo.Categories = cats
	return repo, nil
}

func (r *repoRepo) Upsert(ctx context.Context, repo *model.Repository) (*model.Repository, error) {
	err := r.pool.QueryRow(ctx, `
		INSERT INTO repositories (github_id, owner, name, description, stars, primary_language,
			topics, has_contributing, health_score, last_commit_at, indexed_at)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW())
		ON CONFLICT (github_id) DO UPDATE SET
			owner = EXCLUDED.owner,
			name = EXCLUDED.name,
			description = EXCLUDED.description,
			stars = EXCLUDED.stars,
			primary_language = EXCLUDED.primary_language,
			topics = EXCLUDED.topics,
			has_contributing = EXCLUDED.has_contributing,
			health_score = EXCLUDED.health_score,
			last_commit_at = EXCLUDED.last_commit_at,
			indexed_at = NOW()
		RETURNING id, github_id, owner, name, description, stars, primary_language,
			topics, has_contributing, health_score, last_commit_at, indexed_at,
			tags, difficulty_level, activity_status`,
		repo.GitHubID, repo.Owner, repo.Name, repo.Description, repo.Stars,
		repo.PrimaryLanguage, repo.Topics, repo.HasContributing, repo.HealthScore, repo.LastCommitAt,
	).Scan(
		&repo.ID, &repo.GitHubID, &repo.Owner, &repo.Name, &repo.Description,
		&repo.Stars, &repo.PrimaryLanguage, &repo.Topics, &repo.HasContributing,
		&repo.HealthScore, &repo.LastCommitAt, &repo.IndexedAt,
		&repo.Tags, &repo.DifficultyLevel, &repo.ActivityStatus,
	)
	if err != nil {
		return nil, fmt.Errorf("upserting repo: %w", err)
	}
	cats, _ := r.getRepoCategories(ctx, repo.ID)
	repo.Categories = cats
	return repo, nil
}

func (r *repoRepo) GetAll(ctx context.Context) ([]model.Repository, error) {
	rows, err := r.pool.Query(ctx, `
		SELECT id, github_id, owner, name, description, stars, primary_language,
			topics, has_contributing, health_score, last_commit_at, indexed_at,
			tags, difficulty_level, activity_status
		FROM repositories ORDER BY stars DESC`)
	if err != nil {
		return nil, fmt.Errorf("querying repos: %w", err)
	}
	defer rows.Close()

	var repos []model.Repository
	for rows.Next() {
		var repo model.Repository
		if err := rows.Scan(
			&repo.ID, &repo.GitHubID, &repo.Owner, &repo.Name, &repo.Description,
			&repo.Stars, &repo.PrimaryLanguage, &repo.Topics, &repo.HasContributing,
			&repo.HealthScore, &repo.LastCommitAt, &repo.IndexedAt,
			&repo.Tags, &repo.DifficultyLevel, &repo.ActivityStatus,
		); err != nil {
			return nil, fmt.Errorf("scanning repo: %w", err)
		}
		cats, _ := r.getRepoCategories(ctx, repo.ID)
		repo.Categories = cats
		repos = append(repos, repo)
	}
	return repos, nil
}

func (r *repoRepo) GetAllGitHubIDs(ctx context.Context) ([]int64, error) {
	rows, err := r.pool.Query(ctx, `SELECT github_id FROM repositories`)
	if err != nil {
		return nil, fmt.Errorf("querying github ids: %w", err)
	}
	defer rows.Close()

	var ids []int64
	for rows.Next() {
		var id int64
		if err := rows.Scan(&id); err != nil {
			return nil, fmt.Errorf("scanning github id: %w", err)
		}
		ids = append(ids, id)
	}
	return ids, nil
}

func (r *repoRepo) Count(ctx context.Context) (int, error) {
	var count int
	err := r.pool.QueryRow(ctx, `SELECT COUNT(*) FROM repositories`).Scan(&count)
	if err != nil {
		return 0, fmt.Errorf("counting repos: %w", err)
	}
	return count, nil
}

func (r *repoRepo) Delete(ctx context.Context, id uuid.UUID) error {
	_, err := r.pool.Exec(ctx, `DELETE FROM repositories WHERE id = $1`, id)
	if err != nil {
		return fmt.Errorf("deleting repo: %w", err)
	}
	return nil
}

func (r *repoRepo) ListWithIssueCounts(ctx context.Context, limit, offset int) ([]model.RepoListItem, int, error) {
	var totalCount int
	if err := r.pool.QueryRow(ctx, `SELECT COUNT(*) FROM repositories`).Scan(&totalCount); err != nil {
		return nil, 0, fmt.Errorf("counting repos: %w", err)
	}

	rows, err := r.pool.Query(ctx, `
		SELECT r.id, r.owner, r.name, r.stars, r.primary_language,
			COUNT(i.id) AS issue_count, r.indexed_at, r.tags, r.difficulty_level, r.activity_status
		FROM repositories r
		LEFT JOIN issues i ON i.repo_id = r.id
		GROUP BY r.id
		ORDER BY r.stars DESC
		LIMIT $1 OFFSET $2`, limit, offset,
	)
	if err != nil {
		return nil, 0, fmt.Errorf("listing repos: %w", err)
	}
	defer rows.Close()

	var repos []model.RepoListItem
	for rows.Next() {
		var repo model.RepoListItem
		if err := rows.Scan(&repo.ID, &repo.Owner, &repo.Name, &repo.Stars,
			&repo.PrimaryLanguage, &repo.IssueCount, &repo.IndexedAt,
			&repo.Tags, &repo.DifficultyLevel, &repo.ActivityStatus); err != nil {
			return nil, 0, fmt.Errorf("scanning repo: %w", err)
		}

		// Load categories for this repo
		cats, err := r.getRepoCategories(ctx, repo.ID)
		if err == nil {
			repo.Categories = cats
		}

		repos = append(repos, repo)
	}
	return repos, totalCount, nil
}

func (r *repoRepo) UpdateMetadata(ctx context.Context, id uuid.UUID, tags []string, difficulty, activity string) error {
	_, err := r.pool.Exec(ctx, `
		UPDATE repositories
		SET tags = $2, difficulty_level = $3, activity_status = $4
		WHERE id = $1`,
		id, tags, difficulty, activity,
	)
	if err != nil {
		return fmt.Errorf("updating repo metadata: %w", err)
	}
	return nil
}

func (r *repoRepo) AssignCategory(ctx context.Context, repoID, categoryID uuid.UUID) error {
	_, err := r.pool.Exec(ctx, `
		INSERT INTO repo_categories (repo_id, category_id)
		VALUES ($1, $2)
		ON CONFLICT (repo_id, category_id) DO NOTHING`,
		repoID, categoryID,
	)
	if err != nil {
		return fmt.Errorf("assigning category: %w", err)
	}
	return nil
}

func (r *repoRepo) RemoveCategory(ctx context.Context, repoID, categoryID uuid.UUID) error {
	_, err := r.pool.Exec(ctx, `
		DELETE FROM repo_categories
		WHERE repo_id = $1 AND category_id = $2`,
		repoID, categoryID,
	)
	if err != nil {
		return fmt.Errorf("removing category: %w", err)
	}
	return nil
}

func (r *repoRepo) GetCategories(ctx context.Context) ([]model.Category, error) {
	rows, err := r.pool.Query(ctx, `
		SELECT id, name, slug, description, icon
		FROM categories
		ORDER BY name`)
	if err != nil {
		return nil, fmt.Errorf("querying categories: %w", err)
	}
	defer rows.Close()

	var cats []model.Category
	for rows.Next() {
		var cat model.Category
		if err := rows.Scan(&cat.ID, &cat.Name, &cat.Slug, &cat.Description, &cat.Icon); err != nil {
			return nil, fmt.Errorf("scanning category: %w", err)
		}
		cats = append(cats, cat)
	}
	return cats, nil
}

func (r *repoRepo) getRepoCategories(ctx context.Context, repoID uuid.UUID) ([]model.Category, error) {
	rows, err := r.pool.Query(ctx, `
		SELECT c.id, c.name, c.slug, c.description, c.icon
		FROM categories c
		JOIN repo_categories rc ON rc.category_id = c.id
		WHERE rc.repo_id = $1
		ORDER BY c.name`, repoID,
	)
	if err != nil {
		return nil, fmt.Errorf("querying repo categories: %w", err)
	}
	defer rows.Close()

	var cats []model.Category
	for rows.Next() {
		var cat model.Category
		if err := rows.Scan(&cat.ID, &cat.Name, &cat.Slug, &cat.Description, &cat.Icon); err != nil {
			return nil, fmt.Errorf("scanning category: %w", err)
		}
		cats = append(cats, cat)
	}
	return cats, nil
}
