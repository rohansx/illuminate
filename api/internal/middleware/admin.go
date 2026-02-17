package middleware

import (
	"net/http"
	"sync"
	"time"

	"github.com/google/uuid"
	"github.com/rohansx/illuminate/api/internal/httputil"
	"github.com/rohansx/illuminate/api/internal/repository"
)

type roleCache struct {
	mu      sync.RWMutex
	entries map[uuid.UUID]roleCacheEntry
}

type roleCacheEntry struct {
	role      string
	expiresAt time.Time
}

var cache = &roleCache{entries: make(map[uuid.UUID]roleCacheEntry)}

func (c *roleCache) get(id uuid.UUID) (string, bool) {
	c.mu.RLock()
	defer c.mu.RUnlock()
	entry, ok := c.entries[id]
	if !ok || time.Now().After(entry.expiresAt) {
		return "", false
	}
	return entry.role, true
}

func (c *roleCache) set(id uuid.UUID, role string) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.entries[id] = roleCacheEntry{role: role, expiresAt: time.Now().Add(30 * time.Second)}
}

func Admin(userRepo repository.UserRepo) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			userID := GetUserID(r.Context())
			if userID == uuid.Nil {
				httputil.Error(w, http.StatusUnauthorized, "unauthorized")
				return
			}

			role, ok := cache.get(userID)
			if !ok {
				var err error
				role, err = userRepo.GetRole(r.Context(), userID)
				if err != nil {
					httputil.Error(w, http.StatusInternalServerError, "failed to check role")
					return
				}
				cache.set(userID, role)
			}

			if role != "admin" {
				httputil.Error(w, http.StatusForbidden, "admin access required")
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}
