package model

import (
	"time"

	"github.com/google/uuid"
)

type Contribution struct {
	ID          uuid.UUID  `json:"id"`
	UserID      uuid.UUID  `json:"user_id"`
	GitHubPRID  int64      `json:"github_pr_id"`
	RepoOwner   string     `json:"repo_owner"`
	RepoName    string     `json:"repo_name"`
	PRNumber    int        `json:"pr_number"`
	PRTitle     string     `json:"pr_title"`
	PRURL       string     `json:"pr_url"`
	PRState     string     `json:"pr_state"`
	Language    string     `json:"language"`
	Labels      []string   `json:"labels"`
	MergedAt    *time.Time `json:"merged_at"`
	CreatedAt   time.Time  `json:"created_at"`
	SyncedAt    time.Time  `json:"synced_at"`
}

type ContributionFeed struct {
	Contributions []Contribution `json:"contributions"`
	TotalCount    int            `json:"total_count"`
	Page          int            `json:"page"`
	PerPage       int            `json:"per_page"`
}

type ProjectGroup struct {
	RepoOwner string     `json:"repo_owner"`
	RepoName  string     `json:"repo_name"`
	Language  string     `json:"language"`
	PRCount   int        `json:"pr_count"`
	LatestAt  *time.Time `json:"latest_at"`
}

type PortfolioStats struct {
	TotalPRs      int            `json:"total_prs"`
	TotalRepos    int            `json:"total_repos"`
	Languages     map[string]int `json:"languages"`
	FirstContrib  *time.Time     `json:"first_contribution"`
	LatestContrib *time.Time     `json:"latest_contribution"`
	CurrentStreak int            `json:"current_streak"`
	LongestStreak int            `json:"longest_streak"`
}

type PublicProfile struct {
	User        PublicUser     `json:"user"`
	Stats       PortfolioStats `json:"stats"`
	TopProjects []ProjectGroup `json:"top_projects"`
	RecentPRs   []Contribution `json:"recent_prs"`
}

type PublicUser struct {
	GitHubUsername string      `json:"github_username"`
	AvatarURL      string      `json:"avatar_url"`
	Bio            string      `json:"bio"`
	Skills         []UserSkill `json:"skills"`
	CreatedAt      time.Time   `json:"created_at"`
}

type UserSyncInfo struct {
	ID              uuid.UUID
	GitHubUsername  string
	ContribSyncedAt *time.Time
}
