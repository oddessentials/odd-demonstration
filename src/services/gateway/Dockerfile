# src/services/gateway/Dockerfile
FROM node:20-slim

WORKDIR /app

# Copy package files and tsconfig
COPY src/services/gateway/package.json ./
COPY src/services/gateway/tsconfig.json ./

# Install dependencies (including dev deps for ts compilation)
RUN npm install

# Copy source code
COPY src/services/gateway/index.ts ./index.ts

# Copy VERSION file
COPY src/services/gateway/VERSION ./VERSION

# Copy contracts
COPY contracts ./contracts

# Compile TypeScript to JavaScript (outputs to ./dist per tsconfig)
RUN npx tsc

# Copy VERSION to dist directory as well (where the app looks for it)
RUN cp VERSION dist/VERSION

# Environment defaults
ENV CONTRACTS_PATH=/app/contracts
ENV PORT=3000
ENV RABBITMQ_URL=amqp://guest:guest@rabbitmq:5672

EXPOSE 3000

# Run from dist directory where tsc outputs
CMD ["node", "dist/index.js"]
