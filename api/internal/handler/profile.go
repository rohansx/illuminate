package handler

import (
	"net/http"

	"github.com/go-chi/chi/v5"
	"github.com/rohansx/illuminate/api/internal/service"
)

type ProfileHandler struct {
	contribService *service.ContributionService
}

func NewProfileHandler(contribService *service.ContributionService) *ProfileHandler {
	return &ProfileHandler{contribService: contribService}
}

func (h *ProfileHandler) GetPublicProfile(w http.ResponseWriter, r *http.Request) {
	username := chi.URLParam(r, "username")
	if username == "" {
		Error(w, http.StatusBadRequest, "username required")
		return
	}

	profile, err := h.contribService.GetPublicProfile(r.Context(), username)
	if err != nil {
		Error(w, http.StatusInternalServerError, "failed to get profile")
		return
	}
	if profile == nil {
		Error(w, http.StatusNotFound, "user not found")
		return
	}

	JSON(w, http.StatusOK, profile)
}
