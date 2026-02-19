package handler

import (
	"net/http"
	"strconv"

	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"
	"github.com/rohansx/illuminate/api/internal/middleware"
	"github.com/rohansx/illuminate/api/internal/service"
)

type NotificationHandler struct {
	notifService *service.NotificationService
}

func NewNotificationHandler(notifService *service.NotificationService) *NotificationHandler {
	return &NotificationHandler{notifService: notifService}
}

func (h *NotificationHandler) List(w http.ResponseWriter, r *http.Request) {
	userID := middleware.GetUserID(r.Context())
	page, _ := strconv.Atoi(r.URL.Query().Get("page"))
	perPage, _ := strconv.Atoi(r.URL.Query().Get("per_page"))

	list, err := h.notifService.ListByUser(r.Context(), userID, page, perPage)
	if err != nil {
		Error(w, http.StatusInternalServerError, "failed to list notifications")
		return
	}

	JSON(w, http.StatusOK, list)
}

func (h *NotificationHandler) UnreadCount(w http.ResponseWriter, r *http.Request) {
	userID := middleware.GetUserID(r.Context())

	count, err := h.notifService.CountUnread(r.Context(), userID)
	if err != nil {
		Error(w, http.StatusInternalServerError, "failed to get unread count")
		return
	}

	JSON(w, http.StatusOK, map[string]int{"count": count})
}

func (h *NotificationHandler) MarkRead(w http.ResponseWriter, r *http.Request) {
	userID := middleware.GetUserID(r.Context())
	idStr := chi.URLParam(r, "id")
	id, err := uuid.Parse(idStr)
	if err != nil {
		Error(w, http.StatusBadRequest, "invalid notification id")
		return
	}

	if err := h.notifService.MarkRead(r.Context(), id, userID); err != nil {
		Error(w, http.StatusInternalServerError, "failed to mark read")
		return
	}

	JSON(w, http.StatusOK, map[string]string{"status": "ok"})
}

func (h *NotificationHandler) MarkAllRead(w http.ResponseWriter, r *http.Request) {
	userID := middleware.GetUserID(r.Context())

	if err := h.notifService.MarkAllRead(r.Context(), userID); err != nil {
		Error(w, http.StatusInternalServerError, "failed to mark all read")
		return
	}

	JSON(w, http.StatusOK, map[string]string{"status": "ok"})
}
