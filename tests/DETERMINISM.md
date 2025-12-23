# Test Determinism Contracts

This document defines the explicit contracts for integration and E2E test suites to ensure deterministic, reproducible CI behavior.

## Core Principles

1. **Bounded Execution**: All tests have explicit max timeouts
2. **Predictable Polling**: Fixed intervals, not adaptive
3. **Retry Limits**: Finite retries with exponential backoff
4. **Failure Diagnostics**: Mandatory log/state capture on failure

---

## Integration Test Contracts

| Suite | Max Timeout | Poll Interval | Max Retries | Failure Diagnostics |
|-------|------------|---------------|-------------|---------------------|
| `smoke-test.ps1` | 60s | 2s | 10 | Pod logs, RabbitMQ queue depth |
| `integration-gate.ps1` | 120s | 5s | 3 | MongoDB events, Prometheus `/metrics` |
| Full E2E lifecycle | 180s | 5s | 5 | All service logs, `/stats` snapshot |

---

## Enforcement

Determinism constants are **hardcoded** in `scripts/run-all-tests.ps1`:

```powershell
$TIMEOUT_UNIT_TESTS = 120      # seconds
$TIMEOUT_CONTRACT = 60         # seconds
$TIMEOUT_INTEGRATION = 180     # seconds
$POLL_INTERVAL_INTEGRATION = 5 # seconds
$MAX_RETRIES_INTEGRATION = 3
```

These values are NOT configurable via environment variables to prevent CI drift.

---

## Failure Diagnostics Emitted

On any integration test failure, the following is captured:

1. **Pod Logs**: Last 20 lines from gateway, processor, read-model, metrics-engine
2. **Queue Depth**: RabbitMQ queue message counts via Management API
3. **Stats Snapshot**: Current `/stats` response from Read Model

This data is printed to stdout for CI artifact collection.

---

## Graceful Degradation (V3)

When prerequisites are unavailable (e.g., no K8s cluster), integration tests:

- Exit with code `0` (success)
- Print `[SKIP] Integration tests - K8s cluster not available`
- Do NOT fail the CI pipeline

This preserves local dev ergonomics without weakening CI guarantees (CI always runs with cluster).
