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
| X1 | Scripts run on Windows + Linux pwsh | CI `shell: pwsh` on ubuntu | ✅ CI |
| X2 | pwsh 7+ for parallel execution | `run-all-tests.ps1` version check | ✅ Runtime |
| X3 | No hardcoded Windows paths | — | 📝 Documented-Only |
| X4 | No bash-only constructs | — | 📝 Documented-Only |
| V1 | Processor coverage ≥ 80% | `check-coverage.py processor` | ✅ CI |
| V2 | metrics-engine coverage ≥ 10% | `check-coverage.py metrics-engine` | ✅ CI |
| V2a | metrics-engine/validator coverage ≥ 80% | `check-coverage.py metrics-engine` (subpkg) | ✅ CI |
| V3 | read-model coverage ≥ 18% | `check-coverage.py read-model` | ✅ CI |
| V4 | TUI coverage ≥ 14% | `check-coverage.py tui` | ✅ CI |
| V5 | Gateway coverage ≥ 80% | `vitest --coverage` | ✅ CI |
| I1 | Integration gate on contracts change | `dorny/paths-filter` + job | ✅ CI |
| I2 | Integration gate on services change | `dorny/paths-filter` + job | ✅ CI |
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
| All PowerShell scripts execute on Windows pwsh AND Linux pwsh | CI runs on `ubuntu-latest` with `shell: pwsh` |
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
| TUI (Rust) | 14% | 20% | |
| Gateway (TypeScript) | 80% | 85% | Core logic in lib/ modules maintains 100% coverage |

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

## Integration Gate Invariants

| Invariant | Enforcement |
|-----------|-------------|
| Integration gate runs when `contracts/` changes | `dorny/paths-filter` + conditional job |
| Integration gate runs when `src/services/` changes | `dorny/paths-filter` + conditional job |
| Integration gate verifies full job lifecycle | `scripts/integration-gate.ps1` |

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
