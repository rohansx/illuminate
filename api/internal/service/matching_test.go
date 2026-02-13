package service

import (
	"testing"

	"github.com/rohansx/illuminate/api/internal/model"
)

func TestScoreIssue(t *testing.T) {
	ms := NewMatchingService()

	tests := []struct {
		name     string
		user     *model.User
		issue    *model.Issue
		wantMin  float64
		wantMax  float64
	}{
		{
			name: "perfect skill match, beginner issue",
			user: &model.User{
				ComfortLevel: "beginner",
				Skills: []model.UserSkill{
					{Language: "Go", Proficiency: 0.8},
				},
			},
			issue: &model.Issue{
				Difficulty:     1,
				FreshnessScore: 0.9,
				CommentCount:   0,
				Skills:         []model.IssueSkill{{Language: "Go"}},
				Repo:           &model.Repository{HealthScore: 0.8},
			},
			wantMin: 0.7,
			wantMax: 1.0,
		},
		{
			name: "no skill overlap",
			user: &model.User{
				ComfortLevel: "beginner",
				Skills: []model.UserSkill{
					{Language: "Python", Proficiency: 0.9},
				},
			},
			issue: &model.Issue{
				Difficulty:     1,
				FreshnessScore: 0.5,
				CommentCount:   3,
				Skills:         []model.IssueSkill{{Language: "Rust"}},
				Repo:           &model.Repository{HealthScore: 0.5},
			},
			wantMin: 0.0,
			wantMax: 0.6,
		},
		{
			name: "growth stretch — intermediate user, advanced issue",
			user: &model.User{
				ComfortLevel: "intermediate",
				Skills: []model.UserSkill{
					{Language: "TypeScript", Proficiency: 0.7},
				},
			},
			issue: &model.Issue{
				Difficulty:     3,
				FreshnessScore: 0.8,
				CommentCount:   1,
				Skills:         []model.IssueSkill{{Language: "TypeScript"}},
				Repo:           &model.Repository{HealthScore: 0.9},
			},
			wantMin: 0.5,
			wantMax: 0.9,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			score, reasons := ms.ScoreIssue(tt.user, tt.issue)
			if score < tt.wantMin || score > tt.wantMax {
				t.Errorf("score = %f, want between %f and %f", score, tt.wantMin, tt.wantMax)
			}
			t.Logf("score=%.3f reasons=%v", score, reasons)
		})
	}
}

func TestScoreIssues_Sorting(t *testing.T) {
	ms := NewMatchingService()

	user := &model.User{
		ComfortLevel: "beginner",
		Skills: []model.UserSkill{
			{Language: "Go", Proficiency: 0.9},
		},
	}

	issues := []model.Issue{
		{
			Title:          "Low match",
			Difficulty:     3,
			FreshnessScore: 0.2,
			CommentCount:   15,
			Skills:         []model.IssueSkill{{Language: "Haskell"}},
			Repo:           &model.Repository{HealthScore: 0.3},
		},
		{
			Title:          "High match",
			Difficulty:     1,
			FreshnessScore: 1.0,
			CommentCount:   0,
			Skills:         []model.IssueSkill{{Language: "Go"}},
			Repo:           &model.Repository{HealthScore: 0.9},
		},
	}

	scored := ms.ScoreIssues(user, issues)
	if scored[0].Title != "High match" {
		t.Errorf("expected 'High match' first, got '%s'", scored[0].Title)
	}
	if scored[0].MatchScore <= scored[1].MatchScore {
		t.Errorf("expected first score > second, got %f <= %f", scored[0].MatchScore, scored[1].MatchScore)
	}
}
