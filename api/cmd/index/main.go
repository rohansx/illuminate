package main

import (
	"context"
	"log"
	"log/slog"
	"os"
	"time"

	"github.com/rohansx/illuminate/api/internal/config"
	"github.com/rohansx/illuminate/api/internal/database"
	"github.com/rohansx/illuminate/api/internal/repository"
	"github.com/rohansx/illuminate/api/internal/service"
)

func main() {
	logger := slog.New(slog.NewJSONHandler(os.Stdout, nil))
	slog.SetDefault(logger)

	cfg, err := config.Load()
	if err != nil {
		log.Fatalf("loading config: %v", err)
	}

	ctx := context.Background()
	pool, err := database.NewPool(ctx, cfg.DatabaseURL)
	if err != nil {
		log.Fatalf("connecting to database: %v", err)
	}
	defer pool.Close()

	repoRepo := repository.NewRepoRepo(pool)
	issueRepo := repository.NewIssueRepo(pool)
	githubService := service.NewGitHubService(cfg.GitHubClientID, cfg.GitHubClientSecret, "")
	issueService := service.NewIssueService(issueRepo, repoRepo, githubService)

	repos, err := repoRepo.GetAll(ctx)
	if err != nil {
		log.Fatalf("getting repos: %v", err)
	}

	slog.Info("indexing issues", "repo_count", len(repos))

	for i, repo := range repos {
		if err := issueService.IndexRepository(ctx, repo.Owner, repo.Name); err != nil {
			slog.Warn("failed to index", "repo", repo.FullName(), "error", err)
			continue
		}

		slog.Info("indexed repo", "repo", repo.FullName(), "progress", i+1, "total", len(repos))

		// Rate limit: ~1 req/sec to GitHub
		time.Sleep(time.Second)
	}

	slog.Info("indexing complete")
}
