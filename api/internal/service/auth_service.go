package service

import (
	"context"
	"crypto/sha256"
	"fmt"
	"log/slog"
	"time"

	"github.com/google/uuid"
	"github.com/rohansx/illuminate/api/internal/crypto"
	"github.com/rohansx/illuminate/api/internal/model"
	"github.com/rohansx/illuminate/api/internal/repository"
)

type AuthResult struct {
	User         *model.User
	AccessToken  string
	RefreshToken string
}

type AuthService struct {
	github        *GitHubService
	userRepo      repository.UserRepo
	tokenRepo     repository.TokenRepo
	encryptor     *crypto.Encryptor
	jwt           *crypto.JWTManager
	adminUsername string
}

func NewAuthService(
	github *GitHubService,
	userRepo repository.UserRepo,
	tokenRepo repository.TokenRepo,
	encryptor *crypto.Encryptor,
	jwt *crypto.JWTManager,
	adminUsername string,
) *AuthService {
	return &AuthService{
		github:        github,
		userRepo:      userRepo,
		tokenRepo:     tokenRepo,
		encryptor:     encryptor,
		jwt:           jwt,
		adminUsername: adminUsername,
	}
}

func (s *AuthService) HandleCallback(ctx context.Context, code string) (*AuthResult, error) {
	oauthToken, err := s.github.ExchangeCode(ctx, code)
	if err != nil {
		return nil, fmt.Errorf("exchanging code: %w", err)
	}

	ghUser, err := s.github.GetUser(ctx, oauthToken.AccessToken)
	if err != nil {
		return nil, fmt.Errorf("getting github user: %w", err)
	}

	tokenEnc, err := s.encryptor.Encrypt(oauthToken.AccessToken)
	if err != nil {
		return nil, fmt.Errorf("encrypting token: %w", err)
	}

	user := &model.User{
		GitHubID:       ghUser.ID,
		GitHubUsername: ghUser.Login,
		AvatarURL:      ghUser.AvatarURL,
		Bio:            ghUser.Bio,
	}

	user, err = s.userRepo.Upsert(ctx, user, tokenEnc)
	if err != nil {
		return nil, fmt.Errorf("upserting user: %w", err)
	}

	// Auto-promote to admin
	if s.adminUsername != "" && user.GitHubUsername == s.adminUsername && user.Role != "admin" {
		if err := s.userRepo.UpdateRole(ctx, user.ID, "admin"); err != nil {
			slog.Warn("failed to auto-promote admin", "error", err)
		} else {
			user.Role = "admin"
		}
	} else if s.adminUsername == "" && user.Role != "admin" {
		count, err := s.userRepo.Count(ctx)
		if err == nil && count == 1 {
			if err := s.userRepo.UpdateRole(ctx, user.ID, "admin"); err != nil {
				slog.Warn("failed to auto-promote first user", "error", err)
			} else {
				user.Role = "admin"
			}
		}
	}

	accessToken, err := s.jwt.Generate(user.ID)
	if err != nil {
		return nil, fmt.Errorf("generating jwt: %w", err)
	}

	refreshToken, refreshHash, err := crypto.GenerateRefreshToken()
	if err != nil {
		return nil, fmt.Errorf("generating refresh token: %w", err)
	}

	err = s.tokenRepo.Create(ctx, user.ID, refreshHash, time.Now().Add(7*24*time.Hour))
	if err != nil {
		return nil, fmt.Errorf("storing refresh token: %w", err)
	}

	slog.Info("user authenticated", "user_id", user.ID, "github_username", user.GitHubUsername)

	return &AuthResult{
		User:         user,
		AccessToken:  accessToken,
		RefreshToken: refreshToken,
	}, nil
}

func (s *AuthService) RefreshAccessToken(ctx context.Context, refreshToken string) (*AuthResult, error) {
	hash := sha256.Sum256([]byte(refreshToken))

	stored, err := s.tokenRepo.GetByHash(ctx, hash[:])
	if err != nil {
		return nil, fmt.Errorf("looking up refresh token: %w", err)
	}
	if stored == nil {
		return nil, fmt.Errorf("invalid refresh token")
	}

	// Rotate: delete old token
	if err := s.tokenRepo.DeleteByHash(ctx, hash[:]); err != nil {
		return nil, fmt.Errorf("deleting old refresh token: %w", err)
	}

	user, err := s.userRepo.GetByID(ctx, stored.UserID)
	if err != nil {
		return nil, fmt.Errorf("getting user: %w", err)
	}
	if user == nil {
		return nil, fmt.Errorf("user not found")
	}

	accessToken, err := s.jwt.Generate(user.ID)
	if err != nil {
		return nil, fmt.Errorf("generating jwt: %w", err)
	}

	newRefreshToken, newHash, err := crypto.GenerateRefreshToken()
	if err != nil {
		return nil, fmt.Errorf("generating new refresh token: %w", err)
	}

	err = s.tokenRepo.Create(ctx, user.ID, newHash, time.Now().Add(7*24*time.Hour))
	if err != nil {
		return nil, fmt.Errorf("storing new refresh token: %w", err)
	}

	return &AuthResult{
		User:         user,
		AccessToken:  accessToken,
		RefreshToken: newRefreshToken,
	}, nil
}

func (s *AuthService) Logout(ctx context.Context, userID uuid.UUID) error {
	if err := s.tokenRepo.DeleteByUserID(ctx, userID); err != nil {
		return fmt.Errorf("deleting refresh tokens: %w", err)
	}
	return nil
}
