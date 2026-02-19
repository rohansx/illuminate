package service

import (
	"context"
	"fmt"

	"github.com/google/uuid"
	"github.com/rohansx/illuminate/api/internal/model"
	"github.com/rohansx/illuminate/api/internal/repository"
)

var validStatuses = map[string]bool{
	"interested":   true,
	"researching":  true,
	"working":      true,
	"pr_submitted": true,
	"completed":    true,
	"abandoned":    true,
}

type IssueProgressService struct {
	repo repository.IssueProgressRepo
}

func NewIssueProgressService(repo repository.IssueProgressRepo) *IssueProgressService {
	return &IssueProgressService{repo: repo}
}

func (s *IssueProgressService) Get(ctx context.Context, userID, issueID uuid.UUID) (*model.IssueProgress, error) {
	return s.repo.Get(ctx, userID, issueID)
}

func (s *IssueProgressService) Upsert(ctx context.Context, userID, issueID uuid.UUID, status string) (*model.IssueProgress, error) {
	if !validStatuses[status] {
		return nil, fmt.Errorf("invalid status: %s", status)
	}
	return s.repo.Upsert(ctx, userID, issueID, status)
}

func (s *IssueProgressService) UpdateStatus(ctx context.Context, userID, issueID uuid.UUID, status string) (*model.IssueProgress, error) {
	if !validStatuses[status] {
		return nil, fmt.Errorf("invalid status: %s", status)
	}
	return s.repo.UpdateStatus(ctx, userID, issueID, status)
}

func (s *IssueProgressService) AddNote(ctx context.Context, userID, issueID uuid.UUID, note string) (*model.IssueProgress, error) {
	if note == "" {
		return nil, fmt.Errorf("note cannot be empty")
	}
	return s.repo.AddNote(ctx, userID, issueID, note)
}

func (s *IssueProgressService) Delete(ctx context.Context, userID, issueID uuid.UUID) error {
	return s.repo.Delete(ctx, userID, issueID)
}

func (s *IssueProgressService) ListByUser(ctx context.Context, userID uuid.UUID) ([]model.IssueProgress, error) {
	return s.repo.ListByUser(ctx, userID)
}
