package handler

import (
	"encoding/json"
	"net/http"

	"github.com/rohansx/illuminate/api/internal/middleware"
	"github.com/rohansx/illuminate/api/internal/model"
	"github.com/rohansx/illuminate/api/internal/service"
)

type UserHandler struct {
	userService *service.UserService
}

func NewUserHandler(userService *service.UserService) *UserHandler {
	return &UserHandler{userService: userService}
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
