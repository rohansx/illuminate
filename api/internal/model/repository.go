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
}

func (r *Repository) FullName() string {
	return r.Owner + "/" + r.Name
}
