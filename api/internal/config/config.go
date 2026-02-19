package config

import (
	"fmt"
	"time"

	"github.com/kelseyhightower/envconfig"
)

type Config struct {
	Port               int    `envconfig:"PORT" default:"8080"`
	DatabaseURL        string `envconfig:"DATABASE_URL" required:"true"`
	RedisURL           string `envconfig:"REDIS_URL" default:""`
	GitHubClientID     string `envconfig:"GITHUB_CLIENT_ID" required:"true"`
	GitHubClientSecret string `envconfig:"GITHUB_CLIENT_SECRET" required:"true"`
	EncryptKey         string `envconfig:"ENCRYPT_KEY" required:"true"`
	JWTSecret          string `envconfig:"JWT_SECRET" required:"true"`
	FrontendURL        string `envconfig:"FRONTEND_URL" default:"http://localhost:5173"`
	BackendURL         string `envconfig:"BACKEND_URL" default:"http://localhost:8080"`
	CookieDomain       string `envconfig:"COOKIE_DOMAIN" default:"localhost"`
	Env                string `envconfig:"ENV" default:"development"`
	AdminGitHubUsername string `envconfig:"ADMIN_GITHUB_USERNAME" default:""`
	GLMAPIKey          string        `envconfig:"GLM_API_KEY" default:""`
	DiscoveryInterval  time.Duration `envconfig:"DISCOVERY_INTERVAL" default:"1h"`
}

func (c *Config) IsProd() bool {
	return c.Env == "production"
}

func Load() (*Config, error) {
	var cfg Config
	if err := envconfig.Process("illuminate", &cfg); err != nil {
		return nil, fmt.Errorf("loading config: %w", err)
	}
	return &cfg, nil
}
