# ODTO Web Terminal

xterm.js-based terminal that mirrors the TUI via WebSocket PTY streaming.

## Features

- **Identical Rendering**: xterm.js renders exactly what the TUI displays
- **Session Reconnect**: Refreshing the page reconnects to the same PTY session
- **Auto-Reconnect**: Automatic reconnection with exponential backoff on disconnect
- **Fallback Dashboard**: Shows stats and instructions when terminal unavailable

## Architecture

```
Browser (xterm.js) <--WebSocket--> nginx (/ws proxy) <--> web-pty-ws (PTY broker)
                                     |
                                     +--> /api (read-model proxy for fallback stats)
```

## Access

Single access URL: http://localhost:8081

All WebSocket traffic flows through nginx proxy - no direct PTY server access.

## Configuration

The PTY server is configured via environment variables in the Kubernetes manifest.
See `src/services/web-pty-server/README.md` for details.

## Development

```bash
# Serve locally (requires PTY server running)
npx serve .

# Install xterm.js dependencies (for bundled builds)
npm install
npm run build
```
