# src/services/read-model/Dockerfile
# Multi-stage build for Go Read Model service
# Context: ./src/services/read-model (service-local, not repo root)

# ============================================================
# Build Stage - Compile Go binary
# ============================================================
FROM golang:1.21-alpine AS builder

WORKDIR /app

# Copy go module files for dependency download (cache layer)
COPY go.mod go.sum ./
RUN go mod download

# Copy Go source files
COPY *.go ./

# Build static binary
RUN CGO_ENABLED=0 GOOS=linux go build -ldflags="-s -w" -o /read-model

# ============================================================
# Runtime Stage - Minimal distroless image
# ============================================================
FROM gcr.io/distroless/static-debian12

# Copy compiled binary
COPY --from=builder /read-model /read-model

# Copy VERSION file for runtime version reporting
COPY VERSION /VERSION

EXPOSE 8080

CMD ["/read-model"]
