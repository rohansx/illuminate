package repository

import (
	"context"
	"fmt"

	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
	"github.com/rohansx/illuminate/api/internal/model"
)

type NotificationRepo interface {
	Create(ctx context.Context, notif *model.Notification) (*model.Notification, error)
	ListByUser(ctx context.Context, userID uuid.UUID, limit, offset int) ([]model.Notification, int, error)
	CountUnread(ctx context.Context, userID uuid.UUID) (int, error)
	MarkRead(ctx context.Context, id, userID uuid.UUID) error
	MarkAllRead(ctx context.Context, userID uuid.UUID) error
}

type notificationRepo struct {
	pool *pgxpool.Pool
}

func NewNotificationRepo(pool *pgxpool.Pool) NotificationRepo {
	return &notificationRepo{pool: pool}
}

func (r *notificationRepo) Create(ctx context.Context, notif *model.Notification) (*model.Notification, error) {
	err := r.pool.QueryRow(ctx, `
		INSERT INTO notifications (user_id, type, title, message, link)
		VALUES ($1, $2, $3, $4, $5)
		RETURNING id, user_id, type, title, message, link, read, created_at`,
		notif.UserID, notif.Type, notif.Title, notif.Message, notif.Link,
	).Scan(
		&notif.ID, &notif.UserID, &notif.Type, &notif.Title, &notif.Message,
		&notif.Link, &notif.Read, &notif.CreatedAt,
	)
	if err != nil {
		return nil, fmt.Errorf("creating notification: %w", err)
	}
	return notif, nil
}

func (r *notificationRepo) ListByUser(ctx context.Context, userID uuid.UUID, limit, offset int) ([]model.Notification, int, error) {
	var total int
	if err := r.pool.QueryRow(ctx, `SELECT COUNT(*) FROM notifications WHERE user_id = $1`, userID).Scan(&total); err != nil {
		return nil, 0, fmt.Errorf("counting notifications: %w", err)
	}

	rows, err := r.pool.Query(ctx, `
		SELECT id, user_id, type, title, message, link, read, created_at
		FROM notifications
		WHERE user_id = $1
		ORDER BY created_at DESC
		LIMIT $2 OFFSET $3`, userID, limit, offset)
	if err != nil {
		return nil, 0, fmt.Errorf("listing notifications: %w", err)
	}
	defer rows.Close()

	var notifs []model.Notification
	for rows.Next() {
		var n model.Notification
		if err := rows.Scan(&n.ID, &n.UserID, &n.Type, &n.Title, &n.Message, &n.Link, &n.Read, &n.CreatedAt); err != nil {
			return nil, 0, fmt.Errorf("scanning notification: %w", err)
		}
		notifs = append(notifs, n)
	}
	return notifs, total, nil
}

func (r *notificationRepo) CountUnread(ctx context.Context, userID uuid.UUID) (int, error) {
	var count int
	err := r.pool.QueryRow(ctx, `SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND read = FALSE`, userID).Scan(&count)
	if err != nil {
		return 0, fmt.Errorf("counting unread: %w", err)
	}
	return count, nil
}

func (r *notificationRepo) MarkRead(ctx context.Context, id, userID uuid.UUID) error {
	_, err := r.pool.Exec(ctx, `UPDATE notifications SET read = TRUE WHERE id = $1 AND user_id = $2`, id, userID)
	if err != nil {
		return fmt.Errorf("marking read: %w", err)
	}
	return nil
}

func (r *notificationRepo) MarkAllRead(ctx context.Context, userID uuid.UUID) error {
	_, err := r.pool.Exec(ctx, `UPDATE notifications SET read = TRUE WHERE user_id = $1 AND read = FALSE`, userID)
	if err != nil {
		return fmt.Errorf("marking all read: %w", err)
	}
	return nil
}
