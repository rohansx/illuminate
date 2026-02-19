package handler

import (
	"net/http"

	"github.com/rohansx/illuminate/api/internal/middleware"
	"github.com/rohansx/illuminate/api/internal/service"
)

type GrowthHandler struct {
	growthService *service.GrowthService
}

func NewGrowthHandler(growthService *service.GrowthService) *GrowthHandler {
	return &GrowthHandler{growthService: growthService}
}

func (h *GrowthHandler) GetGrowthProfile(w http.ResponseWriter, r *http.Request) {
	userID := middleware.GetUserID(r.Context())

	profile, err := h.growthService.GetGrowthProfile(r.Context(), userID)
	if err != nil {
		Error(w, http.StatusInternalServerError, "failed to get growth profile")
		return
	}
	JSON(w, http.StatusOK, profile)
}
