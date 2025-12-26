# System Invariants

This document defines the non-negotiable guarantees that the Distributed Task Observatory maintains. These invariants are enforced by CI and must pass on every merge to `main`. Items marked 📝 are governance-only and not CI-enforced.

---

## Invariant ↔ Enforcement Map

> [!IMPORTANT]
> Invariants marked **📝 Documented-Only** are manual governance controls (non-enforced).
> Changes to this file require review (add to CODEOWNERS or branch protection path rules).

| ID | Invariant | Automated Check | Status |
|----|-----------|-----------------|--------|
| C1 | Event messages conform to schema | `validate-contracts.ps1`, Gateway AJV | ✅ CI |
| C2 | Job objects conform to schema | `validate-contracts.ps1`, Gateway AJV | ✅ CI |
| C3 | Schemas have `$version` and `$id` | `test-contracts-sanity.py` | ✅ CI |
| C4 | Breaking changes require major version | `check-schema-compat.py --ci` | ✅ CI (conditional: `schemas` + `compat_script` filters) |
| C5 | Schemas documented in VERSIONS.md | `test-contracts-sanity.py` | ✅ CI |
| X1 | Scripts run on Linux pwsh | CI `shell: pwsh` on ubuntu-latest | ✅ CI |
| X2 | pwsh 7+ for parallel execution | `run-all-tests.ps1` version check | ✅ Runtime |
| X3 | No hardcoded Windows paths | — | 📝 Documented-Only |
| X4 | No bash-only constructs | — | 📝 Documented-Only |
| V1 | Processor coverage ≥ 80% | `check-coverage.py processor` | ✅ CI |
| V2 | metrics-engine coverage ≥ 10% | `check-coverage.py metrics-engine` | ✅ CI |
| V2a | metrics-engine/validator coverage ≥ 80% | `check-coverage.py metrics-engine` (subpkg) | ✅ CI |
| V3 | read-model coverage ≥ 18% | `check-coverage.py read-model` | ✅ CI |
| V4 | TUI lib coverage ≥ 31% | `check-coverage.py tui` (tarpaulin --lib --exclude-files) | ✅ CI |
| V5 | Gateway coverage ≥ 80% | `vitest --coverage` | ✅ CI |
| V6 | web-pty-server coverage ≥ 80% | `check-coverage.py web-pty-server` (tarpaulin --lib) | ✅ CI |
| V7 | Visual regression tests pass | `tests/visual/` Playwright snapshots | ✅ CI (web_terminal changes) |
| I1 | Integration gate on contracts change | `dorny/paths-filter` + job | ✅ CI |
| I2 | Integration gate on services change | `dorny/paths-filter` + job | ✅ CI |
| I3 | Integration harness self-contained | Docker Compose only | ⏳ Blocked (needs Docker Hub images) |
| I4 | Integration runtime <180s | `integration-harness.ps1` timeout | ⏳ Blocked (needs Docker Hub images) |
| I5 | Artifact capture every run | Guarded `finally` block | ⏳ Blocked (needs Docker Hub images) |
| I6 | Victory gate: 3 green + nightly | — | 📝 Governance-Only |
| I7 | K8s/Compose service parity | `validate-compose-k8s-parity.ps1` | ✅ CI |
| A1 | Hermetic Bazel builds | Bazel `--lockfile_mode=error` | ✅ CI |
| A2 | No manual intervention | — | 📝 Documented-Only |
| A3 | Single test entrypoint | `run-all-tests.ps1` | ✅ CI |


---

## Contract Invariants

| Invariant | Enforcement |
|-----------|-------------|
| All event messages conform to `contracts/schemas/event-envelope.json` | Gateway validation (AJV), Processor validation (jsonschema) |
| All job objects conform to `contracts/schemas/job.json` | Gateway validation, Read Model responses |
| Every schema has `$version` (SemVer) and `$id` | `test-contracts-sanity.py` |
| Schema breaking changes require major version bump | `check-schema-compat.py --ci` |
| All schemas documented in `contracts/VERSIONS.md` | `test-contracts-sanity.py` |

---

## Cross-Platform Invariants

