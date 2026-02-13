package middleware

import (
	"context"
	"net/http"

	"github.com/google/uuid"
	"github.com/rohansx/illuminate/api/internal/crypto"
	"github.com/rohansx/illuminate/api/internal/httputil"
)

type contextKey string

const UserIDKey contextKey = "user_id"

func Auth(jwt *crypto.JWTManager) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			cookie, err := r.Cookie("access_token")
			if err != nil {
				httputil.Error(w, http.StatusUnauthorized, "missing access token")
				return
			}

			claims, err := jwt.Validate(cookie.Value)
			if err != nil {
				httputil.Error(w, http.StatusUnauthorized, "invalid access token")
				return
			}

			ctx := context.WithValue(r.Context(), UserIDKey, claims.UserID)
			next.ServeHTTP(w, r.WithContext(ctx))
		})
	}
}

func GetUserID(ctx context.Context) uuid.UUID {
	id, _ := ctx.Value(UserIDKey).(uuid.UUID)
	return id
}
