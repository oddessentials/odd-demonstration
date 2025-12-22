# src/services/processor/Dockerfile
FROM python:3.11-slim

WORKDIR /app

# Install system dependencies for psycopg2
RUN apt-get update && apt-get install -y libpq-dev gcc && rm -rf /var/lib/apt/lists/*

# Copy requirements
COPY src/services/processor/requirements.txt ./

# Install dependencies
RUN pip install --no-cache-dir -r requirements.txt

# Copy source code
COPY src/services/processor/main.py ./main.py

# Copy contracts
COPY contracts ./contracts

# Environment defaults
ENV CONTRACTS_PATH=/app/contracts


# Environment defaults
ENV RABBITMQ_URL=amqp://guest:guest@rabbitmq:5672
ENV POSTGRES_URL=postgresql://admin:password123@postgres:5432/task_db

CMD ["python", "main.py"]