| Invariant | Enforcement |
|-----------|-------------|
| All PowerShell scripts execute on Linux pwsh | CI runs on `ubuntu-latest` with `shell: pwsh` |
| PowerShell 7+ is required for parallel execution | `run-all-tests.ps1` checks `$PSVersionTable.PSVersion.Major` with sequential fallback |
| No hardcoded Windows paths (e.g., `C:\`) | Code review, cross-platform CI job |
| No bash-only constructs in scripts | PowerShell-only scripts in `scripts/` |

---

## Coverage Invariants

Thresholds are externalized in `coverage-config.json` and enforced by `scripts/check-coverage.py`.

| Service | Min Threshold | Warn Threshold | Notes |
|---------|---------------|----------------|-------|
| Processor (Python) | 80% | 85% | Target achieved |
| Metrics Engine (Go) | 10% | 15% | Infrastructure-heavy main; business logic in validator (80%+) |
| Metrics Engine Validator (Go) | 80% | 85% | Core validation logic |
| Read Model (Go) | 18% | 25% | Infrastructure-heavy; HTTP handlers and middleware tested |
| TUI (Rust lib) | 31% | 32% | Lib-only; `--exclude-files main.rs` needed because `--lib` still measures bin |
| Gateway (TypeScript) | 80% | 85% | Core logic in lib/ modules maintains 100% coverage |
| web-pty-server (Rust) | 80% | 85% | PTY broker; 81% achieved with 35 unit tests |
| Visual Tests (Playwright) | — | — | Screenshot comparison; requires running cluster |

> [!NOTE]
> **Go Service Architecture Tradeoff**: The `metrics-engine` and `read-model` packages are infrastructure-heavy,
> with ~70-80% of code in `main()` handling external connections (RabbitMQ, Redis, MongoDB, PostgreSQL) and
> infinite processing loops. This code cannot be meaningfully unit-tested without either (a) significant
> refactoring for dependency injection, or (b) integration tests against real services.
>
> The **business logic** is isolated in the `metrics-engine/validator` subpackage, which maintains 80%+ coverage.
> The main package thresholds reflect what's achievable with unit tests against testable helper functions,
> struct serialization, HTTP handlers, and middleware.

**Ratchet Policy**: Coverage can only increase. Decreases trigger warnings (not failures) with manual override option.

---

## Integration Invariants

| ID | Invariant | Enforcement |
|----|-----------|-------------|
| I1 | Integration gate runs on `contracts/` changes | `dorny/paths-filter` + `compat_critical` |
| I2 | Integration gate runs on `src/services/` changes | `dorny/paths-filter` + `compat_critical` |
| I3 | Integration harness is self-contained | Docker Compose only (no K8s) |
| I4 | Integration runtime <120s (wall-clock) | `integration-harness.ps1` exits 1 on breach |
| I5 | Artifact capture on every run | Guarded capture in `finally` block |
| I6 | Victory gate: 3 green PRs + 1 nightly | 📝 Governance-only |
| I7 | K8s and Docker Compose have same services | `validate-compose-k8s-parity.ps1` |

> [!NOTE]
> **I6 is intentionally governance-only**: The victory gate requires human judgment for flake detection
> and confidence building. Automated enforcement may be added once baseline stability is proven over
> multiple release cycles.

### Integration Harness Details

The integration harness (`scripts/integration-harness.ps1`) validates 4 canonical proof paths:

| Path | Assertion | Schema |
|------|-----------|--------|
| P1 | Gateway accepts job (201) | `job.json` |
| P2 | Events contain jobId | `event-envelope.json` |
| P3 | Jobs reflect COMPLETED | `job.json` |
| P4 | Metrics counter exposed | Regex |

See [`tests/DETERMINISM.md`](../tests/DETERMINISM.md) and [`docs/TESTING.md`](./TESTING.md) for more details.

---

## Determinism Invariants

See [`tests/DETERMINISM.md`](../tests/DETERMINISM.md) for test timing contracts:

- All tests have explicit max timeouts
- Fixed polling intervals (not adaptive)
- Finite retries with bounded backoff
- Mandatory log capture on failure

---

## Automation Invariants

| Invariant | Enforcement |
|-----------|-------------|
| All Bazel builds are hermetic and reproducible | Bazel with `MODULE.bazel.lock` |
| No manual intervention required for any environment | Scripts automate cluster setup, port-forwarding (governance-only) |
| Single canonical test entrypoint | `scripts/run-all-tests.ps1` |
