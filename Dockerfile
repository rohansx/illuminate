# ── Stage 1: Build frontend ──────────────────────────────────────────
FROM oven/bun:1 AS frontend
WORKDIR /app/web
COPY web/package.json web/bun.lock* ./
RUN bun install --frozen-lockfile
COPY web/ .
ENV VITE_API_BASE=""
RUN bun run build

# ── Stage 2: Build Go binary ────────────────────────────────────────
FROM golang:1.25-alpine AS backend
WORKDIR /app
COPY api/go.mod api/go.sum ./
RUN go mod download
COPY api/ .
RUN CGO_ENABLED=0 GOOS=linux go build -o /illuminate ./cmd/server

# ── Stage 3: Runtime ────────────────────────────────────────────────
FROM alpine:3.21
RUN apk add --no-cache ca-certificates curl

# Install dbmate for migrations
RUN curl -fsSL -o /usr/local/bin/dbmate \
    https://github.com/amacneil/dbmate/releases/latest/download/dbmate-linux-amd64 && \
    chmod +x /usr/local/bin/dbmate

WORKDIR /app

# Copy binary
COPY --from=backend /illuminate .

# Copy frontend build
COPY --from=frontend /app/web/build ./web/build

# Copy migrations
COPY api/migrations ./migrations

# Copy seed data
COPY api/data ./data

# Copy entrypoint
COPY docker-entrypoint.sh .
RUN chmod +x docker-entrypoint.sh

EXPOSE 8080

ENTRYPOINT ["./docker-entrypoint.sh"]
