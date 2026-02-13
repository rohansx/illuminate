package main

import (
	"context"
	"fmt"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/go-chi/chi/v5"
	"github.com/redis/go-redis/v9"
	"github.com/rohansx/illuminate/api/internal/config"
	"github.com/rohansx/illuminate/api/internal/crypto"
	"github.com/rohansx/illuminate/api/internal/database"
	"github.com/rohansx/illuminate/api/internal/handler"
	"github.com/rohansx/illuminate/api/internal/middleware"
	"github.com/rohansx/illuminate/api/internal/repository"
	"github.com/rohansx/illuminate/api/internal/service"
)

func main() {
	logger := slog.New(slog.NewJSONHandler(os.Stdout, nil))
	slog.SetDefault(logger)

	cfg, err := config.Load()
	if err != nil {
		slog.Error("failed to load config", "error", err)
		os.Exit(1)
	}

	// Database
	ctx := context.Background()
	pool, err := database.NewPool(ctx, cfg.DatabaseURL)
	if err != nil {
		slog.Error("failed to connect to database", "error", err)
		os.Exit(1)
	}
	defer pool.Close()

	// Redis (optional — rate limiting is skipped without it)
	var rdb *redis.Client
	if cfg.RedisURL != "" {
		rdb, err = database.NewRedisClient(cfg.RedisURL)
		if err != nil {
			slog.Warn("redis unavailable, rate limiting disabled", "error", err)
			rdb = nil
		} else {
			defer rdb.Close()
		}
	} else {
		slog.Info("redis not configured, rate limiting disabled")
	}

	// Crypto
	encryptor, err := crypto.NewEncryptor(cfg.EncryptKey)
	if err != nil {
		slog.Error("failed to create encryptor", "error", err)
		os.Exit(1)
	}
	jwtManager := crypto.NewJWTManager(cfg.JWTSecret)

	// Repositories
	userRepo := repository.NewUserRepo(pool)
	repoRepo := repository.NewRepoRepo(pool)
	issueRepo := repository.NewIssueRepo(pool)
	tokenRepo := repository.NewTokenRepo(pool)

	// Services
	callbackURL := cfg.BackendURL + "/auth/github/callback"
	githubService := service.NewGitHubService(cfg.GitHubClientID, cfg.GitHubClientSecret, callbackURL)
	authService := service.NewAuthService(githubService, userRepo, tokenRepo, encryptor, jwtManager)
	userService := service.NewUserService(userRepo, githubService, encryptor)
	issueService := service.NewIssueService(issueRepo, repoRepo, githubService)
	matchingService := service.NewMatchingService()

	// Handlers
	authHandler := handler.NewAuthHandler(authService, githubService, cfg.FrontendURL, cfg.CookieDomain, cfg.IsProd())
	userHandler := handler.NewUserHandler(userService)
	issueHandler := handler.NewIssueHandler(issueService, userService, matchingService)

	// Router
	r := chi.NewRouter()
	r.Use(middleware.Logger)
	r.Use(middleware.CORS(cfg.FrontendURL))
	r.Use(middleware.RateLimit(rdb, 60, time.Minute))

	// Public routes
	r.Get("/health", handler.Health)
	r.Get("/auth/github/login", authHandler.Login)
	r.Get("/auth/github/callback", authHandler.Callback)
	r.Post("/auth/refresh", authHandler.Refresh)

	// Protected routes
	r.Group(func(r chi.Router) {
		r.Use(middleware.Auth(jwtManager))
		r.Post("/auth/logout", authHandler.Logout)
		r.Get("/api/users/me", userHandler.GetMe)
		r.Patch("/api/users/me/profile", userHandler.UpdateProfile)
		r.Get("/api/issues/feed", issueHandler.GetFeed)
		r.Get("/api/issues/search", issueHandler.Search)
		r.Get("/api/issues/{id}", issueHandler.GetByID)
	})

	srv := &http.Server{
		Addr:         fmt.Sprintf(":%d", cfg.Port),
		Handler:      r,
		ReadTimeout:  10 * time.Second,
		WriteTimeout: 30 * time.Second,
		IdleTimeout:  60 * time.Second,
	}

	go func() {
		slog.Info("starting server", "port", cfg.Port, "env", cfg.Env)
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			slog.Error("server failed", "error", err)
			os.Exit(1)
		}
	}()

	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)
	<-quit

	slog.Info("shutting down server")
	shutdownCtx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	if err := srv.Shutdown(shutdownCtx); err != nil {
		slog.Error("server shutdown failed", "error", err)
	}
}
