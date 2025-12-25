# src/services/gateway/Dockerfile
# Multi-stage build for Node.js Gateway service
# Context: ./src/services/gateway (service-local, not repo root)
#
# LOCAL BUILDS: Copy contracts into this directory first:
#   cp -r ../../contracts ./contracts
#   docker build -t odto-gateway:local .

# ============================================================
# Build Stage - Compile TypeScript
# ============================================================
FROM node:20-slim AS builder

WORKDIR /app

# Copy package files for dependency installation
COPY package.json package-lock.json ./
COPY tsconfig.json ./

# Install all dependencies (including devDeps for tsc)
RUN npm ci

# Copy source files
COPY index.ts ./
COPY lib ./lib/

# Compile TypeScript to JavaScript
RUN npx tsc

# Copy VERSION into dist/ where __dirname points at runtime
# Gateway reads: path.join(__dirname, 'VERSION') -> /app/dist/VERSION
COPY VERSION ./dist/VERSION

# ============================================================
# Runtime Stage - Minimal production image
# ============================================================
FROM node:20-slim

WORKDIR /app

# Copy compiled output (includes VERSION)
COPY --from=builder /app/dist ./dist

# Copy package files and install production deps only
COPY package.json package-lock.json ./
RUN npm ci --omit=dev && npm cache clean --force

# Bake contracts into image for I3 (self-contained) compliance
# AJV schema validation must not depend on volume mounts
# NOTE: For local builds, copy contracts/ into this directory first
COPY contracts ./contracts

# Environment defaults
ENV NODE_ENV=production
ENV PORT=3000
ENV CONTRACTS_PATH=/app/contracts
ENV RABBITMQ_URL=amqp://guest:guest@rabbitmq:5672

EXPOSE 3000

# Health check for container orchestration
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD node -e "require('http').get('http://localhost:3000/healthz', (r) => process.exit(r.statusCode === 200 ? 0 : 1))"

CMD ["node", "dist/index.js"]

