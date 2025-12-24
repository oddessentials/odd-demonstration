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

COPY --from=builder /read-model /read-model

# Copy VERSION file for runtime version reporting
COPY VERSION /VERSION

EXPOSE 8080

CMD ["/read-model"]
