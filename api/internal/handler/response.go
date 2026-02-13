package handler

import (
	"net/http"

	"github.com/rohansx/illuminate/api/internal/httputil"
)

func JSON(w http.ResponseWriter, status int, data any) {
	httputil.JSON(w, status, data)
}

func Error(w http.ResponseWriter, status int, message string) {
	httputil.Error(w, status, message)
}
