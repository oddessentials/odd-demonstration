# Distributed Task Observatory — Contract Authority & Team Directives

## 1. Contract Authority Model (Non-Negotiable)

This repository is **contract-first**.

- The `/contracts` directory is the **single source of truth**.
- Implementations **must conform** to contracts; implementations do not redefine them.
- Teams may negotiate **how** contracts are satisfied, not **whether** they are satisfied.
- Any breaking contract change requires:

  - New major version
  - Migration note
  - Explicit approval from the Contract Authority team

---

## 2. Required Contract Surfaces

The following contracts **MUST exist** and **MUST be implemented** by all teams.

### 2.1 Eventing (RabbitMQ)

**All messages MUST conform to the canonical event envelope.**

Required fields:

- `contractVersion`
- `eventType`
- `eventId`
- `occurredAt`
- `producer { service, instanceId, version }`
- `correlationId`
- `idempotencyKey`
- `payload`

Rules:

- Producers MUST be idempotent.
- Consumers MUST tolerate duplicate delivery.
- All job lifecycle transitions MUST emit events.

---

### 2.2 Domain Model

The following domain objects are canonical and shared:

- `Job`
- `JobAttempt`
- `JobStatus`
- `Alert`

Rules:

- Field names and semantics MUST match contracts.
- Domain objects MUST NOT be redefined per service.

---

### 2.3 HTTP API (Service APIs)

All HTTP APIs MUST:

- Provide OpenAPI documentation
- Expose `/healthz`, `/readyz`, and `/metrics`
- Accept and return `X-Correlation-Id`
- Use a consistent error format

Minimum required APIs:

- Job submission and status
- Read-only observatory API for UIs

---

### 2.4 Read Model API (UI Boundary)

There is **one** read model surface for **all UIs** (Rust CLI and Web mirror).

UIs:

- MUST NOT call internal microservice APIs directly
- MUST rely exclusively on the read model API

The read model MUST expose:

- System overview
- Recent jobs
- Active alerts
- Curated metrics snapshot

---

### 2.5 Observability (Prometheus)

All services MUST expose Prometheus metrics.

Required metric families:

- Job counts (submitted, completed, failed)
- Job duration histogram
- Queue depth
- Consumer lag

Rules:

- Metrics MUST include `service` label
- Job IDs MUST NOT appear as labels
- Cardinality MUST be bounded

---

### 2.6 Alerting (Alertmanager)

The following alerts MUST exist:

- Service down
- Job failure rate above threshold
- Queue backlog growth
- Database unavailable

Rules:

- Alerts MUST include severity labels
- Alerts MUST include a runbook reference
- Alerts MUST be testable with promtool

---

### 2.7 Persistence Boundaries

Storage responsibilities are fixed:

- **PostgreSQL**

  - Authoritative job state
  - Attempt history

- **NoSQL**

  - Derived data
  - Aggregates
  - Read-model projections

Services MUST NOT blur these boundaries.

---

### 2.8 Platform Requirements

All deployed components MUST include:

- API documentation UI (OpenAPI/Swagger)
- Database management UI

  - PostgreSQL → pgAdmin
  - NoSQL → appropriate admin UI

- RabbitMQ management UI
- Grafana dashboards provisioned from repo
- Prometheus and Alertmanager configuration from repo

---

## 3. Team Ownership Boundaries

### Contract Authority Team

- Owns `/contracts/**`
- Owns schema versioning and compatibility rules
- Owns CI enforcement of contracts

### Platform Team

- Kubernetes (kind) configuration
- Observability stack
- Database and broker deployments
- UI exposure and ingress

### Service Teams

- Node.js gateway
- Python worker
- Go aggregator/read-model
- Must conform to contracts exactly

### Interface Team

- Rust TUI
- Web mirror
- Read-only access via read model only

---

## 4. Integration Gate (Mandatory)

The system MUST pass an integration proof:

1. Submit a job
2. Observe lifecycle events
3. Metrics appear in Prometheus
4. Dashboards render in Grafana
5. Alert fires on injected failure
6. UI reflects state consistently

Failure to pass this gate blocks merges.

---

## 5. Enforcement

- Contract violations are CI failures
- Undocumented APIs are invalid
- Missing metrics or alerts are invalid
- UIs bypassing the read model are invalid

---

## 6. Contract Versioning (Phase 11)

All schemas MUST include:
- `$id` — stable, unique identifier (e.g., `contracts/schemas/job.json`)
- `$version` — SemVer format (e.g., `1.0.0`)

### Version Bump Rules

| Change Type | Version Bump | Examples |
|-------------|--------------|----------|
| **MAJOR** | Breaking | Remove field, narrow enum, change type, add required field |
| **MINOR** | Additive | Add optional field, widen enum |
| **PATCH** | Non-behavioral | Docs, metadata, clarifications |

### CI Enforcement

- `check-schema-compat.py` validates breaking changes
- Breaking changes without major version bump → CI failure
- All schemas must appear in `contracts/VERSIONS.md`

---

**This document is authoritative.**
