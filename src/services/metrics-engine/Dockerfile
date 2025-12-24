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

COPY --from=builder /metrics-engine /metrics-engine

# Copy VERSION file for runtime version reporting
COPY VERSION /VERSION

CMD ["/metrics-engine"]
