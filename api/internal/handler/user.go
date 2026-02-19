package handler

import (
	"encoding/json"
	"net/http"
	"strconv"

	"github.com/rohansx/illuminate/api/internal/middleware"
	"github.com/rohansx/illuminate/api/internal/model"
	"github.com/rohansx/illuminate/api/internal/service"
)

type UserHandler struct {
	userService  *service.UserService
	github       *service.GitHubService
	savedService *service.SavedIssueService
	notifService *service.NotificationService
}

func NewUserHandler(userService *service.UserService, github *service.GitHubService, savedService *service.SavedIssueService, notifService *service.NotificationService) *UserHandler {
	return &UserHandler{
		userService:  userService,
		github:       github,
		savedService: savedService,
		notifService: notifService,
	}
}

func (h *UserHandler) GetMe(w http.ResponseWriter, r *http.Request) {
	userID := middleware.GetUserID(r.Context())

	user, err := h.userService.GetProfile(r.Context(), userID)
	if err != nil {
		Error(w, http.StatusInternalServerError, "failed to get profile")
		return
	}

	JSON(w, http.StatusOK, user)
}

func (h *UserHandler) UpdateProfile(w http.ResponseWriter, r *http.Request) {
	userID := middleware.GetUserID(r.Context())

	var profile model.UserProfile
	if err := json.NewDecoder(r.Body).Decode(&profile); err != nil {
		Error(w, http.StatusBadRequest, "invalid request body")
		return
	}

	if err := h.userService.UpdateProfile(r.Context(), userID, profile); err != nil {
		Error(w, http.StatusInternalServerError, "failed to update profile")
		return
	}

	JSON(w, http.StatusOK, map[string]string{"status": "updated"})
}

func (h *UserHandler) GetProfile(w http.ResponseWriter, r *http.Request) {
	userID := middleware.GetUserID(r.Context())

	user, err := h.userService.GetProfile(r.Context(), userID)
	if err != nil {
		Error(w, http.StatusInternalServerError, "failed to get profile")
		return
	}

	// Fetch PR counts from GitHub in parallel
	type prResult struct {
		merged *service.GitHubSearchResult
		open   *service.GitHubSearchResult
	}
	ch := make(chan prResult, 1)
	go func() {
		var res prResult
		res.merged, _ = h.github.GetUserPRs(r.Context(), user.GitHubUsername, true, 1, 1)
		res.open, _ = h.github.GetUserPRs(r.Context(), user.GitHubUsername, false, 1, 1)
		ch <- res
	}()

	// Get saved issues count
	savedFeed, _ := h.savedService.GetSaved(r.Context(), userID, 1, 1)
	savedCount := 0
	if savedFeed != nil {
		savedCount = savedFeed.TotalCount
	}

	prs := <-ch

	mergedCount := 0
	openCount := 0
	if prs.merged != nil {
		mergedCount = prs.merged.TotalCount
	}
	if prs.open != nil {
		openCount = prs.open.TotalCount
	}

	profile := map[string]any{
		"user":            user,
		"merged_pr_count": mergedCount,
		"open_pr_count":   openCount,
		"saved_count":     savedCount,
	}

	JSON(w, http.StatusOK, profile)
}

func (h *UserHandler) SetManualSkills(w http.ResponseWriter, r *http.Request) {
	userID := middleware.GetUserID(r.Context())

	var body struct {
		Languages []string `json:"languages"`
	}
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
		Error(w, http.StatusBadRequest, "invalid request body")
		return
	}

	if len(body.Languages) == 0 {
		Error(w, http.StatusBadRequest, "at least one language is required")
		return
	}

	if len(body.Languages) > 20 {
		Error(w, http.StatusBadRequest, "maximum 20 languages allowed")
		return
	}

	skills, err := h.userService.SetManualSkills(r.Context(), userID, body.Languages)
	if err != nil {
		Error(w, http.StatusInternalServerError, "failed to set skills")
		return
	}

	JSON(w, http.StatusOK, map[string]any{"skills": skills})
}

func (h *UserHandler) AnalyzeSkills(w http.ResponseWriter, r *http.Request) {
	userID := middleware.GetUserID(r.Context())

	skills, err := h.userService.AnalyzeSkills(r.Context(), userID)
	if err != nil {
		Error(w, http.StatusInternalServerError, "failed to analyze skills")
		return
	}

	_ = h.notifService.Create(r.Context(), userID, "skills_analyzed",
		"Skills analyzed",
		"Your skill profile has been updated based on your GitHub activity",
		"/app/profile")

	JSON(w, http.StatusOK, map[string]any{"skills": skills})
}

func (h *UserHandler) GetPRs(w http.ResponseWriter, r *http.Request) {
	userID := middleware.GetUserID(r.Context())

	user, err := h.userService.GetProfile(r.Context(), userID)
	if err != nil {
		Error(w, http.StatusInternalServerError, "failed to get user")
		return
	}

	merged := r.URL.Query().Get("type") == "merged"
	page, _ := strconv.Atoi(r.URL.Query().Get("page"))
	perPage, _ := strconv.Atoi(r.URL.Query().Get("per_page"))
	if page < 1 {
		page = 1
	}
	if perPage < 1 || perPage > 30 {
		perPage = 20
	}

	result, err := h.github.GetUserPRs(r.Context(), user.GitHubUsername, merged, page, perPage)
	if err != nil {
		Error(w, http.StatusInternalServerError, "failed to fetch PRs")
		return
	}

	JSON(w, http.StatusOK, result)
}
