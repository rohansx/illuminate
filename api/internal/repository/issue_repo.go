package repository

import (
	"context"
	"fmt"
	"strings"

	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
	"github.com/rohansx/illuminate/api/internal/model"
)

type IssueRepo interface {
	GetByID(ctx context.Context, id uuid.UUID) (*model.Issue, error)
	GetFeed(ctx context.Context, filter model.FeedFilter, limit, offset int) ([]model.Issue, int, error)
	Search(ctx context.Context, query string, limit, offset int) ([]model.Issue, int, error)
	Upsert(ctx context.Context, issue *model.Issue) (*model.Issue, error)
	SetSkills(ctx context.Context, issueID uuid.UUID, skills []model.IssueSkill) error
	Count(ctx context.Context) (int, error)
}

type issueRepo struct {
	pool *pgxpool.Pool
}

func NewIssueRepo(pool *pgxpool.Pool) IssueRepo {
	return &issueRepo{pool: pool}
}

func (r *issueRepo) GetByID(ctx context.Context, id uuid.UUID) (*model.Issue, error) {
	issue := &model.Issue{Repo: &model.Repository{}}
	err := r.pool.QueryRow(ctx, `
		SELECT i.id, i.github_id, i.repo_id, i.number, i.title, i.body, i.summary,
			i.labels, i.difficulty, i.time_estimate, i.status, i.comment_count,
			i.freshness_score, i.created_at, i.indexed_at,
			r.id, r.github_id, r.owner, r.name, r.description, r.stars,
			r.primary_language, r.topics, r.has_contributing, r.health_score,
			r.last_commit_at, r.indexed_at
		FROM issues i
		JOIN repositories r ON r.id = i.repo_id
		WHERE i.id = $1`, id,
	).Scan(
		&issue.ID, &issue.GitHubID, &issue.RepoID, &issue.Number, &issue.Title,
		&issue.Body, &issue.Summary, &issue.Labels, &issue.Difficulty, &issue.TimeEstimate,
		&issue.Status, &issue.CommentCount, &issue.FreshnessScore, &issue.CreatedAt, &issue.IndexedAt,
		&issue.Repo.ID, &issue.Repo.GitHubID, &issue.Repo.Owner, &issue.Repo.Name,
		&issue.Repo.Description, &issue.Repo.Stars, &issue.Repo.PrimaryLanguage,
		&issue.Repo.Topics, &issue.Repo.HasContributing, &issue.Repo.HealthScore,
		&issue.Repo.LastCommitAt, &issue.Repo.IndexedAt,
	)
	if err != nil {
		if err == pgx.ErrNoRows {
			return nil, nil
		}
		return nil, fmt.Errorf("getting issue by id: %w", err)
	}

	skills, err := r.getSkills(ctx, issue.ID)
	if err != nil {
		return nil, err
	}
	issue.Skills = skills

	return issue, nil
}

func (r *issueRepo) GetFeed(ctx context.Context, filter model.FeedFilter, limit, offset int) ([]model.Issue, int, error) {
	var totalCount int
	var args []any
	var joins []string
	var conditions []string
	argN := 1

	conditions = append(conditions, "i.status = 'open'")

	if len(filter.Languages) > 0 {
		conditions = append(conditions, fmt.Sprintf(`EXISTS (
			SELECT 1 FROM issue_skills s WHERE s.issue_id = i.id AND s.language = ANY($%d)
		)`, argN))
		args = append(args, filter.Languages)
		argN++
	}

	if filter.Difficulty > 0 {
		conditions = append(conditions, fmt.Sprintf("i.difficulty = $%d", argN))
		args = append(args, filter.Difficulty)
		argN++
	}

	if filter.Category != "" {
		joins = append(joins, "JOIN repo_categories rc ON rc.repo_id = r.id")
		joins = append(joins, "JOIN categories c ON c.id = rc.category_id")
		conditions = append(conditions, fmt.Sprintf("c.slug = $%d", argN))
		args = append(args, filter.Category)
		argN++
	}

	joinClause := strings.Join(joins, "\n\t\t")
	whereClause := " WHERE " + strings.Join(conditions, " AND ")

	countQuery := `SELECT COUNT(DISTINCT i.id) FROM issues i
		JOIN repositories r ON r.id = i.repo_id
		` + joinClause + whereClause
	if err := r.pool.QueryRow(ctx, countQuery, args...).Scan(&totalCount); err != nil {
		return nil, 0, fmt.Errorf("counting issues: %w", err)
	}

	query := fmt.Sprintf(`
		SELECT DISTINCT i.id, i.github_id, i.repo_id, i.number, i.title, i.body, i.summary,
			i.labels, i.difficulty, i.time_estimate, i.status, i.comment_count,
			i.freshness_score, i.created_at, i.indexed_at,
			r.id, r.github_id, r.owner, r.name, r.description, r.stars,
			r.primary_language, r.topics, r.has_contributing, r.health_score,
			r.last_commit_at, r.indexed_at
		FROM issues i
		JOIN repositories r ON r.id = i.repo_id
		%s%s
		ORDER BY i.freshness_score DESC, i.created_at DESC
		LIMIT $%d OFFSET $%d`, joinClause, whereClause, argN, argN+1)
	args = append(args, limit, offset)

	rows, err := r.pool.Query(ctx, query, args...)
	if err != nil {
		return nil, 0, fmt.Errorf("querying feed: %w", err)
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
			return nil, 0, fmt.Errorf("scanning issue: %w", err)
		}
		issues = append(issues, issue)
	}

	return issues, totalCount, nil
}

