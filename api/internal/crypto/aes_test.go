package crypto

import (
	"encoding/hex"
	"testing"
)

func testKey() string {
	key := make([]byte, 32)
	for i := range key {
		key[i] = byte(i)
	}
	return hex.EncodeToString(key)
}

func TestEncryptDecrypt(t *testing.T) {
	enc, err := NewEncryptor(testKey())
	if err != nil {
		t.Fatalf("NewEncryptor: %v", err)
	}

	plaintext := "gho_test_token_abc123"
	ciphertext, err := enc.Encrypt(plaintext)
	if err != nil {
		t.Fatalf("Encrypt: %v", err)
	}

	result, err := enc.Decrypt(ciphertext)
	if err != nil {
		t.Fatalf("Decrypt: %v", err)
	}

	if result != plaintext {
		t.Fatalf("got %q, want %q", result, plaintext)
	}
}

func TestEncryptProducesDifferentCiphertext(t *testing.T) {
	enc, err := NewEncryptor(testKey())
	if err != nil {
		t.Fatalf("NewEncryptor: %v", err)
	}

	c1, _ := enc.Encrypt("same")
	c2, _ := enc.Encrypt("same")

	if string(c1) == string(c2) {
		t.Fatal("expected different ciphertexts for same plaintext")
	}
}

func TestInvalidKeyLength(t *testing.T) {
	_, err := NewEncryptor("deadbeef")
	if err == nil {
		t.Fatal("expected error for short key")
	}
}
