# TUI (Terminal User Interface)

## Purpose
Terminal-based dashboard for the Distributed Task Observatory. Displays real-time job statistics, recent jobs, and active alerts using a TUI built with Ratatui.

## Run Locally
```bash
cd src/interfaces/tui
cargo run
```

## Test
```bash
cargo test                        # Run unit tests
cargo clippy -- -D warnings       # Lint (strict)
cargo fmt --check                 # Format check
```

## Build / Container
```bash
cargo build --release
docker build -t tui:local .
docker run -it tui:local
```

## Environment Variables
| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| READ_MODEL_URL | No | http://localhost:8080 | Read Model API endpoint |
| GATEWAY_URL | No | http://localhost:3000 | Gateway API endpoint |
