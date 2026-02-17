package service

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"os"
	"time"

	"github.com/google/uuid"
	"github.com/rohansx/illuminate/api/internal/model"
	"github.com/rohansx/illuminate/api/internal/repository"
)

type AdminService struct {
	userRepo      repository.UserRepo
	repoRepo      repository.RepoRepo
	issueRepo     repository.IssueRepo
	issueService  *IssueService
	githubService *GitHubService
	jobManager    *JobManager
}

func NewAdminService(
	userRepo repository.UserRepo,
	repoRepo repository.RepoRepo,
	issueRepo repository.IssueRepo,
	issueService *IssueService,
	githubService *GitHubService,
	jobManager *JobManager,
) *AdminService {
	return &AdminService{
		userRepo:      userRepo,
		repoRepo:      repoRepo,
		issueRepo:     issueRepo,
		issueService:  issueService,
		githubService: githubService,
		jobManager:    jobManager,
	}
}

func (s *AdminService) GetStats(ctx context.Context) (*model.AdminStats, error) {
	userCount, err := s.userRepo.Count(ctx)
	if err != nil {
		return nil, fmt.Errorf("counting users: %w", err)
	}
	repoCount, err := s.repoRepo.Count(ctx)
	if err != nil {
		return nil, fmt.Errorf("counting repos: %w", err)
	}
	issueCount, err := s.issueRepo.Count(ctx)
	if err != nil {
		return nil, fmt.Errorf("counting issues: %w", err)
	}
	return &model.AdminStats{
		UserCount:  userCount,
		RepoCount:  repoCount,
		IssueCount: issueCount,
	}, nil
}

func (s *AdminService) ListUsers(ctx context.Context, page, perPage int) (*model.UserList, error) {
	if page < 1 {
		page = 1
	}
	if perPage < 1 || perPage > 100 {
		perPage = 50
	}
	offset := (page - 1) * perPage

	users, total, err := s.userRepo.ListAll(ctx, perPage, offset)
	if err != nil {
		return nil, fmt.Errorf("listing users: %w", err)
	}
	return &model.UserList{Users: users, TotalCount: total, Page: page, PerPage: perPage}, nil
}

func (s *AdminService) UpdateUserRole(ctx context.Context, userID uuid.UUID, role string) error {
	if role != "user" && role != "admin" {
		return fmt.Errorf("invalid role: %s", role)
	}
	return s.userRepo.UpdateRole(ctx, userID, role)
}

func (s *AdminService) ListRepos(ctx context.Context, page, perPage int) (*model.RepoList, error) {
	if page < 1 {
		page = 1
	}
	if perPage < 1 || perPage > 100 {
		perPage = 50
	}
	offset := (page - 1) * perPage

	repos, total, err := s.repoRepo.ListWithIssueCounts(ctx, perPage, offset)
	if err != nil {
		return nil, fmt.Errorf("listing repos: %w", err)
	}
	return &model.RepoList{Repos: repos, TotalCount: total, Page: page, PerPage: perPage}, nil
}

func (s *AdminService) DeleteRepo(ctx context.Context, id uuid.UUID) error {
	return s.repoRepo.Delete(ctx, id)
}

func (s *AdminService) TriggerSeed(ctx context.Context) (*model.JobStatus, error) {
	return s.jobManager.StartJob("seed", func(ctx context.Context, progressFn func(current, total int)) error {
		data, err := os.ReadFile("data/seed_repos.json")
		if err != nil {
			// Try alternate path for local dev
			data, err = os.ReadFile("api/data/seed_repos.json")
			if err != nil {
				return fmt.Errorf("reading seed file: %w", err)
			}
		}

		type seedRepo struct {
			Owner string `json:"owner"`
			Name  string `json:"name"`
		}

		var seeds []seedRepo
		if err := json.Unmarshal(data, &seeds); err != nil {
			return fmt.Errorf("parsing seed file: %w", err)
		}

		slog.Info("seed started", "total_repos", len(seeds))
		progressFn(0, len(seeds))

		seeded, skipped, failed := 0, 0, 0
		for i, seed := range seeds {
			fullName := seed.Owner + "/" + seed.Name
			slog.Info("seeding repo", "repo", fullName, "progress", fmt.Sprintf("%d/%d", i+1, len(seeds)))

			ghRepo, err := s.githubService.GetPublicRepo(ctx, seed.Owner, seed.Name)
			if err != nil {
				slog.Warn("failed to fetch repo from github", "repo", fullName, "error", err)
				failed++
				progressFn(i+1, len(seeds))
				continue
			}

			var lastCommit *time.Time
			if ghRepo.PushedAt != "" {
				t, err := time.Parse(time.RFC3339, ghRepo.PushedAt)
				if err == nil {
					lastCommit = &t
				}
			}

			repo := &model.Repository{
				GitHubID:        ghRepo.ID,
				Owner:           ghRepo.Owner.Login,
				Name:            ghRepo.Name,
				Description:     ghRepo.Description,
				Stars:           ghRepo.StargazersCount,
				PrimaryLanguage: ghRepo.Language,
				Topics:          ghRepo.Topics,
				HealthScore:     0.5,
				LastCommitAt:    lastCommit,
			}

			if _, err := s.repoRepo.Upsert(ctx, repo); err != nil {
				slog.Warn("failed to upsert repo", "repo", fullName, "error", err)
				failed++
			} else {
				slog.Info("seeded repo", "repo", fullName, "stars", ghRepo.StargazersCount, "language", ghRepo.Language)
				seeded++
			}

			progressFn(i+1, len(seeds))
			time.Sleep(500 * time.Millisecond)
		}

		slog.Info("seed completed", "seeded", seeded, "skipped", skipped, "failed", failed, "total", len(seeds))
		return nil
	})
}

func (s *AdminService) TriggerIndex(ctx context.Context) (*model.JobStatus, error) {
	return s.jobManager.StartJob("index", func(ctx context.Context, progressFn func(current, total int)) error {
		repos, err := s.repoRepo.GetAll(ctx)
		if err != nil {
			return fmt.Errorf("getting repos: %w", err)
		}

		progressFn(0, len(repos))

		for i, repo := range repos {
			if err := s.issueService.IndexRepository(ctx, repo.Owner, repo.Name); err != nil {
				slog.Warn("failed to index", "repo", repo.FullName(), "error", err)
			}
			progressFn(i+1, len(repos))
			time.Sleep(time.Second)
		}
		return nil
	})
}

func (s *AdminService) GetJobs() []model.JobStatus {
	return s.jobManager.GetAll()
}

func (s *AdminService) UpdateRepoMetadata(ctx context.Context, repoID uuid.UUID, tags []string, difficulty, activity string) error {
	return s.repoRepo.UpdateMetadata(ctx, repoID, tags, difficulty, activity)
}

func (s *AdminService) AssignRepoCategory(ctx context.Context, repoID, categoryID uuid.UUID) error {
	return s.repoRepo.AssignCategory(ctx, repoID, categoryID)
}

func (s *AdminService) RemoveRepoCategory(ctx context.Context, repoID, categoryID uuid.UUID) error {
	return s.repoRepo.RemoveCategory(ctx, repoID, categoryID)
}

func (s *AdminService) GetCategories(ctx context.Context) ([]model.Category, error) {
	return s.repoRepo.GetCategories(ctx)
}
