package service

import (
	"context"
	"fmt"

	"github.com/google/uuid"
	"github.com/rohansx/illuminate/api/internal/model"
	"github.com/rohansx/illuminate/api/internal/repository"
)

type SavedIssueService struct {
	savedRepo repository.SavedIssueRepo
	issueRepo repository.IssueRepo
}

func NewSavedIssueService(savedRepo repository.SavedIssueRepo, issueRepo repository.IssueRepo) *SavedIssueService {
	return &SavedIssueService{
		savedRepo: savedRepo,
		issueRepo: issueRepo,
	}
}

func (s *SavedIssueService) Save(ctx context.Context, userID, issueID uuid.UUID) error {
	// Verify issue exists
	issue, err := s.issueRepo.GetByID(ctx, issueID)
	if err != nil {
		return fmt.Errorf("checking issue: %w", err)
	}
	if issue == nil {
		return fmt.Errorf("issue not found")
	}
	return s.savedRepo.Save(ctx, userID, issueID)
}

func (s *SavedIssueService) Unsave(ctx context.Context, userID, issueID uuid.UUID) error {
	return s.savedRepo.Unsave(ctx, userID, issueID)
}

func (s *SavedIssueService) IsSaved(ctx context.Context, userID, issueID uuid.UUID) (bool, error) {
	return s.savedRepo.IsSaved(ctx, userID, issueID)
}

func (s *SavedIssueService) GetSaved(ctx context.Context, userID uuid.UUID, page, perPage int) (*model.IssueFeed, error) {
	if page < 1 {
		page = 1
	}
	if perPage < 1 || perPage > 50 {
		perPage = 20
	}
	offset := (page - 1) * perPage

	issues, totalCount, err := s.savedRepo.GetSavedIssues(ctx, userID, perPage, offset)
	if err != nil {
		return nil, fmt.Errorf("getting saved issues: %w", err)
	}

	return &model.IssueFeed{
		Issues:     issues,
		TotalCount: totalCount,
		Page:       page,
		PerPage:    perPage,
	}, nil
}

func (s *SavedIssueService) GetSavedIssueIDs(ctx context.Context, userID uuid.UUID, issueIDs []uuid.UUID) ([]uuid.UUID, error) {
	return s.savedRepo.GetSavedIssueIDs(ctx, userID, issueIDs)
}
