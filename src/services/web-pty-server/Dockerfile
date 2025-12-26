# Web PTY Server Dockerfile
# Multi-stage build for PTY broker service with embedded TUI binary
#
# Build modes:
#   PTY_TUI_MODE=real (default) - Builds and embeds the actual odd-dashboard TUI
#   PTY_TUI_MODE=mock - Uses a lightweight mock binary for fast CI testing
#
# Build from repo root: docker build -f src/services/web-pty-server/Dockerfile .

ARG PTY_TUI_MODE=real

# ============================================================
# Stage 1: Build web-pty-server (Rust)
# ============================================================
FROM rust:1.83-slim-bookworm AS pty-builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy Cargo files first for dependency caching
COPY src/services/web-pty-server/Cargo.toml src/services/web-pty-server/Cargo.lock* ./

# Create a dummy main.rs to build dependencies
RUN mkdir -p src && echo "fn main() {}" > src/main.rs

# Build dependencies only
RUN cargo build --release

# Remove the dummy source
RUN rm -rf src

# Copy real source code
COPY src/services/web-pty-server/src ./src
COPY src/services/web-pty-server/VERSION ./

# Build the actual application
RUN touch src/main.rs && cargo build --release

# ============================================================
# Stage 2: Build odd-dashboard TUI (Rust) - only for real mode
# ============================================================
FROM rust:1.83-slim-bookworm AS tui-builder

WORKDIR /app

# Install build dependencies (TUI needs more for cross-term/clipboard)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libxcb1-dev \
    libxcb-render0-dev \
    libxcb-shape0-dev \
    libxcb-xfixes0-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy Cargo files first for dependency caching
COPY src/interfaces/tui/Cargo.toml src/interfaces/tui/Cargo.lock* ./

# Create dummy files to build dependencies
RUN mkdir -p src && echo "fn main() {}" > src/main.rs && echo "// lib" > src/lib.rs

# Build dependencies only
RUN cargo build --release || true

# Remove the dummy source
RUN rm -rf src

# Copy real source code
COPY src/interfaces/tui/src ./src
COPY src/interfaces/tui/build.rs ./

# Build the actual application
RUN touch src/main.rs src/lib.rs && cargo build --release

# ============================================================
# Stage 3: Create mock binary (minimal shell script)
# ============================================================
FROM debian:bookworm-slim AS mock-builder

RUN mkdir -p /app && printf '%s\n' \
    '#!/bin/bash' \
    '# Mock TUI binary for integration testing' \
    '# Handle --help flag for validation (exits immediately)' \
    'if [[ "$1" == "--help" || "$1" == "-h" || "$1" == "--version" ]]; then' \
    '  echo "odd-dashboard mock v1.0.0"' \
    '  exit 0' \
    'fi' \
    'echo "ODTO Mock Terminal - Press Ctrl+C to exit"' \
    'echo "Mode: Integration Test (mock)"' \
    'while true; do' \
    '  sleep 1' \
    '  echo "[$(date +%H:%M:%S)] Heartbeat"' \
    'done' > /app/odd-dashboard && chmod +x /app/odd-dashboard

# ============================================================
# Stage 4: Runtime (select real or mock based on ARG)
# ============================================================
FROM debian:bookworm-slim AS runtime-base

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy web-pty-server binary
COPY --from=pty-builder /app/target/release/web-pty-server /app/
COPY --from=pty-builder /app/VERSION /app/

# Environment defaults - canonical path contract
ENV PTY_TUI_BINARY=/app/odd-dashboard
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

# ============================================================
# Final stage: Real mode (default)
# ============================================================
FROM runtime-base AS real

# Copy real TUI binary
COPY --from=tui-builder /app/target/release/odd-dashboard /app/

CMD ["/app/web-pty-server"]

# ============================================================
# Final stage: Mock mode (for CI)
# ============================================================
FROM runtime-base AS mock

# Copy mock script
COPY --from=mock-builder /app/odd-dashboard /app/

# Install bash for mock script
RUN apt-get update && apt-get install -y bash && rm -rf /var/lib/apt/lists/*

CMD ["/app/web-pty-server"]

# ============================================================
# Default target (selected by --target or PTY_TUI_MODE)
# ============================================================
FROM ${PTY_TUI_MODE} AS final
