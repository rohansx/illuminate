package service

import (
	"context"
	"errors"
	"fmt"
	"log/slog"
	"strings"

	"github.com/google/uuid"
	"github.com/rohansx/illuminate/api/internal/model"
	"github.com/rohansx/illuminate/api/internal/repository"
)

var (
	ErrDeepDiveNotConfigured = errors.New("deep dive feature is not available: GLM API key not configured")
	ErrIssueNotFound         = errors.New("issue not found")
)

type DeepDiveService struct {
	deepDiveRepo repository.DeepDiveRepo
	issueRepo    repository.IssueRepo
	repoRepo     repository.RepoRepo
	userRepo     repository.UserRepo
	github       *GitHubService
	glm          *GLMService
}

func NewDeepDiveService(
	deepDiveRepo repository.DeepDiveRepo,
	issueRepo repository.IssueRepo,
	repoRepo repository.RepoRepo,
	userRepo repository.UserRepo,
	github *GitHubService,
	glm *GLMService,
) *DeepDiveService {
	return &DeepDiveService{
		deepDiveRepo: deepDiveRepo,
		issueRepo:    issueRepo,
		repoRepo:     repoRepo,
		userRepo:     userRepo,
		github:       github,
		glm:          glm,
	}
}

func (s *DeepDiveService) Generate(ctx context.Context, issueID, userID uuid.UUID) (*model.DeepDive, error) {
	if !s.glm.IsConfigured() {
		return nil, ErrDeepDiveNotConfigured
	}

	// 1. Fetch the issue (with repo populated)
	issue, err := s.issueRepo.GetByID(ctx, issueID)
	if err != nil {
		return nil, fmt.Errorf("fetching issue: %w", err)
	}
	if issue == nil {
		return nil, ErrIssueNotFound
	}

	// 2. Check cache — return existing if issue hasn't been re-indexed
	existing, err := s.deepDiveRepo.GetByIssueAndUser(ctx, issueID, userID)
	if err != nil {
		return nil, fmt.Errorf("checking cache: %w", err)
	}
	if existing != nil && !existing.IssueIndexedAt.Before(issue.IndexedAt) {
		return existing, nil
	}

	// 3. Fetch the user's skill profile for calibration
	user, err := s.userRepo.GetByID(ctx, userID)
	if err != nil {
		return nil, fmt.Errorf("fetching user: %w", err)
	}

	// 4. Assemble context from GitHub
	repoCtx := s.assembleRepoContext(ctx, issue.Repo.Owner, issue.Repo.Name)

	// 5. Build the prompt
	system, userMessage := s.buildPrompt(issue, user, repoCtx)

	// 6. Call GLM
	result, err := s.glm.SendMessage(ctx, system, userMessage)
	if err != nil {
		return nil, fmt.Errorf("generating deep dive: %w", err)
	}

	// 7. Parse structured response
	sections := parseSections(result.Text)

	// 8. Store in DB
	dd := &model.DeepDive{
		IssueID:           issueID,
		UserID:            userID,
		IssueIndexedAt:    issue.IndexedAt,
		ProjectOverview:   sections["project_overview"],
		IssueContext:      sections["issue_context"],
		SuggestedApproach: sections["suggested_approach"],
		QuestionsToAsk:    sections["questions_to_ask"],
		RedFlags:          sections["red_flags"],
		FirstComment:      sections["first_comment"],
		ModelUsed:         result.Model,
		PromptTokens:      result.PromptTokens,
		CompletionTokens:  result.CompletionTokens,
	}

	dd, err = s.deepDiveRepo.Upsert(ctx, dd)
	if err != nil {
		return nil, fmt.Errorf("saving deep dive: %w", err)
	}

	return dd, nil
}

type repoContext struct {
	readme       string
	contributing string
	languages    map[string]int
}

