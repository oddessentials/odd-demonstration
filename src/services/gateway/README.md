# Gateway Service

## Purpose
API Gateway for the Distributed Task Observatory. Receives job submissions via REST API, validates against JSON Schema contracts, and publishes events to RabbitMQ.

## Run Locally
```bash
cd src/services/gateway
npm install
npm run dev  # Uses tsx for TypeScript execution
```

## Test
```bash
npm run test           # Run unit tests
npm run test:coverage  # Run with coverage report
npm run typecheck      # Type check only (no emit)
npm run lint           # ESLint
```

## Build / Container
```bash
npm run build                              # Compile TypeScript
docker build -t gateway:local .            # Build container
docker run -p 3000:3000 gateway:local      # Run container
```

## Environment Variables
| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| PORT | No | 3000 | HTTP server port |
| RABBITMQ_URL | No | amqp://guest:guest@rabbitmq:5672 | RabbitMQ connection string |
| CONTRACTS_PATH | No | ../../../contracts | Path to contract schemas |
| ALERTMANAGER_URL | No | http://alertmanager:9093 | Alertmanager proxy target |
| PROMETHEUS_URL | No | http://prometheus:9090 | Prometheus proxy target |
