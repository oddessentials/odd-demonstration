# System Invariants

This document defines the non-negotiable guarantees that the Distributed Task Observatory maintains. These invariants are enforced by CI and must pass on every merge to `main`.

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
| C4 | Breaking changes require major version | `check-schema-compat.py --ci` | ✅ CI (conditional) |
| C5 | Schemas documented in VERSIONS.md | `test-contracts-sanity.py` | ✅ CI |
| X1 | Scripts run on Windows + Linux pwsh | CI `shell: pwsh` on ubuntu | ✅ CI |
| X2 | pwsh 7+ for parallel execution | `run-all-tests.ps1` version check | ✅ Runtime |
| X3 | No hardcoded Windows paths | — | 📝 Documented-Only |
| X4 | No bash-only constructs | — | 📝 Documented-Only |
| V1 | Processor coverage ≥ 80% | `check-coverage.py processor` | ✅ CI |
| V2 | metrics-engine coverage ≥ 10% | `check-coverage.py metrics-engine` | ✅ CI |
| V3 | read-model coverage ≥ 3% | `check-coverage.py read-model` | ✅ CI |
| V4 | TUI coverage ≥ 14% | `check-coverage.py tui` | ✅ CI |
| I1 | Integration gate on contracts change | `dorny/paths-filter` + job | ✅ CI |
| I2 | Integration gate on services change | `dorny/paths-filter` + job | ✅ CI |
| A1 | Hermetic builds | Bazel `--lockfile_mode=error` | ✅ CI |
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

| Service | Min Threshold | Warn Threshold |
|---------|---------------|----------------|
| Processor (Python) | 80% | 85% |
| Metrics Engine (Go) | 10% | 15% |
| Read Model (Go) | 3% | 5% |
| TUI (Rust) | 14% | 20% |

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
| All builds are hermetic and reproducible | Bazel with `MODULE.bazel.lock` |
| No manual intervention required for any environment | Scripts automate cluster setup, port-forwarding |
| Single canonical test entrypoint | `scripts/run-all-tests.ps1` |
