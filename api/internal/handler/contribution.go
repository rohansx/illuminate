package handler

import (
	"net/http"
	"strconv"

	"github.com/rohansx/illuminate/api/internal/middleware"
	"github.com/rohansx/illuminate/api/internal/service"
)

type ContributionHandler struct {
	contribService *service.ContributionService
}

func NewContributionHandler(contribService *service.ContributionService) *ContributionHandler {
	return &ContributionHandler{contribService: contribService}
}

func (h *ContributionHandler) GetTimeline(w http.ResponseWriter, r *http.Request) {
	userID := middleware.GetUserID(r.Context())
	page, _ := strconv.Atoi(r.URL.Query().Get("page"))
	perPage, _ := strconv.Atoi(r.URL.Query().Get("per_page"))

	feed, err := h.contribService.GetTimeline(r.Context(), userID, page, perPage)
	if err != nil {
		Error(w, http.StatusInternalServerError, "failed to get contributions")
		return
	}
	JSON(w, http.StatusOK, feed)
}

func (h *ContributionHandler) GetProjects(w http.ResponseWriter, r *http.Request) {
	userID := middleware.GetUserID(r.Context())

	projects, err := h.contribService.GetProjects(r.Context(), userID)
	if err != nil {
		Error(w, http.StatusInternalServerError, "failed to get projects")
		return
	}
	JSON(w, http.StatusOK, projects)
}

func (h *ContributionHandler) GetStats(w http.ResponseWriter, r *http.Request) {
	userID := middleware.GetUserID(r.Context())

	stats, err := h.contribService.GetPortfolioStats(r.Context(), userID)
	if err != nil {
		Error(w, http.StatusInternalServerError, "failed to get stats")
		return
	}
	JSON(w, http.StatusOK, stats)
}

func (h *ContributionHandler) SyncContributions(w http.ResponseWriter, r *http.Request) {
	userID := middleware.GetUserID(r.Context())

	if err := h.contribService.SyncCurrentUser(r.Context(), userID); err != nil {
		Error(w, http.StatusInternalServerError, "failed to sync contributions")
		return
	}
	JSON(w, http.StatusOK, map[string]string{"status": "synced"})
}
