# src/services/gateway/Dockerfile
# Multi-stage build for Node.js Gateway service
#
# BUILD CONTEXT: Repo root (.)
# start-all.ps1 uses repo root context so contracts/ can be accessed.
# All COPY paths must be repo-relative (e.g., src/services/gateway/...)


# ============================================================
# Build Stage - Compile TypeScript
# ============================================================
FROM node:20-slim AS builder

WORKDIR /app

# Copy package files for dependency installation
COPY src/services/gateway/package.json src/services/gateway/package-lock.json ./
COPY src/services/gateway/tsconfig.json ./

# Install all dependencies (including devDeps for tsc)
RUN npm ci

# Copy source files
COPY src/services/gateway/index.ts ./
COPY src/services/gateway/lib ./lib/

# Compile TypeScript to JavaScript
RUN npx tsc

# Copy VERSION into dist/ where __dirname points at runtime
# Gateway reads: path.join(__dirname, 'VERSION') -> /app/dist/VERSION
COPY src/services/gateway/VERSION ./dist/VERSION

# ============================================================
# Runtime Stage - Minimal production image
# ============================================================
FROM node:20-slim

WORKDIR /app

# Copy compiled output (includes VERSION)
COPY --from=builder /app/dist ./dist

# Copy package files and install production deps only
COPY src/services/gateway/package.json src/services/gateway/package-lock.json ./
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

