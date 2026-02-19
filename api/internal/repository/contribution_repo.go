package repository

import (
	"context"
	"fmt"
	"time"

	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
	"github.com/rohansx/illuminate/api/internal/model"
)

type ContributionRepo interface {
	Upsert(ctx context.Context, c *model.Contribution) error
	GetByUserID(ctx context.Context, userID uuid.UUID, limit, offset int) (*model.ContributionFeed, error)
	GetProjectGroups(ctx context.Context, userID uuid.UUID) ([]model.ProjectGroup, error)
	GetStats(ctx context.Context, userID uuid.UUID) (*model.PortfolioStats, error)
	GetByUsername(ctx context.Context, username string, limit int) ([]model.Contribution, error)
	GetProjectGroupsByUsername(ctx context.Context, username string, limit int) ([]model.ProjectGroup, error)
	GetStatsByUsername(ctx context.Context, username string) (*model.PortfolioStats, error)
}

type contributionRepo struct {
	pool *pgxpool.Pool
}

func NewContributionRepo(pool *pgxpool.Pool) ContributionRepo {
	return &contributionRepo{pool: pool}
}

func (r *contributionRepo) Upsert(ctx context.Context, c *model.Contribution) error {
	_, err := r.pool.Exec(ctx, `
		INSERT INTO contributions (user_id, github_pr_id, repo_owner, repo_name, pr_number, pr_title, pr_url, pr_state, language, labels, merged_at, synced_at)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW())
		ON CONFLICT (user_id, github_pr_id) DO UPDATE SET
			pr_title = EXCLUDED.pr_title,
			pr_state = EXCLUDED.pr_state,
			language = EXCLUDED.language,
			labels = EXCLUDED.labels,
			merged_at = EXCLUDED.merged_at,
			synced_at = NOW()`,
		c.UserID, c.GitHubPRID, c.RepoOwner, c.RepoName, c.PRNumber, c.PRTitle, c.PRURL, c.PRState, c.Language, c.Labels, c.MergedAt,
	)
	if err != nil {
		return fmt.Errorf("upserting contribution: %w", err)
	}
	return nil
}

func (r *contributionRepo) GetByUserID(ctx context.Context, userID uuid.UUID, limit, offset int) (*model.ContributionFeed, error) {
	var totalCount int
	if err := r.pool.QueryRow(ctx, `SELECT COUNT(*) FROM contributions WHERE user_id = $1`, userID).Scan(&totalCount); err != nil {
		return nil, fmt.Errorf("counting contributions: %w", err)
	}

	rows, err := r.pool.Query(ctx, `
		SELECT id, user_id, github_pr_id, repo_owner, repo_name, pr_number, pr_title, pr_url, pr_state, language, labels, merged_at, created_at, synced_at
		FROM contributions WHERE user_id = $1
		ORDER BY COALESCE(merged_at, created_at) DESC
		LIMIT $2 OFFSET $3`, userID, limit, offset,
	)
	if err != nil {
		return nil, fmt.Errorf("querying contributions: %w", err)
	}
	defer rows.Close()

	var contributions []model.Contribution
	for rows.Next() {
		var c model.Contribution
		if err := rows.Scan(&c.ID, &c.UserID, &c.GitHubPRID, &c.RepoOwner, &c.RepoName, &c.PRNumber, &c.PRTitle, &c.PRURL, &c.PRState, &c.Language, &c.Labels, &c.MergedAt, &c.CreatedAt, &c.SyncedAt); err != nil {
			return nil, fmt.Errorf("scanning contribution: %w", err)
		}
		contributions = append(contributions, c)
	}

	page := 1
	if limit > 0 {
		page = (offset / limit) + 1
	}

	return &model.ContributionFeed{
		Contributions: contributions,
		TotalCount:    totalCount,
		Page:          page,
		PerPage:       limit,
	}, nil
}

