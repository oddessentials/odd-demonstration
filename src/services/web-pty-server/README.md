# Web PTY Server

WebSocket PTY broker for streaming odd-dashboard TUI to xterm.js in the browser.

## Architecture

The server manages PTY sessions that run the `odd-dashboard` binary and stream terminal output to connected WebSocket clients.

## Features

- **Session Management (R2)**: Single-use reconnect tokens with rotation
- **Resource Limits (R3)**: Per-IP and global session caps, idle timeout
- **Read-Only Mode (R4)**: Filter mutating inputs with rate-limited notices
- **Authentication (R5)**: Bearer token on WebSocket upgrade (never logged)
- **Keepalive (R6)**: WebSocket ping/pong mechanism
- **Terminal Capabilities (R7)**: TERM=xterm-256color, UTF-8, truecolor
- **Output Coalescing (R9)**: Batch writes with backpressure

## Configuration

All configuration is via environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `PTY_WS_PORT` | 9000 | WebSocket server port |
| `PTY_METRICS_PORT` | 9001 | Metrics/health server port |
| `PTY_TUI_BINARY` | odd-dashboard | Path to TUI binary |
| `PTY_AUTH_TOKEN` | (none) | Bearer token for auth (optional) |
| `PTY_READ_ONLY` | false | Enable read-only mode |
| `PTY_IDLE_TIMEOUT_SECS` | 1800 | Session idle timeout |
| `PTY_PER_IP_CAP` | 5 | Max sessions per client IP |
| `PTY_GLOBAL_CAP` | 50 | Max total sessions |
| `PTY_DISCONNECT_GRACE_SECS` | 30 | Grace period after disconnect |
| `PTY_MAX_OUTPUT_QUEUE_BYTES` | 1048576 | Max output buffer size |

## Protocol

### Client → Server

```json
{"type":"input","data":"q"}         // Keyboard input
{"type":"resize","cols":120,"rows":40}  // Terminal resize
{"type":"ping"}                     // Keepalive ping
```

### Server → Client

```json
{"type":"session","sessionId":"...","reconnectToken":"..."}
{"type":"reconnected","sessionId":"...","reconnectToken":"..."}
{"type":"output","data":"..."}      // Terminal output
{"type":"notice","message":"..."}   // Read-only mode notice
{"type":"pong"}                     // Keepalive pong
{"type":"error","message":"...","code":"..."}
```

## Endpoints

- `ws://host:9000/` - WebSocket connection
- `http://host:9001/healthz` - Health check
- `http://host:9001/readyz` - Readiness check
- `http://host:9001/metrics` - Prometheus metrics

## Development

```bash
# Build
cargo build

# Test (35 unit tests)
cargo test

# Run locally
PTY_TUI_BINARY=path/to/odd-dashboard cargo run
```
