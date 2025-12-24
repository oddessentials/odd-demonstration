FROM golang:1.21-alpine AS builder

WORKDIR /app

# Copy go module files
COPY go.mod go.sum ./

# Copy Go source files
COPY *.go ./

# Download dependencies
RUN go mod download

# Build
RUN CGO_ENABLED=0 GOOS=linux go build -o /read-model

FROM gcr.io/distroless/static-debian12

WORKDIR /app
COPY --from=builder /read-model /app/read-model
COPY VERSION /app/VERSION

EXPOSE 8080

CMD ["/app/read-model"]
