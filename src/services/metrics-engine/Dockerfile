FROM golang:1.21-alpine AS builder

WORKDIR /app

# Copy go module files
COPY go.mod go.sum ./

# Copy all Go source files including subdirectories
COPY *.go ./
COPY validator ./validator/

# Download dependencies
RUN go mod download

# Build
RUN CGO_ENABLED=0 GOOS=linux go build -o /metrics-engine

FROM gcr.io/distroless/static-debian12

WORKDIR /app
COPY --from=builder /metrics-engine /app/metrics-engine
COPY VERSION /app/VERSION

CMD ["/app/metrics-engine"]
