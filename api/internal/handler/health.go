package handler

import "net/http"

func Health(w http.ResponseWriter, r *http.Request) {
	JSON(w, http.StatusOK, map[string]string{
		"status":  "ok",
		"version": "0.1.0",
	})
}
