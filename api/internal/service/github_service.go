package service

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
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
	var issues []GitHubIssue
	url := fmt.Sprintf("https://api.github.com/repos/%s/%s/issues?state=open&per_page=100&labels=good+first+issue,help+wanted,beginner,easy,starter", owner, name)
	if err := s.getPublic(ctx, url, &issues); err != nil {
		return nil, fmt.Errorf("getting public repo issues: %w", err)
	}
	return issues, nil
}

func (s *GitHubService) GetPublicRepoLanguages(ctx context.Context, owner, name string) (map[string]int, error) {
	var languages map[string]int
	url := fmt.Sprintf("https://api.github.com/repos/%s/%s/languages", owner, name)
	if err := s.getPublic(ctx, url, &languages); err != nil {
		return nil, fmt.Errorf("getting public repo languages: %w", err)
	}
	return languages, nil
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
	return s.doRequest(req, target)
}

func (s *GitHubService) doRequest(req *http.Request, target any) error {
	client := &http.Client{Timeout: 15 * time.Second}
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
