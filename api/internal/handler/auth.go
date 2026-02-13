package handler

import (
	"crypto/rand"
	"encoding/base64"
	"encoding/json"
	"net/http"

	"github.com/rohansx/illuminate/api/internal/middleware"
	"github.com/rohansx/illuminate/api/internal/service"
)

type AuthHandler struct {
	authService  *service.AuthService
	github       *service.GitHubService
	frontendURL  string
	cookieDomain string
	isProd       bool
}

func NewAuthHandler(
	authService *service.AuthService,
	github *service.GitHubService,
	frontendURL, cookieDomain string,
	isProd bool,
) *AuthHandler {
	return &AuthHandler{
		authService:  authService,
		github:       github,
		frontendURL:  frontendURL,
		cookieDomain: cookieDomain,
		isProd:       isProd,
	}
}

func (h *AuthHandler) Login(w http.ResponseWriter, r *http.Request) {
	b := make([]byte, 32)
	if _, err := rand.Read(b); err != nil {
		Error(w, http.StatusInternalServerError, "failed to generate state")
		return
	}
	state := base64.URLEncoding.EncodeToString(b)

	http.SetCookie(w, &http.Cookie{
		Name:     "oauth_state",
		Value:    state,
		Path:     "/",
		Domain:   h.cookieDomain,
		MaxAge:   300,
		HttpOnly: true,
		Secure:   h.isProd,
		SameSite: http.SameSiteLaxMode,
	})

	http.Redirect(w, r, h.github.AuthURL(state), http.StatusTemporaryRedirect)
}

func (h *AuthHandler) Callback(w http.ResponseWriter, r *http.Request) {
	stateCookie, err := r.Cookie("oauth_state")
	if err != nil || stateCookie.Value != r.URL.Query().Get("state") {
		Error(w, http.StatusBadRequest, "invalid oauth state")
		return
	}

	// Clear state cookie
	http.SetCookie(w, &http.Cookie{
		Name:     "oauth_state",
		Value:    "",
		Path:     "/",
		Domain:   h.cookieDomain,
		MaxAge:   -1,
		HttpOnly: true,
	})

	code := r.URL.Query().Get("code")
	if code == "" {
		Error(w, http.StatusBadRequest, "missing code")
		return
	}

	result, err := h.authService.HandleCallback(r.Context(), code)
	if err != nil {
		Error(w, http.StatusInternalServerError, "authentication failed")
		return
	}

	h.setAuthCookies(w, result.AccessToken, result.RefreshToken)

	redirectURL := h.frontendURL + "/app/feed"
	if !result.User.OnboardingDone {
		redirectURL = h.frontendURL + "/app/onboarding"
	}

	http.Redirect(w, r, redirectURL, http.StatusTemporaryRedirect)
}

func (h *AuthHandler) Refresh(w http.ResponseWriter, r *http.Request) {
	cookie, err := r.Cookie("refresh_token")
	if err != nil {
		Error(w, http.StatusUnauthorized, "missing refresh token")
		return
	}

	result, err := h.authService.RefreshAccessToken(r.Context(), cookie.Value)
	if err != nil {
		Error(w, http.StatusUnauthorized, "invalid refresh token")
		return
	}

	h.setAuthCookies(w, result.AccessToken, result.RefreshToken)
	JSON(w, http.StatusOK, map[string]string{"status": "ok"})
}

func (h *AuthHandler) Logout(w http.ResponseWriter, r *http.Request) {
	userID := middleware.GetUserID(r.Context())

	var body struct {
		// Accept empty body
	}
	json.NewDecoder(r.Body).Decode(&body)

	if err := h.authService.Logout(r.Context(), userID); err != nil {
		Error(w, http.StatusInternalServerError, "logout failed")
		return
	}

	// Clear cookies
	http.SetCookie(w, &http.Cookie{
		Name:     "access_token",
		Value:    "",
		Path:     "/",
		Domain:   h.cookieDomain,
		MaxAge:   -1,
		HttpOnly: true,
	})
	http.SetCookie(w, &http.Cookie{
		Name:     "refresh_token",
		Value:    "",
		Path:     "/",
		Domain:   h.cookieDomain,
		MaxAge:   -1,
		HttpOnly: true,
	})

	JSON(w, http.StatusOK, map[string]string{"status": "logged out"})
}

func (h *AuthHandler) setAuthCookies(w http.ResponseWriter, accessToken, refreshToken string) {
	http.SetCookie(w, &http.Cookie{
		Name:     "access_token",
		Value:    accessToken,
		Path:     "/",
		Domain:   h.cookieDomain,
		MaxAge:   900, // 15 min
		HttpOnly: true,
		Secure:   h.isProd,
		SameSite: http.SameSiteLaxMode,
	})
	http.SetCookie(w, &http.Cookie{
		Name:     "refresh_token",
		Value:    refreshToken,
		Path:     "/",
		Domain:   h.cookieDomain,
		MaxAge:   7 * 24 * 3600, // 7 days
		HttpOnly: true,
		Secure:   h.isProd,
		SameSite: http.SameSiteLaxMode,
	})
}
