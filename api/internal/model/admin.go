package model

import (
	"time"

	"github.com/google/uuid"
)

type AdminStats struct {
	UserCount  int `json:"user_count"`
	RepoCount  int `json:"repo_count"`
	IssueCount int `json:"issue_count"`
}

type UserListItem struct {
	ID             uuid.UUID `json:"id"`
	GitHubUsername string    `json:"github_username"`
	AvatarURL      string    `json:"avatar_url"`
	Role           string    `json:"role"`
	OnboardingDone bool      `json:"onboarding_done"`
	CreatedAt      time.Time `json:"created_at"`
}

type UserList struct {
	Users      []UserListItem `json:"users"`
	TotalCount int            `json:"total_count"`
	Page       int            `json:"page"`
	PerPage    int            `json:"per_page"`
}

type RepoListItem struct {
	ID              uuid.UUID  `json:"id"`
	Owner           string     `json:"owner"`
	Name            string     `json:"name"`
	Stars           int        `json:"stars"`
	PrimaryLanguage string     `json:"primary_language"`
	IssueCount      int        `json:"issue_count"`
	IndexedAt       time.Time  `json:"indexed_at"`
	Tags            []string   `json:"tags"`
	DifficultyLevel string     `json:"difficulty_level"`
	ActivityStatus  string     `json:"activity_status"`
	Categories      []Category `json:"categories,omitempty"`
}

type RepoList struct {
	Repos      []RepoListItem `json:"repos"`
	TotalCount int            `json:"total_count"`
	Page       int            `json:"page"`
	PerPage    int            `json:"per_page"`
}

type JobStatus struct {
	ID        string    `json:"id"`
	Type      string    `json:"type"`
	Status    string    `json:"status"`
	Progress  string    `json:"progress"`
	StartedAt time.Time `json:"started_at"`
	Error     string    `json:"error,omitempty"`
}
