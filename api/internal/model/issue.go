package model

import (
	"time"

	"github.com/google/uuid"
)

type Issue struct {
	ID             uuid.UUID    `json:"id"`
	GitHubID       int64        `json:"github_id"`
	RepoID         uuid.UUID    `json:"repo_id"`
	Number         int          `json:"number"`
	Title          string       `json:"title"`
	Body           string       `json:"body"`
	Summary        string       `json:"summary"`
	Labels         []string     `json:"labels"`
	Difficulty     int          `json:"difficulty"`
	TimeEstimate   string       `json:"time_estimate"`
	Status         string       `json:"status"`
	CommentCount   int          `json:"comment_count"`
	FreshnessScore float32      `json:"freshness_score"`
	CreatedAt      time.Time    `json:"created_at"`
	IndexedAt      time.Time    `json:"indexed_at"`
	Repo           *Repository  `json:"repo,omitempty"`
	Skills         []IssueSkill `json:"skills,omitempty"`
	MatchScore     float64      `json:"match_score,omitempty"`
	MatchReasons   []string     `json:"match_reasons,omitempty"`
	IsSaved        bool         `json:"is_saved,omitempty"`
}

type IssueSkill struct {
	Language  string `json:"language"`
	Framework string `json:"framework,omitempty"`
}

type IssueFeed struct {
	Issues     []Issue `json:"issues"`
	TotalCount int     `json:"total_count"`
	Page       int     `json:"page"`
	PerPage    int     `json:"per_page"`
}

type FeedFilter struct {
	Languages  []string
	Difficulty int    // 0 = any, 1 = beginner, 2 = intermediate, 3 = advanced
	Category   string // category slug, empty = any
}
