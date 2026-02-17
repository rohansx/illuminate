package service

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"log/slog"
	"net/http"
	"net/url"
	"time"

	"golang.org/x/oauth2"
	"golang.org/x/oauth2/github"
)

type GitHubUser struct {
	ID        int64  `json:"id"`
	Login     string `json:"login"`
	AvatarURL string `json:"avatar_url"`
	Bio       string `json:"bio"`
}

type GitHubRepo struct {
	ID          int64    `json:"id"`
	Owner       struct {
		Login string `json:"login"`
	} `json:"owner"`
	Name            string   `json:"name"`
	Description     string   `json:"description"`
	StargazersCount int      `json:"stargazers_count"`
	Language        string   `json:"language"`
	Topics          []string `json:"topics"`
	PushedAt        string   `json:"pushed_at"`
	HasIssues       bool     `json:"has_issues"`
}

type GitHubIssue struct {
	ID     int64  `json:"id"`
	Number int    `json:"number"`
	Title  string `json:"title"`
	Body   string `json:"body"`
	Labels []struct {
		Name string `json:"name"`
	} `json:"labels"`
	Comments  int    `json:"comments"`
	State     string `json:"state"`
	CreatedAt string `json:"created_at"`
	UpdatedAt string `json:"updated_at"`
}

type GitHubService struct {
	oauth *oauth2.Config
}

func NewGitHubService(clientID, clientSecret, callbackURL string) *GitHubService {
	return &GitHubService{
		oauth: &oauth2.Config{
			ClientID:     clientID,
			ClientSecret: clientSecret,
			Endpoint:     github.Endpoint,
			RedirectURL:  callbackURL,
			Scopes:       []string{"read:user", "user:email"},
		},
	}
}

func (s *GitHubService) AuthURL(state string) string {
	return s.oauth.AuthCodeURL(state)
}

func (s *GitHubService) ExchangeCode(ctx context.Context, code string) (*oauth2.Token, error) {
	token, err := s.oauth.Exchange(ctx, code)
	if err != nil {
		return nil, fmt.Errorf("exchanging code: %w", err)
	}
	return token, nil
}

func (s *GitHubService) GetUser(ctx context.Context, accessToken string) (*GitHubUser, error) {
	var user GitHubUser
	if err := s.get(ctx, accessToken, "https://api.github.com/user", &user); err != nil {
		return nil, fmt.Errorf("getting user: %w", err)
	}
	return &user, nil
}

func (s *GitHubService) GetUserRepos(ctx context.Context, accessToken string) ([]GitHubRepo, error) {
	var repos []GitHubRepo
	if err := s.get(ctx, accessToken, "https://api.github.com/user/repos?per_page=100&sort=pushed&affiliation=owner", &repos); err != nil {
		return nil, fmt.Errorf("getting user repos: %w", err)
	}
	return repos, nil
}

func (s *GitHubService) GetRepository(ctx context.Context, accessToken, owner, name string) (*GitHubRepo, error) {
	var repo GitHubRepo
	url := fmt.Sprintf("https://api.github.com/repos/%s/%s", owner, name)
	if err := s.get(ctx, accessToken, url, &repo); err != nil {
		return nil, fmt.Errorf("getting repository: %w", err)
	}
	return &repo, nil
}

func (s *GitHubService) GetRepoIssues(ctx context.Context, accessToken, owner, name string) ([]GitHubIssue, error) {
	var issues []GitHubIssue
	url := fmt.Sprintf("https://api.github.com/repos/%s/%s/issues?state=open&per_page=100&labels=good+first+issue,help+wanted,beginner,easy,starter", owner, name)
	if err := s.get(ctx, accessToken, url, &issues); err != nil {
		return nil, fmt.Errorf("getting repo issues: %w", err)
	}
	return issues, nil
}

func (s *GitHubService) GetRepoLanguages(ctx context.Context, accessToken, owner, name string) (map[string]int, error) {
	var languages map[string]int
	url := fmt.Sprintf("https://api.github.com/repos/%s/%s/languages", owner, name)
	if err := s.get(ctx, accessToken, url, &languages); err != nil {
		return nil, fmt.Errorf("getting repo languages: %w", err)
	}
	return languages, nil
}

func (s *GitHubService) GetPublicRepo(ctx context.Context, owner, name string) (*GitHubRepo, error) {
	var repo GitHubRepo
	url := fmt.Sprintf("https://api.github.com/repos/%s/%s", owner, name)
	if err := s.getPublic(ctx, url, &repo); err != nil {
		return nil, fmt.Errorf("getting public repository: %w", err)
	}
	return &repo, nil
}

func (s *GitHubService) GetPublicRepoIssues(ctx context.Context, owner, name string) ([]GitHubIssue, error) {
	labels := []string{"good first issue", "help wanted", "beginner", "easy", "starter"}
	seen := make(map[int64]bool)
	var all []GitHubIssue

	for _, label := range labels {
		var issues []GitHubIssue
		apiURL := fmt.Sprintf("https://api.github.com/repos/%s/%s/issues?state=open&per_page=30&labels=%s", owner, name, url.QueryEscape(label))
		if err := s.getPublic(ctx, apiURL, &issues); err != nil {
			slog.Debug("label query failed", "repo", owner+"/"+name, "label", label, "error", err)
			continue
		}
		slog.Debug("label query result", "repo", owner+"/"+name, "label", label, "count", len(issues))
		for _, issue := range issues {
			if !seen[issue.ID] {
				seen[issue.ID] = true
				all = append(all, issue)
			}
		}
	}
	return all, nil
}