func (r *contributionRepo) GetProjectGroups(ctx context.Context, userID uuid.UUID) ([]model.ProjectGroup, error) {
	return r.queryProjectGroups(ctx, `
		SELECT repo_owner, repo_name, COALESCE(MAX(language), '') as language, COUNT(*) as pr_count, MAX(COALESCE(merged_at, created_at)) as latest_at
		FROM contributions WHERE user_id = $1
		GROUP BY repo_owner, repo_name
		ORDER BY pr_count DESC`, userID,
	)
}

func (r *contributionRepo) GetProjectGroupsByUsername(ctx context.Context, username string, limit int) ([]model.ProjectGroup, error) {
	return r.queryProjectGroups(ctx, `
		SELECT c.repo_owner, c.repo_name, COALESCE(MAX(c.language), '') as language, COUNT(*) as pr_count, MAX(COALESCE(c.merged_at, c.created_at)) as latest_at
		FROM contributions c
		JOIN users u ON u.id = c.user_id
		WHERE u.github_username = $1
		GROUP BY c.repo_owner, c.repo_name
		ORDER BY pr_count DESC
		LIMIT $2`, username, limit,
	)
}

func (r *contributionRepo) queryProjectGroups(ctx context.Context, query string, args ...any) ([]model.ProjectGroup, error) {
	rows, err := r.pool.Query(ctx, query, args...)
	if err != nil {
		return nil, fmt.Errorf("querying project groups: %w", err)
	}
	defer rows.Close()

	var groups []model.ProjectGroup
	for rows.Next() {
		var g model.ProjectGroup
		if err := rows.Scan(&g.RepoOwner, &g.RepoName, &g.Language, &g.PRCount, &g.LatestAt); err != nil {
			return nil, fmt.Errorf("scanning project group: %w", err)
		}
		groups = append(groups, g)
	}
	return groups, nil
}

func (r *contributionRepo) GetStats(ctx context.Context, userID uuid.UUID) (*model.PortfolioStats, error) {
	return r.queryStats(ctx, userID,
		`SELECT COUNT(*) as total_prs,
			COUNT(DISTINCT repo_owner || '/' || repo_name) as total_repos,
			MIN(COALESCE(merged_at, created_at)) as first_contrib,
			MAX(COALESCE(merged_at, created_at)) as latest_contrib
		FROM contributions WHERE user_id = $1`,
		`SELECT language, COUNT(*) FROM contributions WHERE user_id = $1 AND language != '' GROUP BY language ORDER BY COUNT(*) DESC`,
		`SELECT DISTINCT DATE(COALESCE(merged_at, created_at)) as d FROM contributions WHERE user_id = $1 ORDER BY d DESC`,
	)
}

func (r *contributionRepo) GetStatsByUsername(ctx context.Context, username string) (*model.PortfolioStats, error) {
	return r.queryStats(ctx, username,
		`SELECT COUNT(*) as total_prs,
			COUNT(DISTINCT c.repo_owner || '/' || c.repo_name) as total_repos,
			MIN(COALESCE(c.merged_at, c.created_at)) as first_contrib,
			MAX(COALESCE(c.merged_at, c.created_at)) as latest_contrib
		FROM contributions c JOIN users u ON u.id = c.user_id
		WHERE u.github_username = $1`,
		`SELECT c.language, COUNT(*) FROM contributions c JOIN users u ON u.id = c.user_id WHERE u.github_username = $1 AND c.language != '' GROUP BY c.language ORDER BY COUNT(*) DESC`,
		`SELECT DISTINCT DATE(COALESCE(c.merged_at, c.created_at)) as d FROM contributions c JOIN users u ON u.id = c.user_id WHERE u.github_username = $1 ORDER BY d DESC`,
	)
}

