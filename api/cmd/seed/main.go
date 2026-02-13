package main

import (
	"context"
	"encoding/json"
	"log"
	"log/slog"
	"os"
	"time"

	"github.com/rohansx/illuminate/api/internal/config"
	"github.com/rohansx/illuminate/api/internal/database"
	"github.com/rohansx/illuminate/api/internal/model"
	"github.com/rohansx/illuminate/api/internal/repository"
	"github.com/rohansx/illuminate/api/internal/service"
)

type seedRepo struct {
	Owner string `json:"owner"`
	Name  string `json:"name"`
}

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

	// Read seed data
	data, err := os.ReadFile("data/seed_repos.json")
	if err != nil {
		log.Fatalf("reading seed file: %v", err)
	}

	var seeds []seedRepo
	if err := json.Unmarshal(data, &seeds); err != nil {
		log.Fatalf("parsing seed file: %v", err)
	}

	repoRepo := repository.NewRepoRepo(pool)
	githubService := service.NewGitHubService(cfg.GitHubClientID, cfg.GitHubClientSecret, "")

	slog.Info("seeding repositories", "count", len(seeds))

	for i, seed := range seeds {
		ghRepo, err := githubService.GetPublicRepo(ctx, seed.Owner, seed.Name)
		if err != nil {
			slog.Warn("failed to fetch repo", "repo", seed.Owner+"/"+seed.Name, "error", err)
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
			HealthScore:     0.5, // will be computed during indexing
			LastCommitAt:    lastCommit,
		}

		if _, err := repoRepo.Upsert(ctx, repo); err != nil {
			slog.Warn("failed to upsert repo", "repo", seed.Owner+"/"+seed.Name, "error", err)
			continue
		}

		slog.Info("seeded repo", "repo", seed.Owner+"/"+seed.Name, "progress", i+1, "total", len(seeds))

		// Be nice to GitHub API
		time.Sleep(500 * time.Millisecond)
	}

	slog.Info("seeding complete")
}