func (r *issueRepo) Search(ctx context.Context, query string, limit, offset int) ([]model.Issue, int, error) {
	searchPattern := "%" + strings.ToLower(query) + "%"

	var totalCount int
	err := r.pool.QueryRow(ctx, `
		SELECT COUNT(*) FROM issues i
		WHERE i.status = 'open' AND (LOWER(i.title) LIKE $1 OR LOWER(i.body) LIKE $1)`,
		searchPattern,
	).Scan(&totalCount)
	if err != nil {
		return nil, 0, fmt.Errorf("counting search results: %w", err)
	}

	rows, err := r.pool.Query(ctx, `
		SELECT i.id, i.github_id, i.repo_id, i.number, i.title, i.body, i.summary,
			i.labels, i.difficulty, i.time_estimate, i.status, i.comment_count,
			i.freshness_score, i.created_at, i.indexed_at,
			r.id, r.github_id, r.owner, r.name, r.description, r.stars,
			r.primary_language, r.topics, r.has_contributing, r.health_score,
			r.last_commit_at, r.indexed_at
		FROM issues i
		JOIN repositories r ON r.id = i.repo_id
		WHERE i.status = 'open' AND (LOWER(i.title) LIKE $1 OR LOWER(i.body) LIKE $1)
		ORDER BY i.freshness_score DESC
		LIMIT $2 OFFSET $3`,
		searchPattern, limit, offset,
	)
	if err != nil {
		return nil, 0, fmt.Errorf("searching issues: %w", err)
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
			return nil, 0, fmt.Errorf("scanning search result: %w", err)
		}
		issues = append(issues, issue)
	}

	return issues, totalCount, nil
}

func (r *issueRepo) Upsert(ctx context.Context, issue *model.Issue) (*model.Issue, error) {
	err := r.pool.QueryRow(ctx, `
		INSERT INTO issues (github_id, repo_id, number, title, body, summary,
			labels, difficulty, time_estimate, status, comment_count, freshness_score, indexed_at)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW())
		ON CONFLICT (github_id, repo_id) DO UPDATE SET
			title = EXCLUDED.title,
			body = EXCLUDED.body,
			summary = EXCLUDED.summary,
			labels = EXCLUDED.labels,
			difficulty = EXCLUDED.difficulty,
			time_estimate = EXCLUDED.time_estimate,
			status = EXCLUDED.status,
			comment_count = EXCLUDED.comment_count,
			freshness_score = EXCLUDED.freshness_score,
			indexed_at = NOW()
		RETURNING id, github_id, repo_id, number, title, body, summary,
			labels, difficulty, time_estimate, status, comment_count,
			freshness_score, created_at, indexed_at`,
		issue.GitHubID, issue.RepoID, issue.Number, issue.Title, issue.Body,
		issue.Summary, issue.Labels, issue.Difficulty, issue.TimeEstimate,
		issue.Status, issue.CommentCount, issue.FreshnessScore,
	).Scan(
		&issue.ID, &issue.GitHubID, &issue.RepoID, &issue.Number, &issue.Title,
		&issue.Body, &issue.Summary, &issue.Labels, &issue.Difficulty, &issue.TimeEstimate,
		&issue.Status, &issue.CommentCount, &issue.FreshnessScore, &issue.CreatedAt, &issue.IndexedAt,
	)
	if err != nil {
		return nil, fmt.Errorf("upserting issue: %w", err)
	}
	return issue, nil
}

func (r *issueRepo) SetSkills(ctx context.Context, issueID uuid.UUID, skills []model.IssueSkill) error {
	tx, err := r.pool.Begin(ctx)
	if err != nil {
		return fmt.Errorf("beginning tx: %w", err)
	}
	defer tx.Rollback(ctx)

	_, err = tx.Exec(ctx, `DELETE FROM issue_skills WHERE issue_id = $1`, issueID)
	if err != nil {
		return fmt.Errorf("deleting old skills: %w", err)
	}

	for _, s := range skills {
		_, err = tx.Exec(ctx, `
			INSERT INTO issue_skills (issue_id, language, framework)
			VALUES ($1, $2, $3)
			ON CONFLICT (issue_id, language, framework) DO NOTHING`,
			issueID, s.Language, s.Framework,
		)
		if err != nil {
			return fmt.Errorf("inserting skill: %w", err)
		}
	}

	return tx.Commit(ctx)
}

func (r *issueRepo) getSkills(ctx context.Context, issueID uuid.UUID) ([]model.IssueSkill, error) {
	rows, err := r.pool.Query(ctx, `
		SELECT language, framework FROM issue_skills WHERE issue_id = $1`, issueID,
	)
	if err != nil {
		return nil, fmt.Errorf("querying issue skills: %w", err)
	}
	defer rows.Close()

	var skills []model.IssueSkill
	for rows.Next() {
		var s model.IssueSkill
		if err := rows.Scan(&s.Language, &s.Framework); err != nil {
			return nil, fmt.Errorf("scanning issue skill: %w", err)
		}
		skills = append(skills, s)
	}
	return skills, nil
}

func (r *issueRepo) Count(ctx context.Context) (int, error) {
	var count int
	err := r.pool.QueryRow(ctx, `SELECT COUNT(*) FROM issues`).Scan(&count)
	if err != nil {
		return 0, fmt.Errorf("counting issues: %w", err)
	}
	return count, nil
}
