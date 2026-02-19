package service

import (
	"context"
	"fmt"
	"math"
	"sort"
	"time"

	"github.com/google/uuid"
	"github.com/rohansx/illuminate/api/internal/model"
	"github.com/rohansx/illuminate/api/internal/repository"
)

type GrowthService struct {
	contribRepo repository.ContributionRepo
	userRepo    repository.UserRepo
}

func NewGrowthService(contribRepo repository.ContributionRepo, userRepo repository.UserRepo) *GrowthService {
	return &GrowthService{contribRepo: contribRepo, userRepo: userRepo}
}

func (s *GrowthService) GetGrowthProfile(ctx context.Context, userID uuid.UUID) (*model.GrowthProfile, error) {
	user, err := s.userRepo.GetByID(ctx, userID)
	if err != nil || user == nil {
		return nil, fmt.Errorf("getting user: %w", err)
	}

	stats, err := s.contribRepo.GetStats(ctx, userID)
	if err != nil {
		return nil, fmt.Errorf("getting stats: %w", err)
	}

	projects, err := s.contribRepo.GetProjectGroups(ctx, userID)
	if err != nil {
		return nil, fmt.Errorf("getting projects: %w", err)
	}

	maxRepoPRs := 0
	for _, p := range projects {
		if p.PRCount > maxRepoPRs {
			maxRepoPRs = p.PRCount
		}
	}

	level := calculateLevel(stats, maxRepoPRs)
	levelIdx := levelIndex(level)

	profile := &model.GrowthProfile{
		Level:      level,
		LevelName:  model.LevelNames[level],
		LevelIndex: levelIdx,
		Radar:      calculateRadar(stats, maxRepoPRs),
		NextSteps:  calculateNextSteps(stats, projects, level, maxRepoPRs),
	}

	if levelIdx < len(model.LevelOrder)-1 {
		next := model.LevelOrder[levelIdx+1]
		profile.NextLevel = &next
		profile.NextLevelName = model.LevelNames[next]
		profile.Progress = calculateProgress(level, stats, maxRepoPRs)
	} else {
		profile.Progress = model.LevelProgress{
			CurrentValue: stats.TotalRepos,
			TargetValue:  stats.TotalRepos,
			Metric:       "you've reached the top",
			Percentage:   100,
		}
	}

	return profile, nil
}

func calculateLevel(stats *model.PortfolioStats, maxRepoPRs int) model.GrowthLevel {
	if stats.TotalRepos >= 10 {
		return model.LevelLuminary
	}
	if maxRepoPRs >= 5 {
		return model.LevelSpecialist
	}
	if stats.LongestStreak >= 4 {
		return model.LevelRegular
	}
	if stats.TotalRepos >= 3 {
		return model.LevelContributor
	}
	if stats.TotalPRs >= 1 {
		return model.LevelFirstLight
	}
	return model.LevelExplorer
}

func levelIndex(level model.GrowthLevel) int {
	for i, l := range model.LevelOrder {
		if l == level {
			return i
		}
	}
	return 0
}

func calculateProgress(level model.GrowthLevel, stats *model.PortfolioStats, maxRepoPRs int) model.LevelProgress {
	switch level {
	case model.LevelExplorer:
		return model.LevelProgress{
			CurrentValue: stats.TotalPRs,
			TargetValue:  1,
			Metric:       "merged PRs",
			Percentage:   clamp(stats.TotalPRs * 100),
		}
	case model.LevelFirstLight:
		return model.LevelProgress{
			CurrentValue: stats.TotalRepos,
			TargetValue:  3,
			Metric:       "repos contributed to",
			Percentage:   clamp(stats.TotalRepos * 100 / 3),
		}
	case model.LevelContributor:
		return model.LevelProgress{
			CurrentValue: stats.LongestStreak,
			TargetValue:  4,
			Metric:       "week longest streak",
			Percentage:   clamp(stats.LongestStreak * 100 / 4),
		}
	case model.LevelRegular:
		return model.LevelProgress{
			CurrentValue: maxRepoPRs,
			TargetValue:  5,
			Metric:       "PRs in a single repo",
			Percentage:   clamp(maxRepoPRs * 100 / 5),
		}
	case model.LevelSpecialist:
		return model.LevelProgress{
			CurrentValue: stats.TotalRepos,
			TargetValue:  10,
			Metric:       "repos contributed to",
			Percentage:   clamp(stats.TotalRepos * 100 / 10),
		}
	}
	return model.LevelProgress{Percentage: 100, Metric: "you've reached the top"}
}

