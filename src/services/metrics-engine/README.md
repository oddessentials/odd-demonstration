# Metrics Engine Service

## Purpose
Metrics aggregation service for the Distributed Task Observatory. Consumes events from RabbitMQ, computes aggregates, and stores results in MongoDB and Redis.

## Run Locally
```bash
cd src/services/metrics-engine
go run main.go
```

## Test
```bash
go test ./...                     # Run unit tests
go test -cover ./...              # Run with coverage
golangci-lint run                 # Lint
go fmt ./...                      # Format
```

## Build / Container
```bash
go build -o metrics-engine .
docker build -t metrics-engine:local .
docker run metrics-engine:local
```

## Environment Variables
| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| RABBITMQ_URL | No | amqp://guest:guest@rabbitmq:5672 | RabbitMQ connection string |
| MONGODB_URI | No | mongodb://mongodb:27017 | MongoDB connection string |
| REDIS_URL | No | redis://redis:6379 | Redis connection string |
