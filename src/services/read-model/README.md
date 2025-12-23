# Read Model Service

## Purpose
Read-side query service for the Distributed Task Observatory. Provides REST API for querying job statistics and recent jobs from PostgreSQL, MongoDB, and Redis.

## Run Locally
```bash
cd src/services/read-model
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
go build -o read-model .
docker build -t read-model:local .
docker run -p 8080:8080 read-model:local
```

## Environment Variables
| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| PORT | No | 8080 | HTTP server port |
| POSTGRES_URL | No | postgresql://postgres:postgres@postgres:5432/observatory | PostgreSQL connection string |
| MONGODB_URI | No | mongodb://mongodb:27017 | MongoDB connection string |
| REDIS_URL | No | redis://redis:6379 | Redis connection string |
