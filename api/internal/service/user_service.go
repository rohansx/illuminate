package service

import (
	"context"
	"fmt"
	"log/slog"
	"sort"

	"github.com/google/uuid"
	"github.com/rohansx/illuminate/api/internal/crypto"
	"github.com/rohansx/illuminate/api/internal/model"
	"github.com/rohansx/illuminate/api/internal/repository"
)

type UserService struct {
	userRepo  repository.UserRepo
	github    *GitHubService
	encryptor *crypto.Encryptor
}

func NewUserService(
	userRepo repository.UserRepo,
	github *GitHubService,
	encryptor *crypto.Encryptor,
) *UserService {
	return &UserService{
		userRepo:  userRepo,
		github:    github,
		encryptor: encryptor,
	}
}

func (s *UserService) GetProfile(ctx context.Context, userID uuid.UUID) (*model.User, error) {
	user, err := s.userRepo.GetByID(ctx, userID)
	if err != nil {
		return nil, fmt.Errorf("getting user: %w", err)
	}
	if user == nil {
		return nil, fmt.Errorf("user not found")
	}
	return user, nil
}

func (s *UserService) UpdateProfile(ctx context.Context, userID uuid.UUID, profile model.UserProfile) error {
	if err := s.userRepo.UpdateProfile(ctx, userID, profile); err != nil {
		return fmt.Errorf("updating profile: %w", err)
	}
	return nil
}

func (s *UserService) AnalyzeSkills(ctx context.Context, userID uuid.UUID) ([]model.UserSkill, error) {
	tokenEnc, err := s.userRepo.GetAccessToken(ctx, userID)
	if err != nil {
		return nil, fmt.Errorf("getting access token: %w", err)
	}

	accessToken, err := s.encryptor.Decrypt(tokenEnc)
	if err != nil {
		return nil, fmt.Errorf("decrypting token: %w", err)
	}

	repos, err := s.github.GetUserRepos(ctx, accessToken)
	if err != nil {
		return nil, fmt.Errorf("getting user repos: %w", err)
	}

	// Aggregate language bytes across all repos
	langTotals := make(map[string]int)
	for _, repo := range repos {
		languages, err := s.github.GetRepoLanguages(ctx, accessToken, repo.Owner.Login, repo.Name)
		if err != nil {
			slog.Warn("failed to get languages for repo", "repo", repo.Name, "error", err)
			continue
		}
		for lang, bytes := range languages {
			langTotals[lang] += bytes
		}
	}

	if len(langTotals) == 0 {
		return nil, nil
	}

	// Compute total bytes for normalization
	var totalBytes int
	for _, bytes := range langTotals {
		totalBytes += bytes
	}

	// Convert to skills with proficiency score (0.0–1.0)
	var skills []model.UserSkill
	for lang, bytes := range langTotals {
		proficiency := float32(bytes) / float32(totalBytes)
		if proficiency < 0.01 {
			continue // Skip languages under 1%
		}
		skills = append(skills, model.UserSkill{
			Language:    lang,
			Proficiency: proficiency,
			Source:      "github",
		})
	}

	// Sort by proficiency descending
	sort.Slice(skills, func(i, j int) bool {
		return skills[i].Proficiency > skills[j].Proficiency
	})

	// Keep top 15
	if len(skills) > 15 {
		skills = skills[:15]
	}

	if err := s.userRepo.SetGitHubSkills(ctx, userID, skills); err != nil {
		return nil, fmt.Errorf("saving skills: %w", err)
	}

	slog.Info("analyzed user skills", "user_id", userID, "skill_count", len(skills))
	return skills, nil
}

func (s *UserService) SetManualSkills(ctx context.Context, userID uuid.UUID, languages []string) ([]model.UserSkill, error) {
	if err := s.userRepo.SetManualSkills(ctx, userID, languages); err != nil {
		return nil, fmt.Errorf("setting manual skills: %w", err)
	}

	user, err := s.userRepo.GetByID(ctx, userID)
	if err != nil {
		return nil, fmt.Errorf("getting updated user: %w", err)
	}

	slog.Info("set manual skills", "user_id", userID, "count", len(languages))
	return user.Skills, nil
}
