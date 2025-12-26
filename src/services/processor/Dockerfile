# src/services/processor/Dockerfile
# Python Processor service
#
# BUILD CONTEXT: Repo root (.)
# start-all.ps1 uses repo root context so contracts/ can be accessed.
# All COPY paths must be repo-relative (e.g., src/services/processor/...)


FROM python:3.11-slim

WORKDIR /app

# Install system dependencies for psycopg2 binary compilation
RUN apt-get update && \
    apt-get install -y --no-install-recommends libpq-dev gcc && \
    rm -rf /var/lib/apt/lists/*

# Copy and install Python dependencies first (cache layer)
COPY src/services/processor/requirements.txt ./
RUN pip install --no-cache-dir -r requirements.txt

# Copy application code
COPY src/services/processor/main.py ./
COPY src/services/processor/schema_validator.py ./
COPY src/services/processor/schemas ./schemas/

# Copy VERSION file for runtime version reporting
COPY src/services/processor/VERSION ./VERSION

# Bake contracts into image for I3 (self-contained) compliance
COPY contracts ./contracts

# Environment defaults
ENV CONTRACTS_PATH=/app/contracts
ENV RABBITMQ_URL=amqp://guest:guest@rabbitmq:5672
ENV POSTGRES_URL=postgresql://admin:password123@postgres:5432/task_db

CMD ["python", "main.py"]
