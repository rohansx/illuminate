package service

import (
	"context"
	"fmt"
	"log/slog"
	"sync"
	"time"

	"github.com/rohansx/illuminate/api/internal/model"
	"github.com/rohansx/illuminate/api/internal/repository"
)

var discoveryQueries = []struct {
	Query string
	Label string
}{
	{Query: "good-first-issues:>10 stars:>500 pushed:>2025-01-01", Label: "good-first-issues-popular"},
	{Query: "good-first-issues:>3 stars:>100 pushed:>2025-01-01", Label: "good-first-issues-emerging"},
	{Query: "help-wanted-issues:>5 stars:>200 pushed:>2025-01-01", Label: "help-wanted"},
	{Query: "topic:hacktoberfest stars:>200 pushed:>2025-01-01", Label: "hacktoberfest"},
	{Query: "topic:good-first-issue stars:>100 pushed:>2025-01-01", Label: "topic-gfi"},
	{Query: "topic:beginner-friendly stars:>50 pushed:>2025-01-01", Label: "beginner-friendly"},
	{Query: "good-first-issues:>3 language:python stars:>100", Label: "python"},
	{Query: "good-first-issues:>3 language:javascript stars:>100", Label: "javascript"},
	{Query: "good-first-issues:>3 language:typescript stars:>100", Label: "typescript"},
	{Query: "good-first-issues:>3 language:go stars:>100", Label: "go"},
	{Query: "good-first-issues:>3 language:rust stars:>100", Label: "rust"},
	{Query: "good-first-issues:>3 language:java stars:>100", Label: "java"},
	{Query: "good-first-issues:>5 stars:>300 sort:updated", Label: "recently-updated"},
	{Query: "help-wanted-issues:>10 stars:>1000", Label: "major-projects"},
}

const (
	queriesPerRun  = 6
	maxNewPerRun   = 50
	searchSleep    = 2 * time.Second
	seedSleep      = 500 * time.Millisecond
	staleCutoffMon = 6
)

type DiscoveryService struct {
	repoRepo      repository.RepoRepo
	issueService  *IssueService
	githubService *GitHubService
	jobManager    *JobManager

	mu          sync.Mutex
	queryOffset int
}

func NewDiscoveryService(
	repoRepo repository.RepoRepo,
	issueService *IssueService,
	githubService *GitHubService,
	jobManager *JobManager,
) *DiscoveryService {
	return &DiscoveryService{
		repoRepo:      repoRepo,
		issueService:  issueService,
		githubService: githubService,
		jobManager:    jobManager,
	}
}

func (s *DiscoveryService) Discover(ctx context.Context) (*model.JobStatus, error) {
	s.mu.Lock()
	offset := s.queryOffset
	s.queryOffset = (s.queryOffset + queriesPerRun) % len(discoveryQueries)
	s.mu.Unlock()

	// Pick queries for this run
	queries := make([]struct {
		Query string
		Label string
	}, queriesPerRun)
	for i := 0; i < queriesPerRun; i++ {
		queries[i] = discoveryQueries[(offset+i)%len(discoveryQueries)]
	}

	return s.jobManager.StartJob("discovery", func(ctx context.Context, progressFn func(current, total int)) error {
		// Load existing github IDs for dedup
		existingIDs, err := s.repoRepo.GetAllGitHubIDs(ctx)
		if err != nil {
			return fmt.Errorf("loading existing github ids: %w", err)
		}
		knownIDs := make(map[int64]bool, len(existingIDs))
		for _, id := range existingIDs {
			knownIDs[id] = true
		}

		slog.Info("discovery started", "existing_repos", len(existingIDs), "queries", queriesPerRun)

		// Collect new repos from search results
		type newRepo struct {
			Owner string
			Name  string
			Repo  GitHubRepo
		}
		var newRepos []newRepo
		staleCutoff := time.Now().AddDate(0, -staleCutoffMon, 0)

		for i, q := range queries {
			slog.Info("discovery: searching", "query", q.Label, "progress", fmt.Sprintf("%d/%d", i+1, len(queries)))

			result, err := s.githubService.SearchRepositories(ctx, q.Query, 1)
			if err != nil {
				slog.Warn("discovery: search failed", "query", q.Label, "error", err)
				time.Sleep(searchSleep)
				continue
			}

			for _, ghRepo := range result.Items {
				if knownIDs[ghRepo.ID] {
					continue
				}
				if ghRepo.StargazersCount < 50 {
					continue
				}
				// Check staleness
				if ghRepo.PushedAt != "" {
					pushed, err := time.Parse(time.RFC3339, ghRepo.PushedAt)
					if err == nil && pushed.Before(staleCutoff) {
						continue
					}
				}
				// Mark as known to dedup within this run
				knownIDs[ghRepo.ID] = true
				newRepos = append(newRepos, newRepo{
					Owner: ghRepo.Owner.Login,
					Name:  ghRepo.Name,
					Repo:  ghRepo,
				})
				if len(newRepos) >= maxNewPerRun {
					break
				}
			}

			if len(newRepos) >= maxNewPerRun {
				break
			}
			time.Sleep(searchSleep)
		}

		slog.Info("discovery: found new repos", "count", len(newRepos))
		if len(newRepos) == 0 {
			progressFn(1, 1)
			return nil
		}

		// Seed and index each new repo
		seeded, failed := 0, 0
		progressFn(0, len(newRepos))

		for i, nr := range newRepos {
			fullName := nr.Owner + "/" + nr.Name

			var lastCommit *time.Time
			if nr.Repo.PushedAt != "" {
				t, err := time.Parse(time.RFC3339, nr.Repo.PushedAt)
				if err == nil {
					lastCommit = &t
				}
			}

			repo := &model.Repository{
				GitHubID:        nr.Repo.ID,
				Owner:           nr.Owner,
				Name:            nr.Name,
				Description:     nr.Repo.Description,
				Stars:           nr.Repo.StargazersCount,
				PrimaryLanguage: nr.Repo.Language,
				Topics:          nr.Repo.Topics,
				HealthScore:     0.5,
				LastCommitAt:    lastCommit,
			}

			if _, err := s.repoRepo.Upsert(ctx, repo); err != nil {
				slog.Warn("discovery: failed to upsert", "repo", fullName, "error", err)
				failed++
				progressFn(i+1, len(newRepos))
				time.Sleep(seedSleep)
				continue
			}

			// Index issues immediately
			if err := s.issueService.IndexRepository(ctx, nr.Owner, nr.Name); err != nil {
				slog.Warn("discovery: failed to index", "repo", fullName, "error", err)
			}

			slog.Info("discovery: seeded+indexed", "repo", fullName, "stars", nr.Repo.StargazersCount, "language", nr.Repo.Language)
			seeded++
			progressFn(i+1, len(newRepos))
			time.Sleep(seedSleep)
		}

		slog.Info("discovery completed", "seeded", seeded, "failed", failed, "total_found", len(newRepos))
		return nil
	})
}
