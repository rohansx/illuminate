package service

import (
	"context"
	"fmt"
	"log/slog"
	"math"
	"strings"
	"time"

	"github.com/google/uuid"
	"github.com/rohansx/illuminate/api/internal/model"
	"github.com/rohansx/illuminate/api/internal/repository"
)

type IssueService struct {
	issueRepo repository.IssueRepo
	repoRepo  repository.RepoRepo
	github    *GitHubService
}

func NewIssueService(
	issueRepo repository.IssueRepo,
	repoRepo repository.RepoRepo,
	github *GitHubService,
) *IssueService {
	return &IssueService{
		issueRepo: issueRepo,
		repoRepo:  repoRepo,
		github:    github,
	}
}

func (s *IssueService) GetByID(ctx context.Context, id uuid.UUID) (*model.Issue, error) {
	issue, err := s.issueRepo.GetByID(ctx, id)
	if err != nil {
		return nil, fmt.Errorf("getting issue: %w", err)
	}
	return issue, nil
}

func (s *IssueService) GetFeed(ctx context.Context, filter model.FeedFilter, page, perPage int) (*model.IssueFeed, error) {
	if page < 1 {
		page = 1
	}
	if perPage < 1 || perPage > 50 {
		perPage = 20
	}
	offset := (page - 1) * perPage

	issues, totalCount, err := s.issueRepo.GetFeed(ctx, filter, perPage, offset)
	if err != nil {
		return nil, fmt.Errorf("getting feed: %w", err)
	}

	return &model.IssueFeed{
		Issues:     issues,
		TotalCount: totalCount,
		Page:       page,
		PerPage:    perPage,
	}, nil
}

func (s *IssueService) GetCategories(ctx context.Context) ([]model.Category, error) {
	return s.repoRepo.GetCategories(ctx)
}

func (s *IssueService) Search(ctx context.Context, query string, page, perPage int) (*model.IssueFeed, error) {
	if page < 1 {
		page = 1
	}
	if perPage < 1 || perPage > 50 {
		perPage = 20
	}
	offset := (page - 1) * perPage

	issues, totalCount, err := s.issueRepo.Search(ctx, query, perPage, offset)
	if err != nil {
		return nil, fmt.Errorf("searching issues: %w", err)
	}

	return &model.IssueFeed{
		Issues:     issues,
		TotalCount: totalCount,
		Page:       page,
		PerPage:    perPage,
	}, nil
}

func (s *IssueService) GetComments(ctx context.Context, issueID uuid.UUID) ([]GitHubComment, error) {
	issue, err := s.issueRepo.GetByID(ctx, issueID)
	if err != nil {
		return nil, fmt.Errorf("getting issue: %w", err)
	}
	if issue == nil {
		return nil, fmt.Errorf("issue not found")
	}
	return s.github.GetPublicIssueComments(ctx, issue.Repo.Owner, issue.Repo.Name, issue.Number)
}

