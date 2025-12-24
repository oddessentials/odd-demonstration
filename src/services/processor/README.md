# Processor Service

## Purpose
Task processing service for the Distributed Task Observatory. Consumes job events from RabbitMQ, processes tasks, and persists results to PostgreSQL.

## Run Locally
```bash
cd src/services/processor
pip install -r requirements.txt
python main.py
```

## Test
```bash
pytest tests/                    # Run unit tests
pytest --cov=. tests/            # Run with coverage
ruff check .                     # Lint
black --check .                  # Format check
```

## Build / Container
```bash
docker build -t processor:local .
docker run processor:local
```

## Environment Variables
| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| RABBITMQ_URL | No | amqp://guest:guest@rabbitmq:5672 | RabbitMQ connection string |
| DATABASE_URL | No | postgresql://postgres:postgres@postgres:5432/observatory | PostgreSQL connection string |
