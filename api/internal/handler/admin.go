package handler

import (
	"encoding/json"
	"net/http"
	"strconv"

	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"
	"github.com/rohansx/illuminate/api/internal/model"
	"github.com/rohansx/illuminate/api/internal/service"
)

type AdminHandler struct {
	adminService *service.AdminService
}

func NewAdminHandler(adminService *service.AdminService) *AdminHandler {
	return &AdminHandler{adminService: adminService}
}

func (h *AdminHandler) GetStats(w http.ResponseWriter, r *http.Request) {
	stats, err := h.adminService.GetStats(r.Context())
	if err != nil {
		Error(w, http.StatusInternalServerError, "failed to get stats")
		return
	}
	JSON(w, http.StatusOK, stats)
}

func (h *AdminHandler) ListUsers(w http.ResponseWriter, r *http.Request) {
	page, _ := strconv.Atoi(r.URL.Query().Get("page"))
	perPage, _ := strconv.Atoi(r.URL.Query().Get("per_page"))

	users, err := h.adminService.ListUsers(r.Context(), page, perPage)
	if err != nil {
		Error(w, http.StatusInternalServerError, "failed to list users")
		return
	}
	JSON(w, http.StatusOK, users)
}

func (h *AdminHandler) UpdateUserRole(w http.ResponseWriter, r *http.Request) {
	idStr := chi.URLParam(r, "id")
	id, err := uuid.Parse(idStr)
	if err != nil {
		Error(w, http.StatusBadRequest, "invalid user id")
		return
	}

	var body struct {
		Role string `json:"role"`
	}
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
		Error(w, http.StatusBadRequest, "invalid request body")
		return
	}

	if err := h.adminService.UpdateUserRole(r.Context(), id, body.Role); err != nil {
		Error(w, http.StatusBadRequest, err.Error())
		return
	}

	JSON(w, http.StatusOK, map[string]string{"status": "updated"})
}

func (h *AdminHandler) TriggerSeed(w http.ResponseWriter, r *http.Request) {
	job, err := h.adminService.TriggerSeed(r.Context())
	if err != nil {
		Error(w, http.StatusConflict, err.Error())
		return
	}
	JSON(w, http.StatusAccepted, job)
}

func (h *AdminHandler) TriggerIndex(w http.ResponseWriter, r *http.Request) {
	job, err := h.adminService.TriggerIndex(r.Context())
	if err != nil {
		Error(w, http.StatusConflict, err.Error())
		return
	}
	JSON(w, http.StatusAccepted, job)
}

func (h *AdminHandler) GetJobs(w http.ResponseWriter, r *http.Request) {
	jobs := h.adminService.GetJobs()
	if jobs == nil {
		jobs = []model.JobStatus{}
	}
	JSON(w, http.StatusOK, jobs)
}

func (h *AdminHandler) ListRepos(w http.ResponseWriter, r *http.Request) {
	page, _ := strconv.Atoi(r.URL.Query().Get("page"))
	perPage, _ := strconv.Atoi(r.URL.Query().Get("per_page"))

	repos, err := h.adminService.ListRepos(r.Context(), page, perPage)
	if err != nil {
		Error(w, http.StatusInternalServerError, "failed to list repos")
		return
	}
	JSON(w, http.StatusOK, repos)
}

func (h *AdminHandler) DeleteRepo(w http.ResponseWriter, r *http.Request) {
	idStr := chi.URLParam(r, "id")
	id, err := uuid.Parse(idStr)
	if err != nil {
		Error(w, http.StatusBadRequest, "invalid repo id")
		return
	}

	if err := h.adminService.DeleteRepo(r.Context(), id); err != nil {
		Error(w, http.StatusInternalServerError, "failed to delete repo")
		return
	}

	JSON(w, http.StatusOK, map[string]string{"status": "deleted"})
}

func (h *AdminHandler) UpdateRepoMetadata(w http.ResponseWriter, r *http.Request) {
	idStr := chi.URLParam(r, "id")
	id, err := uuid.Parse(idStr)
	if err != nil {
		Error(w, http.StatusBadRequest, "invalid repo id")
		return
	}

	var body struct {
		Tags       []string `json:"tags"`
		Difficulty string   `json:"difficulty_level"`
		Activity   string   `json:"activity_status"`
	}
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
		Error(w, http.StatusBadRequest, "invalid request body")
		return
	}

	if err := h.adminService.UpdateRepoMetadata(r.Context(), id, body.Tags, body.Difficulty, body.Activity); err != nil {
		Error(w, http.StatusInternalServerError, "failed to update repo")
		return
	}

	JSON(w, http.StatusOK, map[string]string{"status": "updated"})
}

func (h *AdminHandler) GetCategories(w http.ResponseWriter, r *http.Request) {
	categories, err := h.adminService.GetCategories(r.Context())
	if err != nil {
		Error(w, http.StatusInternalServerError, "failed to get categories")
		return
	}
	JSON(w, http.StatusOK, categories)
}

func (h *AdminHandler) AssignRepoCategory(w http.ResponseWriter, r *http.Request) {
	repoIDStr := chi.URLParam(r, "id")
	repoID, err := uuid.Parse(repoIDStr)
	if err != nil {
		Error(w, http.StatusBadRequest, "invalid repo id")
		return
	}

	var body struct {
		CategoryID string `json:"category_id"`
	}
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
		Error(w, http.StatusBadRequest, "invalid request body")
		return
	}

	categoryID, err := uuid.Parse(body.CategoryID)
	if err != nil {
		Error(w, http.StatusBadRequest, "invalid category id")
		return
	}

	if err := h.adminService.AssignRepoCategory(r.Context(), repoID, categoryID); err != nil {
		Error(w, http.StatusInternalServerError, "failed to assign category")
		return
	}

	JSON(w, http.StatusOK, map[string]string{"status": "assigned"})
}

func (h *AdminHandler) RemoveRepoCategory(w http.ResponseWriter, r *http.Request) {
	repoIDStr := chi.URLParam(r, "id")
	categoryIDStr := chi.URLParam(r, "category_id")

	repoID, err := uuid.Parse(repoIDStr)
	if err != nil {
		Error(w, http.StatusBadRequest, "invalid repo id")
		return
	}

	categoryID, err := uuid.Parse(categoryIDStr)
	if err != nil {
		Error(w, http.StatusBadRequest, "invalid category id")
		return
	}

	if err := h.adminService.RemoveRepoCategory(r.Context(), repoID, categoryID); err != nil {
		Error(w, http.StatusInternalServerError, "failed to remove category")
		return
	}

	JSON(w, http.StatusOK, map[string]string{"status": "removed"})
}
