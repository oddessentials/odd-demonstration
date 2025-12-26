# Visual Regression Tests

Playwright-based visual regression tests for the ODTO Web Terminal.

## Prerequisites

```bash
# Install dependencies
cd tests/visual
npm install
npx playwright install chromium
```

## Running Tests

```bash
# Start the cluster first
./scripts/start-all.ps1

# Run visual tests
cd tests/visual
npm test

# Update golden snapshots (when intentionally changing UI)
npm run test:update

# Run with UI for debugging
npm run test:ui
```

## Golden Images

Snapshots are stored in `terminal.spec.ts-snapshots/`. These are committed to git and serve as the baseline for comparison.

To update snapshots after intentional UI changes:
```bash
npm run test:update
git add terminal.spec.ts-snapshots/
git commit -m "test: update visual snapshots"
```

## Test Coverage

| Test | What it verifies |
|------|------------------|
| `terminal renders with correct theme` | xterm.js theme colors match TUI |
| `terminal shows TUI dashboard content` | Full dashboard renders correctly |
| `connection status indicator works` | WebSocket status UI |
| `terminal resizes correctly` | Responsive terminal resize |
| `shows fallback when WebSocket unavailable` | Fallback dashboard UI |
| `retry button reconnects` | Reconnection flow works |

## CI Integration

Visual tests run in the `visual-regression` job, triggered by:
- Changes to `src/interfaces/web/`
- Changes to `src/services/web-pty-server/`
- Manual dispatch

⚠️ **Note**: Visual tests require a running cluster. In CI, this uses Docker Compose with pre-built images.
