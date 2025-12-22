FROM golang:1.21-alpine AS builder

WORKDIR /app

COPY go.mod ./
COPY *.go ./
RUN go mod tidy
RUN CGO_ENABLED=0 GOOS=linux go build -o /metrics-engine

FROM gcr.io/distroless/static-debian12

COPY --from=builder /metrics-engine /metrics-engine

CMD ["/metrics-engine"]