func (s *DeepDiveService) assembleRepoContext(ctx context.Context, owner, name string) repoContext {
	rc := repoContext{}

	// Fetch README
	readme, err := s.github.GetPublicFileContent(ctx, owner, name, "README.md")
	if err != nil {
		slog.Debug("no README found", "repo", owner+"/"+name, "error", err)
	} else {
		if len(readme) > 4000 {
			readme = readme[:4000] + "\n... [truncated]"
		}
		rc.readme = readme
	}

	// Fetch CONTRIBUTING.md
	contributing, err := s.github.GetPublicFileContent(ctx, owner, name, "CONTRIBUTING.md")
	if err != nil {
		slog.Debug("no CONTRIBUTING.md found", "repo", owner+"/"+name, "error", err)
	} else {
		if len(contributing) > 3000 {
			contributing = contributing[:3000] + "\n... [truncated]"
		}
		rc.contributing = contributing
	}

	// Fetch languages
	languages, err := s.github.GetPublicRepoLanguages(ctx, owner, name)
	if err != nil {
		slog.Debug("failed to get languages", "repo", owner+"/"+name, "error", err)
	}
	rc.languages = languages

	return rc
}

func (s *DeepDiveService) buildPrompt(issue *model.Issue, user *model.User, rc repoContext) (string, string) {
	system := `You are an expert open-source mentor helping developers contribute to open-source projects. You produce clear, actionable analysis of GitHub issues.

You MUST structure your response with EXACTLY these 6 sections, using these EXACT headers:

## PROJECT_OVERVIEW
What the project does, its tech stack, and how contributions work.

## ISSUE_CONTEXT
A plain-language explanation of what is broken or missing and why it matters.

## SUGGESTED_APPROACH
A numbered checklist of concrete steps for tackling this issue. Format each step as:
1. [ ] Step description
2. [ ] Step description
Include which files/areas to look at and what to test. NOT code, but clear enough to follow.

## QUESTIONS_TO_ASK
3-5 smart questions the contributor should post in the issue thread before starting. Write them as exact copy-pasteable questions — phrased politely and specifically, as if posting on GitHub.

## RED_FLAGS
Warnings about stale issues, duplicate PRs, abandoned repos, or anything else to watch out for. If there are no red flags, say so explicitly.

## FIRST_COMMENT
Write a ready-to-paste GitHub comment that the contributor can post on the issue to express interest and ask for clarification. It should:
- Introduce themselves as interested in working on this
- Mention their relevant skills briefly
- Ask 1-2 of the most important clarifying questions
- Be concise, professional, and friendly

Rules:
- Never write code. Describe approaches in plain language.
- Be honest about difficulty and time estimates.
- Tailor your explanation to the contributor's experience level.
- Be concise but thorough. Each section should be 2-5 sentences or bullet points.`

	// Build user message with all context
	var b strings.Builder

	// User skill profile
	b.WriteString("## CONTRIBUTOR PROFILE\n")
	if user != nil {
		b.WriteString(fmt.Sprintf("- Comfort level: %s\n", user.ComfortLevel))
		b.WriteString(fmt.Sprintf("- Time commitment: %s\n", user.TimeCommitment))
		if len(user.Skills) > 0 {
			b.WriteString("- Skills: ")
			for i, skill := range user.Skills {
				if i > 0 {
					b.WriteString(", ")
				}
				b.WriteString(fmt.Sprintf("%s (%.0f%%)", skill.Language, skill.Proficiency*100))
			}
			b.WriteString("\n")
		}
		if len(user.Goals) > 0 {
			b.WriteString(fmt.Sprintf("- Goals: %s\n", strings.Join(user.Goals, ", ")))
		}
	}

	// Repository info
	b.WriteString("\n## REPOSITORY\n")
	if issue.Repo != nil {
		b.WriteString(fmt.Sprintf("- Name: %s/%s\n", issue.Repo.Owner, issue.Repo.Name))
		b.WriteString(fmt.Sprintf("- Description: %s\n", issue.Repo.Description))
		b.WriteString(fmt.Sprintf("- Stars: %d\n", issue.Repo.Stars))
		b.WriteString(fmt.Sprintf("- Primary language: %s\n", issue.Repo.PrimaryLanguage))
		if len(issue.Repo.Topics) > 0 {
			b.WriteString(fmt.Sprintf("- Topics: %s\n", strings.Join(issue.Repo.Topics, ", ")))
		}
		b.WriteString(fmt.Sprintf("- Health score: %.2f\n", issue.Repo.HealthScore))
		b.WriteString(fmt.Sprintf("- Has CONTRIBUTING.md: %v\n", issue.Repo.HasContributing))
		if issue.Repo.LastCommitAt != nil {
			b.WriteString(fmt.Sprintf("- Last commit: %s\n", issue.Repo.LastCommitAt.Format("2006-01-02")))
		}
	}

	// Languages breakdown
	if len(rc.languages) > 0 {
		b.WriteString("\n## LANGUAGE BREAKDOWN\n")
		var total int
		for _, bytes := range rc.languages {
			total += bytes
		}
		for lang, bytes := range rc.languages {
			pct := float64(bytes) / float64(total) * 100
			if pct >= 1.0 {
				b.WriteString(fmt.Sprintf("- %s: %.1f%%\n", lang, pct))
			}
		}
	}

	// README
	if rc.readme != "" {
		b.WriteString("\n## README CONTENT\n")
		b.WriteString(rc.readme)
		b.WriteString("\n")
	}

	// CONTRIBUTING.md
	if rc.contributing != "" {
		b.WriteString("\n## CONTRIBUTING GUIDE\n")
		b.WriteString(rc.contributing)
		b.WriteString("\n")
	}

	// The issue itself
	b.WriteString("\n## ISSUE\n")
	b.WriteString(fmt.Sprintf("- Title: %s\n", issue.Title))
	b.WriteString(fmt.Sprintf("- Number: #%d\n", issue.Number))
	b.WriteString(fmt.Sprintf("- Labels: %s\n", strings.Join(issue.Labels, ", ")))
	b.WriteString(fmt.Sprintf("- Difficulty: %d/3\n", issue.Difficulty))
	b.WriteString(fmt.Sprintf("- Time estimate: %s\n", issue.TimeEstimate))
	b.WriteString(fmt.Sprintf("- Comments: %d\n", issue.CommentCount))
	b.WriteString(fmt.Sprintf("- Freshness: %.0f%%\n", issue.FreshnessScore*100))
	if issue.Body != "" {
		b.WriteString(fmt.Sprintf("\n### Issue body:\n%s\n", issue.Body))
	}

	return system, b.String()
}

// parseSections extracts the 5 named sections from the AI response.
func parseSections(text string) map[string]string {
	sections := map[string]string{
		"project_overview":   "",
		"issue_context":      "",
		"suggested_approach": "",
		"questions_to_ask":   "",
		"red_flags":          "",
		"first_comment":      "",
	}

	headerMap := map[string]string{
		"## PROJECT_OVERVIEW":   "project_overview",
		"## ISSUE_CONTEXT":      "issue_context",
		"## SUGGESTED_APPROACH": "suggested_approach",
		"## QUESTIONS_TO_ASK":   "questions_to_ask",
		"## RED_FLAGS":          "red_flags",
		"## FIRST_COMMENT":      "first_comment",
	}

	lines := strings.Split(text, "\n")
	var currentKey string

	for _, line := range lines {
		trimmed := strings.TrimSpace(line)
		if key, ok := headerMap[trimmed]; ok {
			currentKey = key
			continue
		}
		if currentKey != "" {
			sections[currentKey] += line + "\n"
		}
	}

	// Trim whitespace from each section
	for k, v := range sections {
		sections[k] = strings.TrimSpace(v)
	}

	return sections
}
