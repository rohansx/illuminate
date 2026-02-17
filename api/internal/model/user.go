package model

import (
	"time"

	"github.com/google/uuid"
)

type User struct {
	ID             uuid.UUID   `json:"id"`
	GitHubID       int64       `json:"github_id"`
	GitHubUsername string      `json:"github_username"`
	AvatarURL      string      `json:"avatar_url"`
	Bio            string      `json:"bio"`
	Role           string      `json:"role"`
	ComfortLevel   string      `json:"comfort_level"`
	TimeCommitment string      `json:"time_commitment"`
	Goals          []string    `json:"goals"`
	OnboardingDone bool        `json:"onboarding_done"`
	Skills         []UserSkill `json:"skills"`
	CreatedAt      time.Time   `json:"created_at"`
	UpdatedAt      time.Time   `json:"updated_at"`
}

type UserSkill struct {
	Language    string  `json:"language"`
	Proficiency float32 `json:"proficiency"`
	Source      string  `json:"source"`
}

type UserProfile struct {
	ComfortLevel   string   `json:"comfort_level"`
	TimeCommitment string   `json:"time_commitment"`
	Goals          []string `json:"goals"`
}
