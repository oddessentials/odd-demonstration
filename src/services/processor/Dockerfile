# src/services/processor/Dockerfile
FROM python:3.11-slim

WORKDIR /app

# Install system dependencies for psycopg2
RUN apt-get update && apt-get install -y libpq-dev gcc && rm -rf /var/lib/apt/lists/*

# Copy requirements
COPY src/services/processor/requirements.txt ./

# Install dependencies
RUN pip install --no-cache-dir -r requirements.txt

# Copy source code (all Python files including schema_validator)
COPY src/services/processor/main.py ./main.py
COPY src/services/processor/schema_validator.py ./schema_validator.py

# Copy VERSION file
COPY src/services/processor/VERSION ./VERSION

# Copy schemas directory
COPY src/services/processor/schemas ./schemas

# Copy contracts
COPY contracts ./contracts

# Environment defaults
ENV CONTRACTS_PATH=/app/contracts
ENV RABBITMQ_URL=amqp://guest:guest@rabbitmq:5672
ENV POSTGRES_URL=postgresql://admin:password123@postgres:5432/task_db

CMD ["python", "main.py"]
