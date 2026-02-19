package repository

import (
	"context"
	"fmt"

	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
	"github.com/rohansx/illuminate/api/internal/model"
)

type IssueProgressRepo interface {
	Get(ctx context.Context, userID, issueID uuid.UUID) (*model.IssueProgress, error)
	Upsert(ctx context.Context, userID, issueID uuid.UUID, status string) (*model.IssueProgress, error)
	UpdateStatus(ctx context.Context, userID, issueID uuid.UUID, status string) (*model.IssueProgress, error)
	AddNote(ctx context.Context, userID, issueID uuid.UUID, note string) (*model.IssueProgress, error)
	Delete(ctx context.Context, userID, issueID uuid.UUID) error
	ListByUser(ctx context.Context, userID uuid.UUID) ([]model.IssueProgress, error)
}

type issueProgressRepo struct {
	pool *pgxpool.Pool
}

func NewIssueProgressRepo(pool *pgxpool.Pool) IssueProgressRepo {
	return &issueProgressRepo{pool: pool}
}

func (r *issueProgressRepo) scanRow(row pgx.Row) (*model.IssueProgress, error) {
	p := &model.IssueProgress{}
	err := row.Scan(&p.ID, &p.UserID, &p.IssueID, &p.Status, &p.Notes, &p.StartedAt, &p.UpdatedAt)
	if err != nil {
		if err == pgx.ErrNoRows {
			return nil, nil
		}
		return nil, err
	}
	return p, nil
}

func (r *issueProgressRepo) Get(ctx context.Context, userID, issueID uuid.UUID) (*model.IssueProgress, error) {
	row := r.pool.QueryRow(ctx, `
		SELECT id, user_id, issue_id, status, notes, started_at, updated_at
		FROM issue_progress
		WHERE user_id = $1 AND issue_id = $2`, userID, issueID)
	p, err := r.scanRow(row)
	if err != nil {
		return nil, fmt.Errorf("getting issue progress: %w", err)
	}
	return p, nil
}

func (r *issueProgressRepo) Upsert(ctx context.Context, userID, issueID uuid.UUID, status string) (*model.IssueProgress, error) {
	row := r.pool.QueryRow(ctx, `
		INSERT INTO issue_progress (user_id, issue_id, status)
		VALUES ($1, $2, $3)
		ON CONFLICT (user_id, issue_id) DO UPDATE SET
			status = EXCLUDED.status,
			updated_at = NOW()
		RETURNING id, user_id, issue_id, status, notes, started_at, updated_at`,
		userID, issueID, status)
	p, err := r.scanRow(row)
	if err != nil {
		return nil, fmt.Errorf("upserting issue progress: %w", err)
	}
	return p, nil
}

func (r *issueProgressRepo) UpdateStatus(ctx context.Context, userID, issueID uuid.UUID, status string) (*model.IssueProgress, error) {
	row := r.pool.QueryRow(ctx, `
		UPDATE issue_progress SET status = $3, updated_at = NOW()
		WHERE user_id = $1 AND issue_id = $2
		RETURNING id, user_id, issue_id, status, notes, started_at, updated_at`,
		userID, issueID, status)
	p, err := r.scanRow(row)
	if err != nil {
		return nil, fmt.Errorf("updating issue progress status: %w", err)
	}
	return p, nil
}

func (r *issueProgressRepo) AddNote(ctx context.Context, userID, issueID uuid.UUID, note string) (*model.IssueProgress, error) {
	row := r.pool.QueryRow(ctx, `
		UPDATE issue_progress SET notes = array_append(notes, $3), updated_at = NOW()
		WHERE user_id = $1 AND issue_id = $2
		RETURNING id, user_id, issue_id, status, notes, started_at, updated_at`,
		userID, issueID, note)
	p, err := r.scanRow(row)
	if err != nil {
		return nil, fmt.Errorf("adding note to issue progress: %w", err)
	}
	return p, nil
}

func (r *issueProgressRepo) Delete(ctx context.Context, userID, issueID uuid.UUID) error {
	_, err := r.pool.Exec(ctx, `
		DELETE FROM issue_progress WHERE user_id = $1 AND issue_id = $2`,
		userID, issueID)
	if err != nil {
		return fmt.Errorf("deleting issue progress: %w", err)
	}
	return nil
}

func (r *issueProgressRepo) ListByUser(ctx context.Context, userID uuid.UUID) ([]model.IssueProgress, error) {
	rows, err := r.pool.Query(ctx, `
		SELECT id, user_id, issue_id, status, notes, started_at, updated_at
		FROM issue_progress
		WHERE user_id = $1
		ORDER BY updated_at DESC`, userID)
	if err != nil {
		return nil, fmt.Errorf("listing issue progress: %w", err)
	}
	defer rows.Close()

	var results []model.IssueProgress
	for rows.Next() {
		var p model.IssueProgress
		if err := rows.Scan(&p.ID, &p.UserID, &p.IssueID, &p.Status, &p.Notes, &p.StartedAt, &p.UpdatedAt); err != nil {
			return nil, fmt.Errorf("scanning issue progress: %w", err)
		}
		results = append(results, p)
	}
	return results, nil
}
