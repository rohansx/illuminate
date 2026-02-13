package crypto

import (
	"testing"

	"github.com/google/uuid"
)

func TestJWTGenerateValidate(t *testing.T) {
	mgr := NewJWTManager("test-secret-key-that-is-long-enough")
	userID := uuid.New()

	token, err := mgr.Generate(userID)
	if err != nil {
		t.Fatalf("Generate: %v", err)
	}

	claims, err := mgr.Validate(token)
	if err != nil {
		t.Fatalf("Validate: %v", err)
	}

	if claims.UserID != userID {
		t.Fatalf("got userID %v, want %v", claims.UserID, userID)
	}

	if claims.Issuer != "illuminate" {
		t.Fatalf("got issuer %q, want %q", claims.Issuer, "illuminate")
	}
}

func TestJWTInvalidToken(t *testing.T) {
	mgr := NewJWTManager("test-secret")
	_, err := mgr.Validate("invalid.token.here")
	if err == nil {
		t.Fatal("expected error for invalid token")
	}
}

func TestGenerateRefreshToken(t *testing.T) {
	token, hash, err := GenerateRefreshToken()
	if err != nil {
		t.Fatalf("GenerateRefreshToken: %v", err)
	}
	if len(token) == 0 {
		t.Fatal("expected non-empty token")
	}
	if len(hash) != 32 {
		t.Fatalf("expected 32-byte hash, got %d", len(hash))
	}
}
