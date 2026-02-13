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
			topics, has_contributing, health_score, last_commit_at, indexed_at
		FROM repositories WHERE id = $1`, id,
	).Scan(
		&repo.ID, &repo.GitHubID, &repo.Owner, &repo.Name, &repo.Description,
		&repo.Stars, &repo.PrimaryLanguage, &repo.Topics, &repo.HasContributing,
		&repo.HealthScore, &repo.LastCommitAt, &repo.IndexedAt,
	)
	if err != nil {
		if err == pgx.ErrNoRows {
			return nil, nil
		}
		return nil, fmt.Errorf("getting repo by id: %w", err)
	}
	return repo, nil
}

func (r *repoRepo) GetByGitHubID(ctx context.Context, githubID int64) (*model.Repository, error) {
	repo := &model.Repository{}
	err := r.pool.QueryRow(ctx, `
		SELECT id, github_id, owner, name, description, stars, primary_language,
			topics, has_contributing, health_score, last_commit_at, indexed_at
		FROM repositories WHERE github_id = $1`, githubID,
	).Scan(
		&repo.ID, &repo.GitHubID, &repo.Owner, &repo.Name, &repo.Description,
		&repo.Stars, &repo.PrimaryLanguage, &repo.Topics, &repo.HasContributing,
		&repo.HealthScore, &repo.LastCommitAt, &repo.IndexedAt,
	)
	if err != nil {
		if err == pgx.ErrNoRows {
			return nil, nil
		}
		return nil, fmt.Errorf("getting repo by github id: %w", err)
	}
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
			topics, has_contributing, health_score, last_commit_at, indexed_at`,
		repo.GitHubID, repo.Owner, repo.Name, repo.Description, repo.Stars,
		repo.PrimaryLanguage, repo.Topics, repo.HasContributing, repo.HealthScore, repo.LastCommitAt,
	).Scan(
		&repo.ID, &repo.GitHubID, &repo.Owner, &repo.Name, &repo.Description,
		&repo.Stars, &repo.PrimaryLanguage, &repo.Topics, &repo.HasContributing,
		&repo.HealthScore, &repo.LastCommitAt, &repo.IndexedAt,
	)
	if err != nil {
		return nil, fmt.Errorf("upserting repo: %w", err)
	}
	return repo, nil
}

func (r *repoRepo) GetAll(ctx context.Context) ([]model.Repository, error) {
	rows, err := r.pool.Query(ctx, `
		SELECT id, github_id, owner, name, description, stars, primary_language,
			topics, has_contributing, health_score, last_commit_at, indexed_at
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
		); err != nil {
			return nil, fmt.Errorf("scanning repo: %w", err)
		}
		repos = append(repos, repo)
	}
	return repos, nil
}
