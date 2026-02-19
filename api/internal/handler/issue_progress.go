package handler

import (
	"encoding/json"
	"net/http"

	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"
	"github.com/rohansx/illuminate/api/internal/middleware"
	"github.com/rohansx/illuminate/api/internal/model"
	"github.com/rohansx/illuminate/api/internal/service"
)

type IssueProgressHandler struct {
	progressService *service.IssueProgressService
}

func NewIssueProgressHandler(progressService *service.IssueProgressService) *IssueProgressHandler {
	return &IssueProgressHandler{progressService: progressService}
}

func (h *IssueProgressHandler) Get(w http.ResponseWriter, r *http.Request) {
	userID := middleware.GetUserID(r.Context())
	issueID, err := uuid.Parse(chi.URLParam(r, "id"))
	if err != nil {
		Error(w, http.StatusBadRequest, "invalid issue ID")
		return
	}

	progress, err := h.progressService.Get(r.Context(), userID, issueID)
	if err != nil {
		Error(w, http.StatusInternalServerError, "failed to get progress")
		return
	}
	if progress == nil {
		JSON(w, http.StatusOK, map[string]any{"progress": nil})
		return
	}
	JSON(w, http.StatusOK, map[string]any{"progress": progress})
}

func (h *IssueProgressHandler) Upsert(w http.ResponseWriter, r *http.Request) {
	userID := middleware.GetUserID(r.Context())
	issueID, err := uuid.Parse(chi.URLParam(r, "id"))
	if err != nil {
		Error(w, http.StatusBadRequest, "invalid issue ID")
		return
	}

	var body struct {
		Status string `json:"status"`
	}
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
		Error(w, http.StatusBadRequest, "invalid request body")
		return
	}

	progress, err := h.progressService.Upsert(r.Context(), userID, issueID, body.Status)
	if err != nil {
		Error(w, http.StatusBadRequest, err.Error())
		return
	}
	JSON(w, http.StatusOK, map[string]any{"progress": progress})
}

func (h *IssueProgressHandler) AddNote(w http.ResponseWriter, r *http.Request) {
	userID := middleware.GetUserID(r.Context())
	issueID, err := uuid.Parse(chi.URLParam(r, "id"))
	if err != nil {
		Error(w, http.StatusBadRequest, "invalid issue ID")
		return
	}

	var body struct {
		Note string `json:"note"`
	}
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
		Error(w, http.StatusBadRequest, "invalid request body")
		return
	}

	progress, err := h.progressService.AddNote(r.Context(), userID, issueID, body.Note)
	if err != nil {
		Error(w, http.StatusBadRequest, err.Error())
		return
	}
	JSON(w, http.StatusOK, map[string]any{"progress": progress})
}

func (h *IssueProgressHandler) Delete(w http.ResponseWriter, r *http.Request) {
	userID := middleware.GetUserID(r.Context())
	issueID, err := uuid.Parse(chi.URLParam(r, "id"))
	if err != nil {
		Error(w, http.StatusBadRequest, "invalid issue ID")
		return
	}

	if err := h.progressService.Delete(r.Context(), userID, issueID); err != nil {
		Error(w, http.StatusInternalServerError, "failed to delete progress")
		return
	}
	JSON(w, http.StatusOK, map[string]string{"status": "deleted"})
}

func (h *IssueProgressHandler) ListByUser(w http.ResponseWriter, r *http.Request) {
	userID := middleware.GetUserID(r.Context())

	list, err := h.progressService.ListByUser(r.Context(), userID)
	if err != nil {
		Error(w, http.StatusInternalServerError, "failed to list progress")
		return
	}
	if list == nil {
		list = []model.IssueProgress{}
	}
	JSON(w, http.StatusOK, map[string]any{"progress": list})
}
