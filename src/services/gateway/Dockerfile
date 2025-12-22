# src/services/gateway/Dockerfile
FROM node:20-slim

WORKDIR /app

# Copy package files
COPY src/services/gateway/package.json ./
# We need to create a dummy lock file if it doesn't exist to satisfy COPY or just skip it
# Since we didn't commit a lock file to the specific service dir yet, I'll just skip it for now.
# Actually, npm install will generate it.

# Install dependencies
RUN npm install

# Copy source code
COPY src/services/gateway/index.js ./index.js

# Copy contracts
COPY contracts ./contracts

# Environment defaults
ENV CONTRACTS_PATH=/app/contracts


# Environment defaults
ENV PORT=3000
ENV RABBITMQ_URL=amqp://guest:guest@rabbitmq:5672

EXPOSE 3000

CMD ["node", "index.js"]
