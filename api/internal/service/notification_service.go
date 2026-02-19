package service

import (
	"context"
	"fmt"

	"github.com/google/uuid"
	"github.com/rohansx/illuminate/api/internal/model"
	"github.com/rohansx/illuminate/api/internal/repository"
)

type NotificationService struct {
	notifRepo repository.NotificationRepo
}

func NewNotificationService(notifRepo repository.NotificationRepo) *NotificationService {
	return &NotificationService{notifRepo: notifRepo}
}

func (s *NotificationService) Create(ctx context.Context, userID uuid.UUID, notifType, title, message, link string) error {
	notif := &model.Notification{
		UserID:  userID,
		Type:    notifType,
		Title:   title,
		Message: message,
		Link:    link,
	}
	_, err := s.notifRepo.Create(ctx, notif)
	if err != nil {
		return fmt.Errorf("creating notification: %w", err)
	}
	return nil
}

func (s *NotificationService) ListByUser(ctx context.Context, userID uuid.UUID, page, perPage int) (*model.NotificationList, error) {
	if page < 1 {
		page = 1
	}
	if perPage < 1 || perPage > 50 {
		perPage = 20
	}
	offset := (page - 1) * perPage

	notifs, total, err := s.notifRepo.ListByUser(ctx, userID, perPage, offset)
	if err != nil {
		return nil, err
	}

	unread, err := s.notifRepo.CountUnread(ctx, userID)
	if err != nil {
		return nil, err
	}

	if notifs == nil {
		notifs = []model.Notification{}
	}

	return &model.NotificationList{
		Notifications: notifs,
		TotalCount:    total,
		UnreadCount:   unread,
	}, nil
}

func (s *NotificationService) CountUnread(ctx context.Context, userID uuid.UUID) (int, error) {
	return s.notifRepo.CountUnread(ctx, userID)
}

func (s *NotificationService) MarkRead(ctx context.Context, id, userID uuid.UUID) error {
	return s.notifRepo.MarkRead(ctx, id, userID)
}

func (s *NotificationService) MarkAllRead(ctx context.Context, userID uuid.UUID) error {
	return s.notifRepo.MarkAllRead(ctx, userID)
}