func calculateRadar(stats *model.PortfolioStats, maxRepoPRs int) model.RadarScores {
	recency := 0
	if stats.LatestContrib != nil {
		daysSince := int(time.Since(*stats.LatestContrib).Hours() / 24)
		if daysSince <= 0 {
			recency = 100
		} else if daysSince >= 90 {
			recency = 0
		} else {
			recency = 100 - (daysSince * 100 / 90)
		}
	}

	return model.RadarScores{
		Volume:      scaleSqrt(stats.TotalPRs, 50),
		Breadth:     scaleSqrt(stats.TotalRepos, 15),
		Consistency: scaleSqrt(stats.LongestStreak, 12),
		Depth:       scaleSqrt(maxRepoPRs, 20),
		Diversity:   scaleSqrt(len(stats.Languages), 6),
		Recency:     recency,
	}
}

func scaleSqrt(value, max int) int {
	if value <= 0 {
		return 0
	}
	if value >= max {
		return 100
	}
	ratio := float64(value) / float64(max)
	score := int(math.Sqrt(ratio) * 100)
	if score > 100 {
		return 100
	}
	return score
}

func calculateNextSteps(stats *model.PortfolioStats, projects []model.ProjectGroup, level model.GrowthLevel, maxRepoPRs int) []model.NextStep {
	var steps []model.NextStep
	numLanguages := len(stats.Languages)

	if stats.TotalPRs == 0 {
		steps = append(steps, model.NextStep{
			ID:          "first_pr",
			Title:       "Submit your first PR",
			Description: "Browse the issue feed and find a good-first-issue to get started.",
			Priority:    1,
		})
	}

	if numLanguages == 1 && stats.TotalPRs >= 3 {
		steps = append(steps, model.NextStep{
			ID:          "try_new_language",
			Title:       "Try a new language",
			Description: "You've only contributed in one language. Explore issues in a different ecosystem.",
			Priority:    2,
		})
	}

	if stats.TotalRepos <= 2 && stats.TotalPRs >= 3 {
		steps = append(steps, model.NextStep{
			ID:          "explore_new_repos",
			Title:       "Contribute to a new project",
			Description: "Broaden your impact by contributing to a project you haven't touched before.",
			Priority:    2,
		})
	}

	if stats.LongestStreak < 4 && stats.TotalPRs >= 3 {
		steps = append(steps, model.NextStep{
			ID:          "build_streak",
			Title:       "Build a contribution streak",
			Description: "Try contributing at least once a week for 4 weeks to level up.",
			Priority:    3,
		})
	}

	if maxRepoPRs < 5 && stats.TotalRepos >= 3 {
		steps = append(steps, model.NextStep{
			ID:          "go_deeper",
			Title:       "Go deeper in a project",
			Description: "Pick a project you like and aim for 5+ PRs to become a specialist.",
			Priority:    3,
		})
	}

	if level == model.LevelContributor && stats.LongestStreak >= 2 && stats.LongestStreak < 4 {
		steps = append(steps, model.NextStep{
			ID:          "streak_push",
			Title:       "Keep your streak going",
			Description: fmt.Sprintf("You're at %d weeks — just %d more to reach Regular.", stats.LongestStreak, 4-stats.LongestStreak),
			Priority:    1,
		})
	}

	if level == model.LevelSpecialist && stats.TotalRepos >= 7 && stats.TotalRepos < 10 {
		steps = append(steps, model.NextStep{
			ID:          "luminary_push",
			Title:       "Reach Luminary status",
			Description: fmt.Sprintf("You've contributed to %d repos — just %d more to reach Luminary.", stats.TotalRepos, 10-stats.TotalRepos),
			Priority:    1,
		})
	}

	sort.Slice(steps, func(i, j int) bool {
		return steps[i].Priority < steps[j].Priority
	})
	if len(steps) > 3 {
		steps = steps[:3]
	}

	if len(steps) == 0 {
		steps = append(steps, model.NextStep{
			ID:          "keep_going",
			Title:       "Keep up the momentum",
			Description: "You're at the highest level. Continue contributing and mentoring others.",
			Priority:    1,
		})
	}

	return steps
}

func clamp(v int) int {
	if v > 100 {
		return 100
	}
	if v < 0 {
		return 0
	}
	return v
}
