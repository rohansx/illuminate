package handler

import (
	"net/http"
	"strconv"
	"strings"

	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"
	"github.com/rohansx/illuminate/api/internal/middleware"
	"github.com/rohansx/illuminate/api/internal/model"
	"github.com/rohansx/illuminate/api/internal/service"
)

type IssueHandler struct {
	issueService    *service.IssueService
	userService     *service.UserService
	matchingService *service.MatchingService
	savedService    *service.SavedIssueService
}

func NewIssueHandler(
	issueService *service.IssueService,
	userService *service.UserService,
	matchingService *service.MatchingService,
	savedService *service.SavedIssueService,
) *IssueHandler {
	return &IssueHandler{
		issueService:    issueService,
		userService:     userService,
		matchingService: matchingService,
		savedService:    savedService,
	}
}

func (h *IssueHandler) GetFeed(w http.ResponseWriter, r *http.Request) {
	userID := middleware.GetUserID(r.Context())
	page, _ := strconv.Atoi(r.URL.Query().Get("page"))
	perPage, _ := strconv.Atoi(r.URL.Query().Get("per_page"))

	// Get user's skills to filter by language
	user, err := h.userService.GetProfile(r.Context(), userID)
	if err != nil {
		Error(w, http.StatusInternalServerError, "failed to get user")
		return
	}

	var languages []string
	for _, skill := range user.Skills {
		languages = append(languages, skill.Language)
	}

	// Allow language override from query
	if langParam := r.URL.Query().Get("languages"); langParam != "" {
		languages = strings.Split(langParam, ",")
	}

	// Build filter
	filter := model.FeedFilter{
		Languages: languages,
		Category:  r.URL.Query().Get("category"),
	}
	if d, err := strconv.Atoi(r.URL.Query().Get("difficulty")); err == nil && d >= 1 && d <= 3 {
		filter.Difficulty = d
	}

	feed, err := h.issueService.GetFeed(r.Context(), filter, page, perPage)
	if err != nil {
		Error(w, http.StatusInternalServerError, "failed to get feed")
		return
	}

	// Score and rank issues for this user
	feed.Issues = h.matchingService.ScoreIssues(user, feed.Issues)

	// Enrich with saved status
	if len(feed.Issues) > 0 {
		issueIDs := make([]uuid.UUID, len(feed.Issues))
		for i, issue := range feed.Issues {
			issueIDs[i] = issue.ID
		}
		savedIDs, err := h.savedService.GetSavedIssueIDs(r.Context(), userID, issueIDs)
		if err == nil {
			savedSet := make(map[uuid.UUID]bool, len(savedIDs))
			for _, id := range savedIDs {
				savedSet[id] = true
			}
			for i := range feed.Issues {
				feed.Issues[i].IsSaved = savedSet[feed.Issues[i].ID]
			}
		}
	}

	JSON(w, http.StatusOK, feed)
}

func (h *IssueHandler) GetCategories(w http.ResponseWriter, r *http.Request) {
	cats, err := h.issueService.GetCategories(r.Context())
	if err != nil {
		Error(w, http.StatusInternalServerError, "failed to get categories")
		return
	}
	JSON(w, http.StatusOK, cats)
}

func (h *IssueHandler) GetByID(w http.ResponseWriter, r *http.Request) {
	idStr := chi.URLParam(r, "id")
	id, err := uuid.Parse(idStr)
	if err != nil {
		Error(w, http.StatusBadRequest, "invalid issue id")
		return
	}

	issue, err := h.issueService.GetByID(r.Context(), id)
	if err != nil {
		Error(w, http.StatusInternalServerError, "failed to get issue")
		return
	}
	if issue == nil {
		Error(w, http.StatusNotFound, "issue not found")
		return
	}

	JSON(w, http.StatusOK, issue)
}

func (h *IssueHandler) GetComments(w http.ResponseWriter, r *http.Request) {
	idStr := chi.URLParam(r, "id")
	id, err := uuid.Parse(idStr)
	if err != nil {
		Error(w, http.StatusBadRequest, "invalid issue id")
		return
	}

	comments, err := h.issueService.GetComments(r.Context(), id)
	if err != nil {
		Error(w, http.StatusInternalServerError, "failed to get comments")
		return
	}

	JSON(w, http.StatusOK, comments)
}

func (h *IssueHandler) Search(w http.ResponseWriter, r *http.Request) {
	query := r.URL.Query().Get("q")
	if query == "" {
		Error(w, http.StatusBadRequest, "missing search query")
		return
	}

	page, _ := strconv.Atoi(r.URL.Query().Get("page"))
	perPage, _ := strconv.Atoi(r.URL.Query().Get("per_page"))

	feed, err := h.issueService.Search(r.Context(), query, page, perPage)
	if err != nil {
		Error(w, http.StatusInternalServerError, "search failed")
		return
	}

	JSON(w, http.StatusOK, feed)
}
