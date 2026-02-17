package handler

import (
	"net/http"
	"strconv"

	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"
	"github.com/rohansx/illuminate/api/internal/middleware"
	"github.com/rohansx/illuminate/api/internal/service"
)

type SavedIssueHandler struct {
	savedService *service.SavedIssueService
}

func NewSavedIssueHandler(savedService *service.SavedIssueService) *SavedIssueHandler {
	return &SavedIssueHandler{savedService: savedService}
}

func (h *SavedIssueHandler) Save(w http.ResponseWriter, r *http.Request) {
	userID := middleware.GetUserID(r.Context())
	issueID, err := uuid.Parse(chi.URLParam(r, "id"))
	if err != nil {
		Error(w, http.StatusBadRequest, "invalid issue id")
		return
	}

	if err := h.savedService.Save(r.Context(), userID, issueID); err != nil {
		Error(w, http.StatusInternalServerError, "failed to save issue")
		return
	}

	JSON(w, http.StatusCreated, map[string]string{"status": "saved"})
}

func (h *SavedIssueHandler) Unsave(w http.ResponseWriter, r *http.Request) {
	userID := middleware.GetUserID(r.Context())
	issueID, err := uuid.Parse(chi.URLParam(r, "id"))
	if err != nil {
		Error(w, http.StatusBadRequest, "invalid issue id")
		return
	}

	if err := h.savedService.Unsave(r.Context(), userID, issueID); err != nil {
		Error(w, http.StatusInternalServerError, "failed to unsave issue")
		return
	}

	JSON(w, http.StatusOK, map[string]string{"status": "unsaved"})
}

func (h *SavedIssueHandler) IsSaved(w http.ResponseWriter, r *http.Request) {
	userID := middleware.GetUserID(r.Context())
	issueID, err := uuid.Parse(chi.URLParam(r, "id"))
	if err != nil {
		Error(w, http.StatusBadRequest, "invalid issue id")
		return
	}

	saved, err := h.savedService.IsSaved(r.Context(), userID, issueID)
	if err != nil {
		Error(w, http.StatusInternalServerError, "failed to check saved status")
		return
	}

	JSON(w, http.StatusOK, map[string]bool{"saved": saved})
}

func (h *SavedIssueHandler) ListSaved(w http.ResponseWriter, r *http.Request) {
	userID := middleware.GetUserID(r.Context())
	page, _ := strconv.Atoi(r.URL.Query().Get("page"))
	perPage, _ := strconv.Atoi(r.URL.Query().Get("per_page"))

	feed, err := h.savedService.GetSaved(r.Context(), userID, page, perPage)
	if err != nil {
		Error(w, http.StatusInternalServerError, "failed to get saved issues")
		return
	}

	JSON(w, http.StatusOK, feed)
}
