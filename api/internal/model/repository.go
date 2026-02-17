package model

import (
	"time"

	"github.com/google/uuid"
)

type Repository struct {
	ID              uuid.UUID  `json:"id"`
	GitHubID        int64      `json:"github_id"`
	Owner           string     `json:"owner"`
	Name            string     `json:"name"`
	Description     string     `json:"description"`
	Stars           int        `json:"stars"`
	PrimaryLanguage string     `json:"primary_language"`
	Topics          []string   `json:"topics"`
	HasContributing bool       `json:"has_contributing"`
	HealthScore     float32    `json:"health_score"`
	LastCommitAt    *time.Time `json:"last_commit_at"`
	IndexedAt       time.Time  `json:"indexed_at"`
	Tags            []string   `json:"tags"`
	DifficultyLevel string     `json:"difficulty_level"`
	ActivityStatus  string     `json:"activity_status"`
	Categories      []Category `json:"categories,omitempty"`
}

type Category struct {
	ID          uuid.UUID `json:"id"`
	Name        string    `json:"name"`
	Slug        string    `json:"slug"`
	Description string    `json:"description"`
	Icon        string    `json:"icon"`
}

func (r *Repository) FullName() string {
	return r.Owner + "/" + r.Name
}
