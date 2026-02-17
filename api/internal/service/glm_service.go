package service

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"
)

const (
	glmAPIURL = "https://open.bigmodel.cn/api/paas/v4/chat/completions"
	glmModel  = "glm-4.5-air"
)

type GLMService struct {
	apiKey string
	client *http.Client
}

func NewGLMService(apiKey string) *GLMService {
	return &GLMService{
		apiKey: apiKey,
		client: &http.Client{Timeout: 120 * time.Second},
	}
}

type glmMessage struct {
	Role    string `json:"role"`
	Content string `json:"content"`
}

type glmRequest struct {
	Model    string       `json:"model"`
	Messages []glmMessage `json:"messages"`
}

type glmResponse struct {
	ID      string `json:"id"`
	Model   string `json:"model"`
	Choices []struct {
		Message struct {
			Role    string `json:"role"`
			Content string `json:"content"`
		} `json:"message"`
		FinishReason string `json:"finish_reason"`
	} `json:"choices"`
	Usage struct {
		PromptTokens     int `json:"prompt_tokens"`
		CompletionTokens int `json:"completion_tokens"`
		TotalTokens      int `json:"total_tokens"`
	} `json:"usage"`
}

type GLMResult struct {
	Text             string
	Model            string
	PromptTokens     int
	CompletionTokens int
}

func (s *GLMService) IsConfigured() bool {
	return s.apiKey != ""
}

func (s *GLMService) SendMessage(ctx context.Context, system, userMessage string) (*GLMResult, error) {
	if !s.IsConfigured() {
		return nil, fmt.Errorf("GLM API key not configured")
	}

	messages := []glmMessage{
		{Role: "system", Content: system},
		{Role: "user", Content: userMessage},
	}

	reqBody := glmRequest{
		Model:    glmModel,
		Messages: messages,
	}

	jsonBody, err := json.Marshal(reqBody)
	if err != nil {
		return nil, fmt.Errorf("marshalling request: %w", err)
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodPost, glmAPIURL, bytes.NewReader(jsonBody))
	if err != nil {
		return nil, fmt.Errorf("creating request: %w", err)
	}

	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+s.apiKey)

	resp, err := s.client.Do(req)
	if err != nil {
		return nil, fmt.Errorf("calling GLM API: %w", err)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("reading response: %w", err)
	}

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("GLM API error (%d): %s", resp.StatusCode, string(body))
	}

	var glmResp glmResponse
	if err := json.Unmarshal(body, &glmResp); err != nil {
		return nil, fmt.Errorf("parsing response: %w", err)
	}

	if len(glmResp.Choices) == 0 {
		return nil, fmt.Errorf("empty response from GLM")
	}

	return &GLMResult{
		Text:             glmResp.Choices[0].Message.Content,
		Model:            glmResp.Model,
		PromptTokens:     glmResp.Usage.PromptTokens,
		CompletionTokens: glmResp.Usage.CompletionTokens,
	}, nil
}
