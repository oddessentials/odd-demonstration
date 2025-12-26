# src/services/metrics-engine/Dockerfile
# Multi-stage build for Go Metrics Engine service
# Context: ./src/services/metrics-engine (service-local, not repo root)

# ============================================================
# Build Stage - Compile Go binary
# ============================================================
FROM golang:1.21-alpine AS builder

WORKDIR /app

# Copy go module files for dependency download (cache layer)
COPY go.mod go.sum ./
RUN go mod download

# Copy all Go source files including subdirectories
COPY *.go ./
COPY validator ./validator/

# Build static binary
RUN CGO_ENABLED=0 GOOS=linux go build -ldflags="-s -w" -o /metrics-engine

# ============================================================
# Runtime Stage - Minimal distroless image
# ============================================================
FROM gcr.io/distroless/static-debian12

# Copy compiled binary
COPY --from=builder /metrics-engine /metrics-engine

# Copy VERSION file for runtime version reporting
COPY VERSION /VERSION

CMD ["/metrics-engine"]
