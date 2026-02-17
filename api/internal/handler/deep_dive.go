package handler

import (
	"errors"
	"log/slog"
	"net/http"

	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"
	"github.com/rohansx/illuminate/api/internal/middleware"
	"github.com/rohansx/illuminate/api/internal/service"
)

type DeepDiveHandler struct {
	deepDiveService *service.DeepDiveService
}

func NewDeepDiveHandler(deepDiveService *service.DeepDiveService) *DeepDiveHandler {
	return &DeepDiveHandler{deepDiveService: deepDiveService}
}

func (h *DeepDiveHandler) Generate(w http.ResponseWriter, r *http.Request) {
	idStr := chi.URLParam(r, "id")
	issueID, err := uuid.Parse(idStr)
	if err != nil {
		Error(w, http.StatusBadRequest, "invalid issue id")
		return
	}

	userID := middleware.GetUserID(r.Context())

	dd, err := h.deepDiveService.Generate(r.Context(), issueID, userID)
	if err != nil {
		switch {
		case errors.Is(err, service.ErrDeepDiveNotConfigured):
			Error(w, http.StatusServiceUnavailable, "deep dive feature is not available")
		case errors.Is(err, service.ErrIssueNotFound):
			Error(w, http.StatusNotFound, "issue not found")
		default:
			slog.Error("deep dive generation failed", "error", err, "issue_id", idStr)
			Error(w, http.StatusInternalServerError, "failed to generate deep dive")
		}
		return
	}

	JSON(w, http.StatusOK, dd)
}
