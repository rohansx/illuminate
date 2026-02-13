package middleware

import (
	"context"
	"fmt"
	"net/http"
	"time"

	"github.com/redis/go-redis/v9"
	"github.com/rohansx/illuminate/api/internal/httputil"
)

func RateLimit(rdb *redis.Client, limit int, window time.Duration) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			if rdb == nil {
				next.ServeHTTP(w, r)
				return
			}

			ip := r.RemoteAddr
			key := fmt.Sprintf("ratelimit:%s", ip)

			ctx := context.Background()
			count, err := rdb.Incr(ctx, key).Result()
			if err != nil {
				// If Redis is down, allow the request
				next.ServeHTTP(w, r)
				return
			}

			if count == 1 {
				rdb.Expire(ctx, key, window)
			}

			if count > int64(limit) {
				w.Header().Set("Retry-After", fmt.Sprintf("%d", int(window.Seconds())))
				httputil.Error(w, http.StatusTooManyRequests, "rate limit exceeded")
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}
