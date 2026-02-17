package model

import (
	"time"

	"github.com/google/uuid"
)

type DeepDive struct {
	ID                uuid.UUID `json:"id"`
	IssueID           uuid.UUID `json:"issue_id"`
	UserID            uuid.UUID `json:"user_id"`
	IssueIndexedAt    time.Time `json:"-"`
	ProjectOverview   string    `json:"project_overview"`
	IssueContext      string    `json:"issue_context"`
	SuggestedApproach string    `json:"suggested_approach"`
	QuestionsToAsk    string    `json:"questions_to_ask"`
	RedFlags          string    `json:"red_flags"`
	ModelUsed         string    `json:"model_used"`
	PromptTokens      int       `json:"prompt_tokens"`
	CompletionTokens  int       `json:"completion_tokens"`
	CreatedAt         time.Time `json:"created_at"`
}
