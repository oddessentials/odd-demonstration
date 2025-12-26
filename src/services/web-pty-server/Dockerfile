# Web PTY Server Dockerfile
# Multi-stage build for PTY broker service

FROM rust:1.83-slim-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy Cargo files first for dependency caching
COPY Cargo.toml Cargo.lock* ./

# Create a dummy main.rs to build dependencies
RUN mkdir -p src && echo "fn main() {}" > src/main.rs

# Build dependencies only
RUN cargo build --release

# Remove the dummy source
RUN rm -rf src

# Copy real source code
COPY src ./src
COPY VERSION ./

# Build the actual application
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary
COPY --from=builder /app/target/release/web-pty-server /app/
COPY --from=builder /app/VERSION /app/

# Copy the TUI binary (must be built separately and available)
# In production, this would be: COPY --from=tui-builder /app/target/release/odd-dashboard /app/
# For now, we expect it to be mounted or available in PATH

# Environment defaults
ENV PTY_WS_PORT=9000
ENV PTY_METRICS_PORT=9001
ENV PTY_IDLE_TIMEOUT_SECS=1800
ENV PTY_PER_IP_CAP=5
ENV PTY_GLOBAL_CAP=50
ENV PTY_DISCONNECT_GRACE_SECS=30
ENV PTY_MAX_OUTPUT_QUEUE_BYTES=1048576
ENV READ_MODEL_URL=http://read-model:8080
ENV GATEWAY_URL=http://gateway:3000

# Expose ports
EXPOSE 9000
EXPOSE 9001

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s \
    CMD curl -f http://localhost:9001/healthz || exit 1

# Run the server
CMD ["/app/web-pty-server"]
