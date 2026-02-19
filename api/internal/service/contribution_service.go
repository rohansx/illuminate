package service

import (
	"context"
	"fmt"
	"log/slog"
	"strings"
	"time"

	"github.com/google/uuid"
	"github.com/rohansx/illuminate/api/internal/crypto"
	"github.com/rohansx/illuminate/api/internal/model"
	"github.com/rohansx/illuminate/api/internal/repository"
)

type ContributionService struct {
	contribRepo   repository.ContributionRepo
	userRepo      repository.UserRepo
	githubService *GitHubService
	encryptor     *crypto.Encryptor
	jobManager    *JobManager
}

func NewContributionService(
	contribRepo repository.ContributionRepo,
	userRepo repository.UserRepo,
	githubService *GitHubService,
	encryptor *crypto.Encryptor,
	jobManager *JobManager,
) *ContributionService {
	return &ContributionService{
		contribRepo:   contribRepo,
		userRepo:      userRepo,
		githubService: githubService,
		encryptor:     encryptor,
		jobManager:    jobManager,
	}
}

// SyncAll runs a batch sync for all users as a background job.
func (s *ContributionService) SyncAll(ctx context.Context) (*model.JobStatus, error) {
	return s.jobManager.StartJob("contribution-sync", func(ctx context.Context, progressFn func(current, total int)) error {
		users, err := s.userRepo.GetAllForSync(ctx)
		if err != nil {
			return fmt.Errorf("getting users for sync: %w", err)
		}

		slog.Info("contribution sync started", "total_users", len(users))
		progressFn(0, len(users))

		synced, skipped, failed := 0, 0, 0
		for i, user := range users {
			// Skip users synced within the last 6 hours
			if user.ContribSyncedAt != nil && time.Since(*user.ContribSyncedAt) < 6*time.Hour {
				skipped++
				progressFn(i+1, len(users))
				continue
			}

			if err := s.syncUser(ctx, user.ID, user.GitHubUsername); err != nil {
				slog.Warn("failed to sync contributions", "user", user.GitHubUsername, "error", err)
				failed++
			} else {
				synced++
			}

			progressFn(i+1, len(users))
			time.Sleep(2 * time.Second)
		}

		slog.Info("contribution sync completed", "synced", synced, "skipped", skipped, "failed", failed)
		return nil
	})
}

// SyncCurrentUser syncs contributions for a single authenticated user.
func (s *ContributionService) SyncCurrentUser(ctx context.Context, userID uuid.UUID) error {
	user, err := s.userRepo.GetByID(ctx, userID)
	if err != nil || user == nil {
		return fmt.Errorf("getting user: %w", err)
	}
	return s.syncUser(ctx, userID, user.GitHubUsername)
}

func (s *ContributionService) syncUser(ctx context.Context, userID uuid.UUID, username string) error {
	slog.Info("syncing contributions", "user", username)

	// Fetch up to 3 pages of merged PRs (90 total)
	for page := 1; page <= 3; page++ {
		result, err := s.githubService.GetUserPRs(ctx, username, true, page, 30)
		if err != nil {
			return fmt.Errorf("fetching PRs page %d: %w", page, err)
		}

		for _, pr := range result.Items {
			owner, name := parseRepoURL(pr.RepositoryURL)
			if owner == "" || name == "" {
				continue
			}

			var mergedAt *time.Time
			if pr.PullRequest != nil && pr.PullRequest.MergedAt != "" {
				t, err := time.Parse(time.RFC3339, pr.PullRequest.MergedAt)
				if err == nil {
					mergedAt = &t
				}
			}

			var labels []string
			for _, l := range pr.Labels {
				labels = append(labels, l.Name)
			}

			c := &model.Contribution{
				UserID:     userID,
				GitHubPRID: pr.ID,
				RepoOwner:  owner,
				RepoName:   name,
				PRNumber:   pr.Number,
				PRTitle:    pr.Title,
				PRURL:      pr.HTMLURL,
				PRState:    "merged",
				Language:   "",
				Labels:     labels,
				MergedAt:   mergedAt,
			}

			if err := s.contribRepo.Upsert(ctx, c); err != nil {
				slog.Warn("failed to upsert contribution", "pr_id", pr.ID, "error", err)
			}
		}

		// Stop if we got fewer results than requested
		if len(result.Items) < 30 {
			break
		}
	}

	// Mark user as synced
	if err := s.userRepo.UpdateContributionsSyncedAt(ctx, userID); err != nil {
		slog.Warn("failed to update sync timestamp", "user", username, "error", err)
	}

	return nil
}

// parseRepoURL extracts owner/name from "https://api.github.com/repos/owner/name"
func parseRepoURL(repoURL string) (string, string) {
	const prefix = "https://api.github.com/repos/"
	if !strings.HasPrefix(repoURL, prefix) {
		return "", ""
	}
	parts := strings.SplitN(strings.TrimPrefix(repoURL, prefix), "/", 2)
	if len(parts) != 2 {
		return "", ""
	}
	return parts[0], parts[1]
}

// GetTimeline returns paginated contributions for a user.
func (s *ContributionService) GetTimeline(ctx context.Context, userID uuid.UUID, page, perPage int) (*model.ContributionFeed, error) {
	if page < 1 {
		page = 1
	}
	if perPage < 1 || perPage > 100 {
		perPage = 20
	}
	offset := (page - 1) * perPage
	return s.contribRepo.GetByUserID(ctx, userID, perPage, offset)
}

// GetProjects returns contributions grouped by repository.
func (s *ContributionService) GetProjects(ctx context.Context, userID uuid.UUID) ([]model.ProjectGroup, error) {
	return s.contribRepo.GetProjectGroups(ctx, userID)
}

// GetPortfolioStats returns aggregate contribution statistics.
func (s *ContributionService) GetPortfolioStats(ctx context.Context, userID uuid.UUID) (*model.PortfolioStats, error) {
	return s.contribRepo.GetStats(ctx, userID)
}

// GetPublicProfile assembles a public profile from a username.
func (s *ContributionService) GetPublicProfile(ctx context.Context, username string) (*model.PublicProfile, error) {
	user, err := s.userRepo.GetByUsername(ctx, username)
	if err != nil {
		return nil, fmt.Errorf("getting user by username: %w", err)
	}
	if user == nil {
		return nil, nil
	}

	stats, err := s.contribRepo.GetStatsByUsername(ctx, username)
	if err != nil {
		return nil, fmt.Errorf("getting stats: %w", err)
	}

	topProjects, err := s.contribRepo.GetProjectGroupsByUsername(ctx, username, 5)
	if err != nil {
		return nil, fmt.Errorf("getting top projects: %w", err)
	}

	recentPRs, err := s.contribRepo.GetByUsername(ctx, username, 10)
	if err != nil {
		return nil, fmt.Errorf("getting recent PRs: %w", err)
	}

	return &model.PublicProfile{
		User: model.PublicUser{
			GitHubUsername: user.GitHubUsername,
			AvatarURL:      user.AvatarURL,
			Bio:            user.Bio,
			Skills:         user.Skills,
			CreatedAt:      user.CreatedAt,
		},
		Stats:       *stats,
		TopProjects: topProjects,
		RecentPRs:   recentPRs,
	}, nil
}
