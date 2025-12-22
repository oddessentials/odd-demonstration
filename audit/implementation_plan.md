# Implementation Plan - Phase 6: Hardening & Verification

This final phase validates the entire system end-to-end and produces final documentation.

## Proposed Changes

### [Integration Gate Script]

#### [NEW] [scripts/integration-gate.ps1](file:///e:/projects/odd-demonstration/scripts/integration-gate.ps1)
- End-to-end proof script that:
  1. Submits multiple jobs via Gateway
  2. Waits for processing
  3. Verifies all jobs appear in PostgreSQL with COMPLETED status
  4. Checks Redis counters match
  5. Validates Read Model API returns correct data
  6. Outputs PASS/FAIL summary

---

### [Contract Validation]

#### [NEW] [scripts/validate-contracts.ps1](file:///e:/projects/odd-demonstration/scripts/validate-contracts.ps1)
- Validates that all services conform to contracts:
  - Gateway produces valid event envelopes
  - Processor consumes and updates correctly
  - Read Model returns expected schema

---

### [Final Documentation]

#### [MODIFY] [README.md](file:///e:/projects/odd-demonstration/README.md)
- Project overview and architecture diagram
- Quick start guide
- Service inventory
- Management UI links

## Verification Plan

### Automated Tests
- Run `scripts/integration-gate.ps1` and verify all checks pass.
- Run `scripts/validate-contracts.ps1` for schema compliance.

### Manual Verification
- Submit jobs while watching Web UI update in real-time.
- Access all management UIs and confirm they are operational.
