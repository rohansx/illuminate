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
	deepDiveRepo := repository.NewDeepDiveRepo(pool)
	savedIssueRepo := repository.NewSavedIssueRepo(pool)

	// Services
	callbackURL := cfg.BackendURL + "/auth/github/callback"
	githubService := service.NewGitHubService(cfg.GitHubClientID, cfg.GitHubClientSecret, callbackURL)
	authService := service.NewAuthService(githubService, userRepo, tokenRepo, encryptor, jwtManager, cfg.AdminGitHubUsername)
	userService := service.NewUserService(userRepo, githubService, encryptor)
	issueService := service.NewIssueService(issueRepo, repoRepo, githubService)
	matchingService := service.NewMatchingService()
	glmService := service.NewGLMService(cfg.GLMAPIKey)
	deepDiveService := service.NewDeepDiveService(deepDiveRepo, issueRepo, repoRepo, userRepo, githubService, glmService)
	savedIssueService := service.NewSavedIssueService(savedIssueRepo, issueRepo)

	// Admin + Discovery
	jobManager := service.NewJobManager()
	discoveryService := service.NewDiscoveryService(repoRepo, issueService, githubService, jobManager)
	adminService := service.NewAdminService(userRepo, repoRepo, issueRepo, issueService, githubService, jobManager, discoveryService)

	// Handlers
	authHandler := handler.NewAuthHandler(authService, githubService, cfg.FrontendURL, cfg.CookieDomain, cfg.IsProd())
	userHandler := handler.NewUserHandler(userService, githubService, savedIssueService)
	issueHandler := handler.NewIssueHandler(issueService, userService, matchingService, savedIssueService)
	adminHandler := handler.NewAdminHandler(adminService)
	deepDiveHandler := handler.NewDeepDiveHandler(deepDiveService)
	savedIssueHandler := handler.NewSavedIssueHandler(savedIssueService)

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
		r.Get("/api/users/me/stats", userHandler.GetProfile)
		r.Get("/api/users/me/prs", userHandler.GetPRs)
		r.Get("/api/categories", issueHandler.GetCategories)
		r.Get("/api/issues/feed", issueHandler.GetFeed)
		r.Get("/api/issues/saved", savedIssueHandler.ListSaved)
		r.Get("/api/issues/search", issueHandler.Search)
		r.Get("/api/issues/{id}", issueHandler.GetByID)
		r.Get("/api/issues/{id}/deep-dive", deepDiveHandler.Generate)
		r.Get("/api/issues/{id}/comments", issueHandler.GetComments)
		r.Post("/api/issues/{id}/save", savedIssueHandler.Save)
		r.Delete("/api/issues/{id}/save", savedIssueHandler.Unsave)
		r.Get("/api/issues/{id}/saved", savedIssueHandler.IsSaved)
	})

	// Admin routes
	r.Group(func(r chi.Router) {
		r.Use(middleware.Auth(jwtManager))
		r.Use(middleware.Admin(userRepo))
		r.Get("/admin/stats", adminHandler.GetStats)
		r.Get("/admin/users", adminHandler.ListUsers)
		r.Patch("/admin/users/{id}/role", adminHandler.UpdateUserRole)
		r.Post("/admin/seed", adminHandler.TriggerSeed)
		r.Post("/admin/index", adminHandler.TriggerIndex)
		r.Post("/admin/discover", adminHandler.TriggerDiscover)
		r.Get("/admin/jobs", adminHandler.GetJobs)
		r.Get("/admin/repos", adminHandler.ListRepos)
		r.Delete("/admin/repos/{id}", adminHandler.DeleteRepo)
		r.Patch("/admin/repos/{id}/metadata", adminHandler.UpdateRepoMetadata)
		r.Get("/admin/categories", adminHandler.GetCategories)
		r.Post("/admin/repos/{id}/categories", adminHandler.AssignRepoCategory)
		r.Delete("/admin/repos/{id}/categories/{category_id}", adminHandler.RemoveRepoCategory)
	})

	// Static file serving (production: serve SvelteKit build)
	staticDir := "web/build"
	if info, err := os.Stat(staticDir); err == nil && info.IsDir() {
		slog.Info("serving static files", "dir", staticDir)
		fileServer := http.FileServer(http.Dir(staticDir))
		r.NotFound(func(w http.ResponseWriter, req *http.Request) {
			path := staticDir + req.URL.Path
			if _, err := os.Stat(path); err == nil {
				fileServer.ServeHTTP(w, req)
				return
			}
			// SPA fallback
			req.URL.Path = "/200.html"
			fileServer.ServeHTTP(w, req)
		})
	}

	// Discovery scheduler
	var scheduler *service.Scheduler
	if cfg.DiscoveryEnabled {
		scheduler = service.NewScheduler(discoveryService, cfg.DiscoveryInterval)
		scheduler.Start()
	} else {
		slog.Info("auto-discovery disabled")
	}

	srv := &http.Server{
		Addr:         fmt.Sprintf(":%d", cfg.Port),
		Handler:      r,
		ReadTimeout:  10 * time.Second,
		WriteTimeout: 120 * time.Second,
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
	if scheduler != nil {
		scheduler.Stop()
	}
	shutdownCtx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	if err := srv.Shutdown(shutdownCtx); err != nil {
		slog.Error("server shutdown failed", "error", err)
	}
}
