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
| ODD_DASHBOARD_SERVER_MODE | No | (unset) | Set to `1` to bypass prereq checks |

## Server Mode (W11: Container Contract)

When running inside a container via `web-pty-server`, set `ODD_DASHBOARD_SERVER_MODE=1` to skip prerequisite checks. This is **container-only behavior**.

**What changes in server mode:**
- Docker/kubectl/kind/PowerShell detection is bypassed
- `PrerequisiteSetup` view is never entered
- Dashboard/Launcher views load directly
- Prominent warning banner: `⚠️ SERVER MODE: Prereq checks bypassed`

**Local developer runs** default to full prerequisite checks unless the flag is explicitly set. If set locally by accident, the yellow warning banner makes it obvious.

**Who sets the flag:** The `web-pty-server` automatically injects this flag when spawning the TUI in containers. Developers should never need to set it manually.
