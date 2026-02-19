package service

import (
	"context"
	"encoding/csv"
	"encoding/json"
	"fmt"
	"io"
	"log/slog"
	"net/http"
	"os"
	"strings"
	"time"

	"github.com/google/uuid"
	"github.com/rohansx/illuminate/api/internal/model"
	"github.com/rohansx/illuminate/api/internal/repository"
)

type AdminService struct {
	userRepo         repository.UserRepo
	repoRepo         repository.RepoRepo
	issueRepo        repository.IssueRepo
	issueService     *IssueService
	githubService    *GitHubService
	jobManager       *JobManager
	discoveryService *DiscoveryService
	contribService   *ContributionService
}

func NewAdminService(
	userRepo repository.UserRepo,
	repoRepo repository.RepoRepo,
	issueRepo repository.IssueRepo,
	issueService *IssueService,
	githubService *GitHubService,
	jobManager *JobManager,
	discoveryService *DiscoveryService,
	contribService *ContributionService,
) *AdminService {
	return &AdminService{
		userRepo:         userRepo,
		repoRepo:         repoRepo,
		issueRepo:        issueRepo,
		issueService:     issueService,
		githubService:    githubService,
		jobManager:       jobManager,
		discoveryService: discoveryService,
		contribService:   contribService,
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

func (s *AdminService) TriggerDiscover(ctx context.Context) (*model.JobStatus, error) {
	return s.discoveryService.Discover(ctx)
}

func (s *AdminService) TriggerContributionSync(ctx context.Context) (*model.JobStatus, error) {
	return s.contribService.SyncAll(ctx)
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

const openSourceJobsCSV = "https://raw.githubusercontent.com/timqian/open-source-jobs/main/repos.csv"

func (s *AdminService) TriggerHiringSeed(ctx context.Context) (*model.JobStatus, error) {
	return s.jobManager.StartJob("hiring-seed", func(ctx context.Context, progressFn func(current, total int)) error {
		// Fetch CSV from GitHub
		resp, err := http.Get(openSourceJobsCSV)
		if err != nil {
			return fmt.Errorf("fetching open-source-jobs CSV: %w", err)
		}
		defer resp.Body.Close()

		if resp.StatusCode != 200 {
			return fmt.Errorf("open-source-jobs CSV returned %d", resp.StatusCode)
		}

		body, err := io.ReadAll(resp.Body)
		if err != nil {
			return fmt.Errorf("reading CSV body: %w", err)
		}

		reader := csv.NewReader(strings.NewReader(string(body)))
		records, err := reader.ReadAll()
		if err != nil {
			return fmt.Errorf("parsing CSV: %w", err)
		}

		// Skip header row; columns: Repository,Company Name,Company URL,Career URL,Tags,Language,Description
		if len(records) < 2 {
			slog.Info("hiring-seed: no records found in CSV")
			progressFn(1, 1)
			return nil
		}
		rows := records[1:]

		slog.Info("hiring-seed started", "total_repos", len(rows))
		progressFn(0, len(rows))

		seeded, failed := 0, 0
		for i, row := range rows {
			if len(row) < 4 {
				failed++
				progressFn(i+1, len(rows))
				continue
			}

			repoFullName := strings.TrimSpace(row[0])
			careerURL := strings.TrimSpace(row[3])

			parts := strings.SplitN(repoFullName, "/", 2)
			if len(parts) != 2 || parts[0] == "" || parts[1] == "" {
				slog.Warn("hiring-seed: invalid repo name", "name", repoFullName)
				failed++
				progressFn(i+1, len(rows))
				continue
			}
			owner, name := parts[0], parts[1]
			fullName := owner + "/" + name

			ghRepo, err := s.githubService.GetPublicRepo(ctx, owner, name)
			if err != nil {
				slog.Warn("hiring-seed: failed to fetch from github", "repo", fullName, "error", err)
				failed++
				progressFn(i+1, len(rows))
				time.Sleep(500 * time.Millisecond)
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
				IsHiring:        true,
				HiringURL:       careerURL,
			}

			if _, err := s.repoRepo.Upsert(ctx, repo); err != nil {
				slog.Warn("hiring-seed: failed to upsert", "repo", fullName, "error", err)
				failed++
			} else {
				slog.Info("hiring-seed: seeded", "repo", fullName, "career_url", careerURL)
				seeded++
			}

			progressFn(i+1, len(rows))
			time.Sleep(500 * time.Millisecond)
		}

		slog.Info("hiring-seed completed", "seeded", seeded, "failed", failed, "total", len(rows))
		return nil
	})
}
