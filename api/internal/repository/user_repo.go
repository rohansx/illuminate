package repository

import (
	"context"
	"fmt"

	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
	"github.com/rohansx/illuminate/api/internal/model"
)

type UserRepo interface {
	GetByID(ctx context.Context, id uuid.UUID) (*model.User, error)
	GetByGitHubID(ctx context.Context, githubID int64) (*model.User, error)
	Upsert(ctx context.Context, user *model.User, tokenEnc []byte) (*model.User, error)
	UpdateProfile(ctx context.Context, id uuid.UUID, profile model.UserProfile) error
	SetSkills(ctx context.Context, userID uuid.UUID, skills []model.UserSkill) error
	GetAccessToken(ctx context.Context, userID uuid.UUID) ([]byte, error)
}

type userRepo struct {
	pool *pgxpool.Pool
}

func NewUserRepo(pool *pgxpool.Pool) UserRepo {
	return &userRepo{pool: pool}
}

func (r *userRepo) GetByID(ctx context.Context, id uuid.UUID) (*model.User, error) {
	u := &model.User{}
	err := r.pool.QueryRow(ctx, `
		SELECT id, github_id, github_username, avatar_url, bio,
			comfort_level, time_commitment, goals, onboarding_done, created_at, updated_at
		FROM users WHERE id = $1`, id,
	).Scan(
		&u.ID, &u.GitHubID, &u.GitHubUsername, &u.AvatarURL, &u.Bio,
		&u.ComfortLevel, &u.TimeCommitment, &u.Goals, &u.OnboardingDone, &u.CreatedAt, &u.UpdatedAt,
	)
	if err != nil {
		if err == pgx.ErrNoRows {
			return nil, nil
		}
		return nil, fmt.Errorf("getting user by id: %w", err)
	}

	skills, err := r.getSkills(ctx, u.ID)
	if err != nil {
		return nil, err
	}
	u.Skills = skills

	return u, nil
}

func (r *userRepo) GetByGitHubID(ctx context.Context, githubID int64) (*model.User, error) {
	u := &model.User{}
	err := r.pool.QueryRow(ctx, `
		SELECT id, github_id, github_username, avatar_url, bio,
			comfort_level, time_commitment, goals, onboarding_done, created_at, updated_at
		FROM users WHERE github_id = $1`, githubID,
	).Scan(
		&u.ID, &u.GitHubID, &u.GitHubUsername, &u.AvatarURL, &u.Bio,
		&u.ComfortLevel, &u.TimeCommitment, &u.Goals, &u.OnboardingDone, &u.CreatedAt, &u.UpdatedAt,
	)
	if err != nil {
		if err == pgx.ErrNoRows {
			return nil, nil
		}
		return nil, fmt.Errorf("getting user by github id: %w", err)
	}
	return u, nil
}

func (r *userRepo) Upsert(ctx context.Context, user *model.User, tokenEnc []byte) (*model.User, error) {
	err := r.pool.QueryRow(ctx, `
		INSERT INTO users (github_id, github_username, avatar_url, bio, access_token_enc)
		VALUES ($1, $2, $3, $4, $5)
		ON CONFLICT (github_id) DO UPDATE SET
			github_username = EXCLUDED.github_username,
			avatar_url = EXCLUDED.avatar_url,
			bio = EXCLUDED.bio,
			access_token_enc = EXCLUDED.access_token_enc,
			updated_at = NOW()
		RETURNING id, github_id, github_username, avatar_url, bio,
			comfort_level, time_commitment, goals, onboarding_done, created_at, updated_at`,
		user.GitHubID, user.GitHubUsername, user.AvatarURL, user.Bio, tokenEnc,
	).Scan(
		&user.ID, &user.GitHubID, &user.GitHubUsername, &user.AvatarURL, &user.Bio,
		&user.ComfortLevel, &user.TimeCommitment, &user.Goals, &user.OnboardingDone, &user.CreatedAt, &user.UpdatedAt,
	)
	if err != nil {
		return nil, fmt.Errorf("upserting user: %w", err)
	}
	return user, nil
}

func (r *userRepo) UpdateProfile(ctx context.Context, id uuid.UUID, profile model.UserProfile) error {
	_, err := r.pool.Exec(ctx, `
		UPDATE users SET
			comfort_level = $2,
			time_commitment = $3,
			goals = $4,
			onboarding_done = true,
			updated_at = NOW()
		WHERE id = $1`,
		id, profile.ComfortLevel, profile.TimeCommitment, profile.Goals,
	)
	if err != nil {
		return fmt.Errorf("updating profile: %w", err)
	}
	return nil
}

func (r *userRepo) SetSkills(ctx context.Context, userID uuid.UUID, skills []model.UserSkill) error {
	tx, err := r.pool.Begin(ctx)
	if err != nil {
		return fmt.Errorf("beginning tx: %w", err)
	}
	defer tx.Rollback(ctx)

	_, err = tx.Exec(ctx, `DELETE FROM user_skills WHERE user_id = $1`, userID)
	if err != nil {
		return fmt.Errorf("deleting old skills: %w", err)
	}

	for _, s := range skills {
		_, err = tx.Exec(ctx, `
			INSERT INTO user_skills (user_id, language, proficiency, source)
			VALUES ($1, $2, $3, $4)`,
			userID, s.Language, s.Proficiency, s.Source,
		)
		if err != nil {
			return fmt.Errorf("inserting skill: %w", err)
		}
	}

	return tx.Commit(ctx)
}

func (r *userRepo) GetAccessToken(ctx context.Context, userID uuid.UUID) ([]byte, error) {
	var tokenEnc []byte
	err := r.pool.QueryRow(ctx, `SELECT access_token_enc FROM users WHERE id = $1`, userID).Scan(&tokenEnc)
	if err != nil {
		return nil, fmt.Errorf("getting access token: %w", err)
	}
	return tokenEnc, nil
}

func (r *userRepo) getSkills(ctx context.Context, userID uuid.UUID) ([]model.UserSkill, error) {
	rows, err := r.pool.Query(ctx, `
		SELECT language, proficiency, source FROM user_skills
		WHERE user_id = $1 ORDER BY proficiency DESC`, userID,
	)
	if err != nil {
		return nil, fmt.Errorf("querying skills: %w", err)
	}
	defer rows.Close()

	var skills []model.UserSkill
	for rows.Next() {
		var s model.UserSkill
		if err := rows.Scan(&s.Language, &s.Proficiency, &s.Source); err != nil {
			return nil, fmt.Errorf("scanning skill: %w", err)
		}
		skills = append(skills, s)
	}
	return skills, nil
}
