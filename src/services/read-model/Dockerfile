FROM golang:1.21-alpine AS builder

WORKDIR /app

COPY go.mod ./
COPY *.go ./
RUN go mod tidy
RUN CGO_ENABLED=0 GOOS=linux go build -o /read-model

FROM gcr.io/distroless/static-debian12

COPY --from=builder /read-model /read-model

EXPOSE 8080

CMD ["/read-model"]
