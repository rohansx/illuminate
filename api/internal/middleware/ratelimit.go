package middleware

import (
	"context"
	"fmt"
	"log/slog"
	"net/http"
	"sync"
	"sync/atomic"
	"time"

	"github.com/redis/go-redis/v9"
	"github.com/rohansx/illuminate/api/internal/httputil"
)

type memEntry struct {
	count   int64
	resetAt time.Time
}

type memLimiter struct {
	mu      sync.Mutex
	entries map[string]*memEntry
	limit   int
	window  time.Duration
}

func (m *memLimiter) allow(ip string) bool {
	now := time.Now()
	m.mu.Lock()
	defer m.mu.Unlock()

	if m.entries == nil {
		m.entries = make(map[string]*memEntry)
	}

	e, ok := m.entries[ip]
	if !ok || now.After(e.resetAt) {
		m.entries[ip] = &memEntry{count: 1, resetAt: now.Add(m.window)}
		return true
	}

	e.count++
	return e.count <= int64(m.limit)
}

func (m *memLimiter) reject(w http.ResponseWriter, window time.Duration) {
	w.Header().Set("Retry-After", fmt.Sprintf("%d", int(window.Seconds())))
	httputil.Error(w, http.StatusTooManyRequests, "rate limit exceeded")
}

func RateLimit(rdb *redis.Client, limit int, window time.Duration) func(http.Handler) http.Handler {
	fallback := &memLimiter{limit: limit, window: window}
	var fallbackLogged atomic.Bool
	var redisErrLogged atomic.Bool

	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			ip := r.RemoteAddr
			key := fmt.Sprintf("ratelimit:%s", ip)

			if rdb == nil {
				if !fallbackLogged.Swap(true) {
					slog.Warn("rate limiting using in-memory fallback (Redis not configured or unavailable)")
				}
				if !fallback.allow(ip) {
					fallback.reject(w, window)
					return
				}
				next.ServeHTTP(w, r)
				return
			}

			ctx := context.Background()
			count, err := rdb.Incr(ctx, key).Result()
			if err != nil {
				if !redisErrLogged.Swap(true) {
					slog.Warn("Redis unavailable, rate limiting using in-memory fallback", "error", err)
				}
				if !fallback.allow(ip) {
					fallback.reject(w, window)
					return
				}
				next.ServeHTTP(w, r)
				return
			}

			if count == 1 {
				rdb.Expire(ctx, key, window)
			}

			redisErrLogged.Store(false)

			if count > int64(limit) {
				fallback.reject(w, window)
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}
