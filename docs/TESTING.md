# Testing Documentation

This document describes the testing harness, its limitations, and assumptions.

## Contract Validator Limitations

The PowerShell-based contract validator (`scripts/validate-contracts.ps1`) provides lightweight schema validation with the following **supported features**:

| Feature | Supported |
|---------|-----------|
| `required` fields | ✅ Yes |
| `type` checking | ✅ Yes (string, integer, number, boolean, object, array) |
| `enum` validation | ✅ Yes |
| Nested `required` (one level) | ✅ Yes |

### Unsupported JSON Schema Features

The following JSON Schema features are **NOT validated** by the PowerShell validator:

- `$ref` (schema references)
- `oneOf`, `anyOf`, `allOf` (composition)
- `pattern` (regex constraints)
- `format` validation (e.g., `uuid`, `date-time` — parsed but not validated)
- `additionalProperties: false` (extra fields are allowed by default)
- `minLength`, `maxLength`, `minimum`, `maximum`
- `minItems`, `maxItems`

> [!WARNING]
> If schemas evolve to use advanced features, consider migrating to a full JSON Schema validator (e.g., Python `jsonschema` or Node.js `ajv`).

## Fixture Coverage

| Category | Count | Description |
|----------|-------|-------------|
| Golden (pass) | 4 | Valid schemas, optional fields, extra fields |
| Negative (fail) | 7 | Missing required, invalid enum, wrong type, nested required |

### Edge Cases Covered

- [x] Extra fields present (should pass)
- [x] All optional fields present
- [x] Missing top-level required field
- [x] Invalid enum value
- [x] Wrong type for field
- [x] Missing nested required field

### Edge Cases NOT Covered (Future Work)

- [ ] Null values for optional fields
- [ ] Boundary values (very long strings)
- [ ] Backward compatibility (v1.0.0 → v1.1.0 migration)

## Smoke Test Assumptions

The smoke test (`scripts/smoke-test.ps1`) assumes:

1. **kubectl context** is `kind-task-observatory`
2. **Pods are Ready** (verified via `kubectl wait`)
3. **Port-forwards are active** for Gateway (3000) and Read Model (8080)

### Failure Modes

| Condition | Behavior |
|-----------|----------|
| Wrong context | **Fails** with error message |
| Pods not Ready | **Fails** after 10s timeout |
| Port-forward missing | **Fails** on health check |
| RabbitMQ unreachable | **Skips** (optional check) |

## MongoDB Event Ordering Semantics

The integration gate checks for event correlation and deduplication but **does NOT enforce strict ordering** because:

1. MongoDB insertion order may differ from event occurrence order under load.
2. The system does not provide a formal ordering guarantee (no sequence numbers).
3. `occurredAt` timestamps are used for display order, not consistency.

### What IS Guaranteed

- Each event has a unique `eventId`
- Events contain the originating `jobId` in the payload
- No duplicate `eventId` values are returned

### What is NOT Guaranteed

- Strict chronological ordering across concurrent job submissions
- Exactly-once delivery (idempotency is application-level)