func (r *contributionRepo) queryStats(ctx context.Context, arg any, aggregateQuery, langQuery, streakQuery string) (*model.PortfolioStats, error) {
	stats := &model.PortfolioStats{
		Languages: make(map[string]int),
	}

	// Aggregate counts
	err := r.pool.QueryRow(ctx, aggregateQuery, arg).Scan(
		&stats.TotalPRs, &stats.TotalRepos, &stats.FirstContrib, &stats.LatestContrib,
	)
	if err != nil {
		return nil, fmt.Errorf("querying contribution stats: %w", err)
	}

	// Language breakdown
	langRows, err := r.pool.Query(ctx, langQuery, arg)
	if err != nil {
		return nil, fmt.Errorf("querying language stats: %w", err)
	}
	defer langRows.Close()

	for langRows.Next() {
		var lang string
		var count int
		if err := langRows.Scan(&lang, &count); err != nil {
			return nil, fmt.Errorf("scanning language stat: %w", err)
		}
		stats.Languages[lang] = count
	}

	// Streak calculation from ordered dates
	streakRows, err := r.pool.Query(ctx, streakQuery, arg)
	if err != nil {
		return nil, fmt.Errorf("querying streak dates: %w", err)
	}
	defer streakRows.Close()

	var dates []time.Time
	for streakRows.Next() {
		var d time.Time
		if err := streakRows.Scan(&d); err != nil {
			return nil, fmt.Errorf("scanning streak date: %w", err)
		}
		dates = append(dates, d)
	}

	stats.CurrentStreak, stats.LongestStreak = calculateStreaks(dates)

	return stats, nil
}

// calculateStreaks computes current and longest streaks from a desc-sorted list of unique dates.
func calculateStreaks(dates []time.Time) (current, longest int) {
	if len(dates) == 0 {
		return 0, 0
	}

	today := time.Now().UTC().Truncate(24 * time.Hour)
	current = 0
	longest = 0
	streak := 1

	// dates are desc-sorted; check if the most recent date is today or yesterday
	first := dates[0].UTC().Truncate(24 * time.Hour)
	diff := today.Sub(first)
	if diff > 24*time.Hour {
		// Most recent contribution is older than yesterday, current streak is 0
		current = 0
	} else {
		current = 1
	}

	for i := 1; i < len(dates); i++ {
		prev := dates[i-1].UTC().Truncate(24 * time.Hour)
		curr := dates[i].UTC().Truncate(24 * time.Hour)
		gap := prev.Sub(curr)

		if gap == 24*time.Hour {
			streak++
			if current > 0 && i <= current {
				current = streak
			}
		} else {
			if streak > longest {
				longest = streak
			}
			streak = 1
		}
	}

	if streak > longest {
		longest = streak
	}
	if current > longest {
		current = longest
	}

	return current, longest
}

func (r *contributionRepo) GetByUsername(ctx context.Context, username string, limit int) ([]model.Contribution, error) {
	rows, err := r.pool.Query(ctx, `
		SELECT c.id, c.user_id, c.github_pr_id, c.repo_owner, c.repo_name, c.pr_number, c.pr_title, c.pr_url, c.pr_state, c.language, c.labels, c.merged_at, c.created_at, c.synced_at
		FROM contributions c
		JOIN users u ON u.id = c.user_id
		WHERE u.github_username = $1
		ORDER BY COALESCE(c.merged_at, c.created_at) DESC
		LIMIT $2`, username, limit,
	)
	if err != nil {
		return nil, fmt.Errorf("querying contributions by username: %w", err)
	}
	defer rows.Close()

	var contributions []model.Contribution
	for rows.Next() {
		var c model.Contribution
		if err := rows.Scan(&c.ID, &c.UserID, &c.GitHubPRID, &c.RepoOwner, &c.RepoName, &c.PRNumber, &c.PRTitle, &c.PRURL, &c.PRState, &c.Language, &c.Labels, &c.MergedAt, &c.CreatedAt, &c.SyncedAt); err != nil {
			return nil, fmt.Errorf("scanning contribution: %w", err)
		}
		contributions = append(contributions, c)
	}
	return contributions, nil
}
