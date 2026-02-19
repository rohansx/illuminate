package repository

import (
	"context"
	"fmt"

	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
	"github.com/rohansx/illuminate/api/internal/model"
)

type DeepDiveRepo interface {
	GetByIssueAndUser(ctx context.Context, issueID, userID uuid.UUID) (*model.DeepDive, error)
	Upsert(ctx context.Context, dd *model.DeepDive) (*model.DeepDive, error)
}

type deepDiveRepo struct {
	pool *pgxpool.Pool
}

func NewDeepDiveRepo(pool *pgxpool.Pool) DeepDiveRepo {
	return &deepDiveRepo{pool: pool}
}

func (r *deepDiveRepo) GetByIssueAndUser(ctx context.Context, issueID, userID uuid.UUID) (*model.DeepDive, error) {
	dd := &model.DeepDive{}
	err := r.pool.QueryRow(ctx, `
		SELECT id, issue_id, user_id, issue_indexed_at,
			project_overview, issue_context, suggested_approach,
			questions_to_ask, red_flags, first_comment, model_used,
			prompt_tokens, completion_tokens, created_at
		FROM deep_dives
		WHERE issue_id = $1 AND user_id = $2`, issueID, userID,
	).Scan(
		&dd.ID, &dd.IssueID, &dd.UserID, &dd.IssueIndexedAt,
		&dd.ProjectOverview, &dd.IssueContext, &dd.SuggestedApproach,
		&dd.QuestionsToAsk, &dd.RedFlags, &dd.FirstComment, &dd.ModelUsed,
		&dd.PromptTokens, &dd.CompletionTokens, &dd.CreatedAt,
	)
	if err != nil {
		if err == pgx.ErrNoRows {
			return nil, nil
		}
		return nil, fmt.Errorf("getting deep dive: %w", err)
	}
	return dd, nil
}

func (r *deepDiveRepo) Upsert(ctx context.Context, dd *model.DeepDive) (*model.DeepDive, error) {
	err := r.pool.QueryRow(ctx, `
		INSERT INTO deep_dives (issue_id, user_id, issue_indexed_at,
			project_overview, issue_context, suggested_approach,
			questions_to_ask, red_flags, first_comment, model_used,
			prompt_tokens, completion_tokens)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
		ON CONFLICT (issue_id, user_id) DO UPDATE SET
			issue_indexed_at = EXCLUDED.issue_indexed_at,
			project_overview = EXCLUDED.project_overview,
			issue_context = EXCLUDED.issue_context,
			suggested_approach = EXCLUDED.suggested_approach,
			questions_to_ask = EXCLUDED.questions_to_ask,
			red_flags = EXCLUDED.red_flags,
			first_comment = EXCLUDED.first_comment,
			model_used = EXCLUDED.model_used,
			prompt_tokens = EXCLUDED.prompt_tokens,
			completion_tokens = EXCLUDED.completion_tokens,
			created_at = NOW()
		RETURNING id, issue_id, user_id, issue_indexed_at,
			project_overview, issue_context, suggested_approach,
			questions_to_ask, red_flags, first_comment, model_used,
			prompt_tokens, completion_tokens, created_at`,
		dd.IssueID, dd.UserID, dd.IssueIndexedAt,
		dd.ProjectOverview, dd.IssueContext, dd.SuggestedApproach,
		dd.QuestionsToAsk, dd.RedFlags, dd.FirstComment, dd.ModelUsed,
		dd.PromptTokens, dd.CompletionTokens,
	).Scan(
		&dd.ID, &dd.IssueID, &dd.UserID, &dd.IssueIndexedAt,
		&dd.ProjectOverview, &dd.IssueContext, &dd.SuggestedApproach,
		&dd.QuestionsToAsk, &dd.RedFlags, &dd.FirstComment, &dd.ModelUsed,
		&dd.PromptTokens, &dd.CompletionTokens, &dd.CreatedAt,
	)
	if err != nil {
		return nil, fmt.Errorf("upserting deep dive: %w", err)
	}
	return dd, nil
}