func (s *GitHubService) GetPublicRepoLanguages(ctx context.Context, owner, name string) (map[string]int, error) {
	var languages map[string]int
	url := fmt.Sprintf("https://api.github.com/repos/%s/%s/languages", owner, name)
	if err := s.getPublic(ctx, url, &languages); err != nil {
		return nil, fmt.Errorf("getting public repo languages: %w", err)
	}
	return languages, nil
}

type GitHubSearchResult struct {
	TotalCount int        `json:"total_count"`
	Items      []GitHubPR `json:"items"`
}

type GitHubPR struct {
	ID          int64  `json:"id"`
	Number      int    `json:"number"`
	Title       string `json:"title"`
	State       string `json:"state"`
	HTMLURL     string `json:"html_url"`
	CreatedAt   string `json:"created_at"`
	UpdatedAt   string `json:"updated_at"`
	ClosedAt    string `json:"closed_at"`
	PullRequest *struct {
		MergedAt string `json:"merged_at"`
	} `json:"pull_request"`
	RepositoryURL string `json:"repository_url"`
	Labels        []struct {
		Name string `json:"name"`
	} `json:"labels"`
}

func (s *GitHubService) GetUserPRs(ctx context.Context, username string, merged bool, page, perPage int) (*GitHubSearchResult, error) {
	state := "is:open"
	if merged {
		state = "is:merged"
	}
	if page < 1 {
		page = 1
	}
	if perPage < 1 || perPage > 100 {
		perPage = 30
	}
	apiURL := fmt.Sprintf("https://api.github.com/search/issues?q=type:pr+author:%s+%s&sort=updated&order=desc&per_page=%d&page=%d",
		url.QueryEscape(username), state, perPage, page)
	var result GitHubSearchResult
	if err := s.getPublic(ctx, apiURL, &result); err != nil {
		return nil, fmt.Errorf("searching user PRs: %w", err)
	}
	return &result, nil
}

type GitHubComment struct {
	ID        int64  `json:"id"`
	Body      string `json:"body"`
	CreatedAt string `json:"created_at"`
	UpdatedAt string `json:"updated_at"`
	User      struct {
		Login     string `json:"login"`
		AvatarURL string `json:"avatar_url"`
	} `json:"user"`
}

func (s *GitHubService) GetPublicIssueComments(ctx context.Context, owner, name string, number int) ([]GitHubComment, error) {
	var comments []GitHubComment
	apiURL := fmt.Sprintf("https://api.github.com/repos/%s/%s/issues/%d/comments?per_page=100", owner, name, number)
	if err := s.getPublic(ctx, apiURL, &comments); err != nil {
		return nil, fmt.Errorf("getting issue comments: %w", err)
	}
	return comments, nil
}

func (s *GitHubService) GetPublicFileContent(ctx context.Context, owner, name, path string) (string, error) {
	fileURL := fmt.Sprintf("https://api.github.com/repos/%s/%s/contents/%s", owner, name, url.QueryEscape(path))
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, fileURL, nil)
	if err != nil {
		return "", err
	}
	req.Header.Set("Accept", "application/vnd.github.v3.raw")
	if s.oauth.ClientID != "" && s.oauth.ClientSecret != "" {
		req.SetBasicAuth(s.oauth.ClientID, s.oauth.ClientSecret)
	}

	client := &http.Client{Timeout: 30 * time.Second}
	resp, err := client.Do(req)
	if err != nil {
		return "", err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return "", fmt.Errorf("file not found (%d)", resp.StatusCode)
	}

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", fmt.Errorf("reading file content: %w", err)
	}

	return string(body), nil
}

func (s *GitHubService) get(ctx context.Context, accessToken, url string, target any) error {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return err
	}
	req.Header.Set("Authorization", "Bearer "+accessToken)
	req.Header.Set("Accept", "application/vnd.github.v3+json")
	return s.doRequest(req, target)
}

func (s *GitHubService) getPublic(ctx context.Context, url string, target any) error {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return err
	}
	req.Header.Set("Accept", "application/vnd.github.v3+json")
	// Use Basic Auth with OAuth app credentials for higher rate limit (5000/hr vs 60/hr)
	if s.oauth.ClientID != "" && s.oauth.ClientSecret != "" {
		req.SetBasicAuth(s.oauth.ClientID, s.oauth.ClientSecret)
	}
	return s.doRequest(req, target)
}

func (s *GitHubService) doRequest(req *http.Request, target any) error {
	client := &http.Client{Timeout: 30 * time.Second}
	resp, err := client.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(resp.Body)
		return fmt.Errorf("github api error (%d): %s", resp.StatusCode, string(body))
	}

	return json.NewDecoder(resp.Body).Decode(target)
}
