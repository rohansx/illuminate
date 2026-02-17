package repository

import (
	"context"
	"fmt"

	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
	"github.com/rohansx/illuminate/api/internal/model"
)

type SavedIssueRepo interface {
	Save(ctx context.Context, userID, issueID uuid.UUID) error
	Unsave(ctx context.Context, userID, issueID uuid.UUID) error
	IsSaved(ctx context.Context, userID, issueID uuid.UUID) (bool, error)
	GetSavedIssues(ctx context.Context, userID uuid.UUID, limit, offset int) ([]model.Issue, int, error)
	GetSavedIssueIDs(ctx context.Context, userID uuid.UUID, issueIDs []uuid.UUID) ([]uuid.UUID, error)
}

type savedIssueRepo struct {
	pool *pgxpool.Pool
}

func NewSavedIssueRepo(pool *pgxpool.Pool) SavedIssueRepo {
	return &savedIssueRepo{pool: pool}
}

func (r *savedIssueRepo) Save(ctx context.Context, userID, issueID uuid.UUID) error {
	_, err := r.pool.Exec(ctx, `
		INSERT INTO saved_issues (user_id, issue_id)
		VALUES ($1, $2)
		ON CONFLICT (user_id, issue_id) DO NOTHING`, userID, issueID)
	if err != nil {
		return fmt.Errorf("saving issue: %w", err)
	}
	return nil
}

func (r *savedIssueRepo) Unsave(ctx context.Context, userID, issueID uuid.UUID) error {
	_, err := r.pool.Exec(ctx, `
		DELETE FROM saved_issues WHERE user_id = $1 AND issue_id = $2`, userID, issueID)
	if err != nil {
		return fmt.Errorf("unsaving issue: %w", err)
	}
	return nil
}

func (r *savedIssueRepo) IsSaved(ctx context.Context, userID, issueID uuid.UUID) (bool, error) {
	var exists bool
	err := r.pool.QueryRow(ctx, `
		SELECT EXISTS(SELECT 1 FROM saved_issues WHERE user_id = $1 AND issue_id = $2)`,
		userID, issueID).Scan(&exists)
	if err != nil {
		return false, fmt.Errorf("checking saved status: %w", err)
	}
	return exists, nil
}

func (r *savedIssueRepo) GetSavedIssues(ctx context.Context, userID uuid.UUID, limit, offset int) ([]model.Issue, int, error) {
	var totalCount int
	err := r.pool.QueryRow(ctx, `
		SELECT COUNT(*) FROM saved_issues WHERE user_id = $1`, userID).Scan(&totalCount)
	if err != nil {
		return nil, 0, fmt.Errorf("counting saved issues: %w", err)
	}

	rows, err := r.pool.Query(ctx, `
		SELECT i.id, i.github_id, i.repo_id, i.number, i.title, i.body, i.summary,
			i.labels, i.difficulty, i.time_estimate, i.status, i.comment_count,
			i.freshness_score, i.created_at, i.indexed_at,
			r.id, r.github_id, r.owner, r.name, r.description, r.stars,
			r.primary_language, r.topics, r.has_contributing, r.health_score,
			r.last_commit_at, r.indexed_at
		FROM saved_issues si
		JOIN issues i ON i.id = si.issue_id
		JOIN repositories r ON r.id = i.repo_id
		WHERE si.user_id = $1
		ORDER BY si.created_at DESC
		LIMIT $2 OFFSET $3`, userID, limit, offset)
	if err != nil {
		return nil, 0, fmt.Errorf("querying saved issues: %w", err)
	}
	defer rows.Close()

	var issues []model.Issue
	for rows.Next() {
		var issue model.Issue
		issue.Repo = &model.Repository{}
		if err := rows.Scan(
			&issue.ID, &issue.GitHubID, &issue.RepoID, &issue.Number, &issue.Title,
			&issue.Body, &issue.Summary, &issue.Labels, &issue.Difficulty, &issue.TimeEstimate,
			&issue.Status, &issue.CommentCount, &issue.FreshnessScore, &issue.CreatedAt, &issue.IndexedAt,
			&issue.Repo.ID, &issue.Repo.GitHubID, &issue.Repo.Owner, &issue.Repo.Name,
			&issue.Repo.Description, &issue.Repo.Stars, &issue.Repo.PrimaryLanguage,
			&issue.Repo.Topics, &issue.Repo.HasContributing, &issue.Repo.HealthScore,
			&issue.Repo.LastCommitAt, &issue.Repo.IndexedAt,
		); err != nil {
			return nil, 0, fmt.Errorf("scanning saved issue: %w", err)
		}
		issues = append(issues, issue)
	}

	return issues, totalCount, nil
}

func (r *savedIssueRepo) GetSavedIssueIDs(ctx context.Context, userID uuid.UUID, issueIDs []uuid.UUID) ([]uuid.UUID, error) {
	rows, err := r.pool.Query(ctx, `
		SELECT issue_id FROM saved_issues
		WHERE user_id = $1 AND issue_id = ANY($2)`, userID, issueIDs)
	if err != nil {
		return nil, fmt.Errorf("getting saved issue ids: %w", err)
	}
	defer rows.Close()

	var savedIDs []uuid.UUID
	for rows.Next() {
		var id uuid.UUID
		if err := rows.Scan(&id); err != nil {
			return nil, fmt.Errorf("scanning saved issue id: %w", err)
		}
		savedIDs = append(savedIDs, id)
	}
	return savedIDs, nil
}