func (s *IssueService) IndexRepository(ctx context.Context, owner, name string) error {
	ghRepo, err := s.github.GetPublicRepo(ctx, owner, name)
	if err != nil {
		return fmt.Errorf("fetching repo %s/%s: %w", owner, name, err)
	}

	languages, err := s.github.GetPublicRepoLanguages(ctx, owner, name)
	if err != nil {
		slog.Warn("failed to get languages", "repo", owner+"/"+name, "error", err)
	}

	var lastCommit *time.Time
	if ghRepo.PushedAt != "" {
		t, err := time.Parse(time.RFC3339, ghRepo.PushedAt)
		if err == nil {
			lastCommit = &t
		}
	}

	// Check for CONTRIBUTING.md
	hasContributing := false
	if _, err := s.github.GetPublicFileContent(ctx, owner, name, "CONTRIBUTING.md"); err == nil {
		hasContributing = true
	}

	healthScore := computeHealthScore(ghRepo, lastCommit)
	if hasContributing && healthScore < 0.95 {
		healthScore += 0.05
	}

	// Detect hiring signals
	isHiring, hiringURL := detectHiring(ghRepo)
	if !isHiring {
		if _, err := s.github.GetPublicFileContent(ctx, owner, name, "HIRING.md"); err == nil {
			isHiring = true
			hiringURL = fmt.Sprintf("https://github.com/%s/%s/blob/main/HIRING.md", owner, name)
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
		HasContributing: hasContributing,
		HealthScore:     healthScore,
		LastCommitAt:    lastCommit,
		IsHiring:        isHiring,
		HiringURL:       hiringURL,
	}

	repo, err = s.repoRepo.Upsert(ctx, repo)
	if err != nil {
		return fmt.Errorf("upserting repo: %w", err)
	}

	// Fetch and index issues
	ghIssues, err := s.github.GetPublicRepoIssues(ctx, owner, name)
	if err != nil {
		return fmt.Errorf("fetching issues: %w", err)
	}

	for _, gi := range ghIssues {
		labels := make([]string, len(gi.Labels))
		for i, l := range gi.Labels {
			labels[i] = l.Name
		}

		issue := &model.Issue{
			GitHubID:       gi.ID,
			RepoID:         repo.ID,
			Number:         gi.Number,
			Title:          gi.Title,
			Body:           gi.Body,
			Labels:         labels,
			Difficulty:     classifyDifficulty(labels),
			TimeEstimate:   estimateTime(labels),
			Status:         "open",
			CommentCount:   gi.Comments,
			FreshnessScore: computeFreshnessScore(gi.UpdatedAt),
		}

		issue, err = s.issueRepo.Upsert(ctx, issue)
		if err != nil {
			slog.Warn("failed to upsert issue", "issue", gi.Number, "error", err)
			continue
		}

		// Set skills based on repo languages
		var skills []model.IssueSkill
		if ghRepo.Language != "" {
			skills = append(skills, model.IssueSkill{Language: ghRepo.Language})
		}
		for lang := range languages {
			if lang != ghRepo.Language {
				skills = append(skills, model.IssueSkill{Language: lang})
			}
		}

		if err := s.issueRepo.SetSkills(ctx, issue.ID, skills); err != nil {
			slog.Warn("failed to set issue skills", "issue_id", issue.ID, "error", err)
		}
	}

	slog.Info("indexed repository", "repo", owner+"/"+name, "issues", len(ghIssues))
	return nil
}

func (s *IssueService) IndexAllRepositories(ctx context.Context) error {
	repos, err := s.repoRepo.GetAll(ctx)
	if err != nil {
		return fmt.Errorf("getting all repos: %w", err)
	}

	for _, repo := range repos {
		if err := s.IndexRepository(ctx, repo.Owner, repo.Name); err != nil {
			slog.Warn("failed to index repository", "repo", repo.FullName(), "error", err)
			continue
		}
	}
	return nil
}

func computeHealthScore(repo *GitHubRepo, lastCommit *time.Time) float32 {
	var score float32

	// Stars (0-0.20): logarithmic scale
	if repo.StargazersCount > 0 {
		starScore := float32(math.Log10(float64(repo.StargazersCount))) / 5.0
		if starScore > 0.20 {
			starScore = 0.20
		}
		score += starScore
	}

	// Recency (0-0.25): days since last commit
	if lastCommit != nil {
		daysSince := time.Since(*lastCommit).Hours() / 24
		switch {
		case daysSince < 7:
			score += 0.25
		case daysSince < 30:
			score += 0.20
		case daysSince < 90:
			score += 0.12
		case daysSince < 365:
			score += 0.05
		}
	}

	// Has issues enabled (0.05)
	if repo.HasIssues {
		score += 0.05
	}

	// Has topics (0.05)
	if len(repo.Topics) > 0 {
		score += 0.05
	}

	// Has description (0.05)
	if repo.Description != "" {
		score += 0.05
	}

	// License (0.10)
	if repo.License != nil && repo.License.Key != "" {
		score += 0.10
	}

	// Fork ratio (0-0.10): forks/stars indicates community engagement
	if repo.StargazersCount > 0 && repo.ForksCount > 0 {
		forkRatio := float64(repo.ForksCount) / float64(repo.StargazersCount)
		forkScore := float32(math.Min(forkRatio*2.0, 1.0)) * 0.10
		score += forkScore
	}

	// Open issues ratio (0-0.10): moderate open issues = active project
	if repo.OpenIssuesCount > 0 && repo.StargazersCount > 0 {
		ratio := float64(repo.OpenIssuesCount) / float64(repo.StargazersCount)
		switch {
		case ratio < 0.01:
			score += 0.05
		case ratio < 0.05:
			score += 0.10
		case ratio < 0.15:
			score += 0.07
		default:
			score += 0.02
		}
	}

	if score > 1.0 {
		score = 1.0
	}
	return score
}

func computeFreshnessScore(updatedAt string) float32 {
	t, err := time.Parse(time.RFC3339, updatedAt)
	if err != nil {
		return 0.5
	}
	daysSince := time.Since(t).Hours() / 24
	if daysSince < 1 {
		return 1.0
	} else if daysSince < 7 {
		return 0.9
	} else if daysSince < 30 {
		return 0.7
	} else if daysSince < 90 {
		return 0.5
	} else if daysSince < 180 {
		return 0.3
	}
	return 0.1
}

func classifyDifficulty(labels []string) int {
	for _, l := range labels {
		lower := strings.ToLower(l)
		if strings.Contains(lower, "good first issue") || strings.Contains(lower, "beginner") || strings.Contains(lower, "easy") || strings.Contains(lower, "starter") {
			return 1
		}
		if strings.Contains(lower, "help wanted") || strings.Contains(lower, "medium") {
			return 2
		}
		if strings.Contains(lower, "hard") || strings.Contains(lower, "advanced") {
			return 3
		}
	}
	return 2 // default medium
}

func estimateTime(labels []string) string {
	for _, l := range labels {
		lower := strings.ToLower(l)
		if strings.Contains(lower, "good first issue") || strings.Contains(lower, "beginner") || strings.Contains(lower, "easy") {
			return "1-2 hours"
		}
		if strings.Contains(lower, "hard") || strings.Contains(lower, "advanced") {
			return "4-8 hours"
		}
	}
	return "2-4 hours"
}

func detectHiring(repo *GitHubRepo) (bool, string) {
	for _, topic := range repo.Topics {
		lower := strings.ToLower(topic)
		if lower == "hiring" || lower == "jobs" || lower == "careers" ||
			lower == "we-are-hiring" || lower == "work-with-us" {
			return true, fmt.Sprintf("https://github.com/%s/%s", repo.Owner.Login, repo.Name)
		}
	}

	descLower := strings.ToLower(repo.Description)
	for _, kw := range []string{"hiring", "we're hiring", "join our team", "open positions", "careers"} {
		if strings.Contains(descLower, kw) {
			return true, fmt.Sprintf("https://github.com/%s/%s", repo.Owner.Login, repo.Name)
		}
	}

	return false, ""
}

func (s *IssueService) GetHiringRepos(ctx context.Context, page, perPage int) ([]model.Repository, int, error) {
	if page < 1 {
		page = 1
	}
	if perPage < 1 || perPage > 50 {
		perPage = 20
	}
	offset := (page - 1) * perPage
	return s.repoRepo.GetHiring(ctx, perPage, offset)
}
