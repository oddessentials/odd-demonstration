## Summary

VERIFY the visual/behavioral test instability is caused by **WebSocket session leakage** leading to **per-IP session exhaustion (5 max)**. The product changes (server mode, web bundle, parity) are not the problem; **test lifecycle cleanup and staging** are.

---

## Best Path Forward

### 1) Fix session leakage at the test lifecycle level (primary fix)

ENABLE adding **suite-level teardown** for any Playwright suite that opens or triggers WebSockets.

VERIFY teardown meets these criteria (implementation-specific, not prescriptive):

- Closes the page’s active WebSocket connection(s) **if present**
- Runs on **every test** (afterEach / fixture teardown)
- Does not fail the test if the socket is already closed / missing
- Ensures the page/context is closed so reconnection loops can’t consume slots

VERIFY by running the suite repeatedly (≥10 runs) with **zero** “max sessions per IP” errors.

### 2) Keep test parity while removing flake (stage separation)

ENABLE enforcing a clean separation of concerns:

- **CI default:** “behavioral web” tests that use real WS + deterministic assertions + teardown
- **Non-default stage (nightly/manual):** screenshot/visual regression tests (still real WS, but isolated)
- **DISABLE in CI:** any tests that depend on Playwright WebSocket interception/mocking

VERIFY the CI job contains **no permanently skipped suites**; anything not stable belongs in the non-default stage with explicit labeling.

### 3) Optional guardrails (only after teardown is correct)

ENABLE (optional) CI-only session-limit increases **only** if:

- teardown is already verified effective, and
- the increase is documented as CI-only and does not change runtime behavior.

DISABLE container restart-per-test as a primary strategy (too slow, masks leaks).

---

## Acceptance Gates

VERIFY all of the following before re-enabling full coverage:

- No WS session limit failures across repeated runs
- Behavioral web tests run in the correct CI stage, no skips
- Visual/screenshot tests run in their own stage (nightly/manual) with clear ownership
- Invariants remain intact (I4 budget, I7 parity); no “special-case” behavior differences between compose/k8s paths
