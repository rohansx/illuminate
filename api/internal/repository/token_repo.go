package repository

import (
	"context"
	"fmt"
	"time"

	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
)

type RefreshToken struct {
	ID        uuid.UUID
	UserID    uuid.UUID
	TokenHash []byte
	ExpiresAt time.Time
	CreatedAt time.Time
}

type TokenRepo interface {
	Create(ctx context.Context, userID uuid.UUID, tokenHash []byte, expiresAt time.Time) error
	GetByHash(ctx context.Context, tokenHash []byte) (*RefreshToken, error)
	DeleteByUserID(ctx context.Context, userID uuid.UUID) error
	DeleteByHash(ctx context.Context, tokenHash []byte) error
}

type tokenRepo struct {
	pool *pgxpool.Pool
}

func NewTokenRepo(pool *pgxpool.Pool) TokenRepo {
	return &tokenRepo{pool: pool}
}

func (r *tokenRepo) Create(ctx context.Context, userID uuid.UUID, tokenHash []byte, expiresAt time.Time) error {
	_, err := r.pool.Exec(ctx, `
		INSERT INTO refresh_tokens (user_id, token_hash, expires_at)
		VALUES ($1, $2, $3)`,
		userID, tokenHash, expiresAt,
	)
	if err != nil {
		return fmt.Errorf("creating refresh token: %w", err)
	}
	return nil
}

func (r *tokenRepo) GetByHash(ctx context.Context, tokenHash []byte) (*RefreshToken, error) {
	t := &RefreshToken{}
	err := r.pool.QueryRow(ctx, `
		SELECT id, user_id, token_hash, expires_at, created_at
		FROM refresh_tokens
		WHERE token_hash = $1 AND expires_at > NOW()`,
		tokenHash,
	).Scan(&t.ID, &t.UserID, &t.TokenHash, &t.ExpiresAt, &t.CreatedAt)
	if err != nil {
		if err == pgx.ErrNoRows {
			return nil, nil
		}
		return nil, fmt.Errorf("getting refresh token: %w", err)
	}
	return t, nil
}

func (r *tokenRepo) DeleteByUserID(ctx context.Context, userID uuid.UUID) error {
	_, err := r.pool.Exec(ctx, `DELETE FROM refresh_tokens WHERE user_id = $1`, userID)
	if err != nil {
		return fmt.Errorf("deleting refresh tokens: %w", err)
	}
	return nil
}

func (r *tokenRepo) DeleteByHash(ctx context.Context, tokenHash []byte) error {
	_, err := r.pool.Exec(ctx, `DELETE FROM refresh_tokens WHERE token_hash = $1`, tokenHash)
	if err != nil {
		return fmt.Errorf("deleting refresh token: %w", err)
	}
	return nil
}
