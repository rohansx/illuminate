package service

import (
	"strings"

	"github.com/rohansx/illuminate/api/internal/model"
)

// Matching weights
const (
	weightSkillMatch    = 0.35
	weightGrowthMatch   = 0.20
	weightRepoHealth    = 0.20
	weightFreshness     = 0.15
	weightLowCompetition = 0.10
)

type MatchingService struct{}

func NewMatchingService() *MatchingService {
	return &MatchingService{}
}

// ScoreIssue computes a match score (0.0–1.0) for a user/issue pair.
func (s *MatchingService) ScoreIssue(user *model.User, issue *model.Issue) (float64, []string) {
	var reasons []string

	// 1. Skill match: does the user know the languages this issue requires?
	skillScore := s.computeSkillMatch(user, issue)
	if skillScore > 0.5 {
		reasons = append(reasons, "Matches your skills")
	}

	// 2. Growth match: is this issue slightly above the user's comfort level?
	growthScore := s.computeGrowthMatch(user, issue)
	if growthScore > 0.5 {
		reasons = append(reasons, "Good for growth")
	}

	// 3. Repo health
	repoHealthScore := float64(0)
	if issue.Repo != nil {
		repoHealthScore = float64(issue.Repo.HealthScore)
		if repoHealthScore > 0.7 {
			reasons = append(reasons, "Active, healthy repo")
		}
	}

	// 4. Freshness
	freshnessScore := float64(issue.FreshnessScore)
	if freshnessScore > 0.7 {
		reasons = append(reasons, "Recently updated")
	}

	// 5. Low competition (fewer comments = less likely someone is already working on it)
	competitionScore := s.computeCompetitionScore(issue)
	if competitionScore > 0.7 {
		reasons = append(reasons, "Low competition")
	}

	total := (skillScore * weightSkillMatch) +
		(growthScore * weightGrowthMatch) +
		(repoHealthScore * weightRepoHealth) +
		(freshnessScore * weightFreshness) +
		(competitionScore * weightLowCompetition)

	return total, reasons
}

// ScoreIssues scores and sorts a slice of issues for a given user.
func (s *MatchingService) ScoreIssues(user *model.User, issues []model.Issue) []model.Issue {
	for i := range issues {
		score, reasons := s.ScoreIssue(user, &issues[i])
		issues[i].MatchScore = score
		issues[i].MatchReasons = reasons
	}

	// Sort by match score descending (simple insertion sort, good enough for page-sized slices)
	for i := 1; i < len(issues); i++ {
		for j := i; j > 0 && issues[j].MatchScore > issues[j-1].MatchScore; j-- {
			issues[j], issues[j-1] = issues[j-1], issues[j]
		}
	}

	return issues
}

func (s *MatchingService) computeSkillMatch(user *model.User, issue *model.Issue) float64 {
	if len(user.Skills) == 0 || len(issue.Skills) == 0 {
		return 0.5 // neutral if we don't have data
	}

	userSkills := make(map[string]float32)
	for _, skill := range user.Skills {
		userSkills[strings.ToLower(skill.Language)] = skill.Proficiency
	}

	var bestMatch float64
	for _, issueSkill := range issue.Skills {
		if proficiency, ok := userSkills[strings.ToLower(issueSkill.Language)]; ok {
			match := float64(proficiency)
			if match > bestMatch {
				bestMatch = match
			}
		}
	}

	return bestMatch
}

func (s *MatchingService) computeGrowthMatch(user *model.User, issue *model.Issue) float64 {
	// Map comfort level to a numeric value
	comfortMap := map[string]int{
		"beginner":     1,
		"intermediate": 2,
		"advanced":     3,
	}

	userLevel, ok := comfortMap[strings.ToLower(user.ComfortLevel)]
	if !ok {
		userLevel = 1
	}

	// Ideal: issue difficulty is at or one level above user comfort
	diff := issue.Difficulty - userLevel
	switch {
	case diff == 0:
		return 0.8 // perfect match
	case diff == 1:
		return 1.0 // slight stretch = best for growth
	case diff == -1:
		return 0.5 // slightly easy, still fine
	case diff >= 2:
		return 0.2 // too hard
	default:
		return 0.3 // too easy
	}
}

func (s *MatchingService) computeCompetitionScore(issue *model.Issue) float64 {
	switch {
	case issue.CommentCount == 0:
		return 1.0
	case issue.CommentCount <= 2:
		return 0.8
	case issue.CommentCount <= 5:
		return 0.5
	case issue.CommentCount <= 10:
		return 0.3
	default:
		return 0.1
	}
}
