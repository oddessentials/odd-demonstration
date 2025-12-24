# Walkthrough - Distributed Task Observatory

## Overview

The Distributed Task Observatory is a complete, production-grade distributed task processing system demonstrating modern microservice architecture, event-driven design, and observability. This walkthrough documents the implementation across all 12 phases.

---

## 🛠️ Environment & Foundation (Phases 0-1)

### Tools & Cluster
- **Kubernetes:** kind cluster named `task-observatory` with Ingress support
- **Build System:** Docker-native with Bazel support for Go/Rust
- **Contract Authority:** JSON Schemas in `contracts/schemas/`

### Infrastructure Components
| Component | Purpose | Port |
|-----------|---------|------|
| RabbitMQ | Message bus (event spine) | 5672, 15672 |
| PostgreSQL | Authoritative relational state | 5432 |
| Redis | Fast-access aggregated metrics | 6379 |
| MongoDB | Event sourcing audit trail | 27017 |

---

## 🚀 Core Services (Phase 2)

### Node.js Gateway
- Handles job validation and submission
- Publishes to RabbitMQ `jobs.created` queue
- Exposes `/jobs`, `/healthz`, `/metrics` endpoints

### Python Processor
- Consumes jobs from RabbitMQ
- Validates against JSON schema
- Updates PostgreSQL state
- Publishes to `jobs.completed` queue
- Dead-letter queue for invalid messages

---

## 📊 Observability Stack (Phase 3)

### Prometheus Metrics
**Gateway:**
- `gateway_jobs_submitted_total{type}` - Jobs submitted by type
- `gateway_jobs_accepted_total` - Jobs published to queue

**Processor:**
- `processor_jobs_processed_total` - Total jobs received
- `processor_jobs_completed_total` - Successfully completed
- `processor_jobs_failed_total` - Failed jobs
- `processor_job_processing_seconds` - Processing time histogram

### Grafana Dashboard
Six panels showing job throughput, latency percentiles, and failure rates.

---

## 🔄 Aggregation Layer (Phase 4)

### Go Metrics Engine
- Consumes `jobs.completed` from RabbitMQ
- Updates Redis counters
- Persists raw events to MongoDB

### Go Read Model API
- Single source of truth for UIs
- Endpoints: `/health`, `/stats`, `/jobs/recent`, `/events`
- CORS support for browser-based UIs

---

## 🖥️ Interface Layer (Phases 5, 12)

### Rust TUI (`src/interfaces/tui`)

The TUI now includes multiple operational modes:

#### Launcher Mode
Detects if the Kind cluster is running:
- Shows ASCII logo with "Cluster not detected"
- **Press `L`** to launch cluster automatically
- Runs `scripts/start-all.ps1` with progress tracking

#### Loading Splash
Animated loading screen while fetching data:
- Braille spinner animation (10 frames)
- Cycling status messages
- oddessentials.com branding

#### Dashboard Mode
Real-time monitoring view:
- ASCII logo on left, stats on right
- Alerts panel with graceful degradation
- Jobs table with color-coded status
- **Keyboard shortcuts:**
  - `Q` - Quit
  - `R` - Refresh
  - `N` - New Task (placeholder modal)

#### Unit Tests
Deterministic tests for:
- Logo integrity and line count
- Spinner frame validation (Braille characters)
- Message cycling logic
- Setup progress state

### Web Dashboard (`src/interfaces/web`)

Modern glassmorphic UI with feature parity:

#### Loading Splash
- Animated ASCII logo
- Spinner and cycling messages
- Auto-hides after first successful fetch

#### Main Dashboard
- Stats cards (Total, Completed, Failed, Alerts)
- Recent jobs table
- Active alerts panel
- Raw events table (MongoDB)

#### New Task Modal
- Placeholder for future task creation
- Click "➕ New Task" button in header

#### Launcher Page (`launcher.html`)
- Opens directly from filesystem (file://)
- Detects cluster status via API polling
- Shows manual startup instructions
- Auto-redirects when cluster is ready

---

## ⚙️ Startup Automation (Phase 10)

### One-Click Script (`scripts/start-all.ps1`)

Comprehensive automation that:
1. Verifies prerequisites (Docker, kind, kubectl)
2. Creates/verifies Kind cluster
3. Builds all Docker images in parallel
4. Loads images into Kind
5. Applies Kubernetes manifests
6. Waits for pod readiness
7. Starts background port-forwards
8. Verifies connectivity
9. Prints access URLs

**Usage:**
```powershell
# Standard run
.\scripts\start-all.ps1

# For TUI integration (JSON output)
.\scripts\start-all.ps1 -OutputJson

# Skip image builds
.\scripts\start-all.ps1 -SkipBuild
```

### TUI-Integrated Launch
The TUI can launch the cluster automatically:
1. Run `cargo run --release` in `src/interfaces/tui`
2. If cluster not detected, launcher mode appears
3. Press `L` to start setup
4. Watch progress with animated status
5. Dashboard loads when ready

---

## ✅ Verification (Phase 6-7)

### Integration Gate v2
Deterministic end-to-end tests:
- Pre-flight checks (context, pod readiness)
- Health checks with retry
- Job submission and processing
- MongoDB event persistence
- Stats aggregation
- Metrics exposure

**Run:** `.\scripts\integration-gate.ps1`

### Unit Tests per Service
| Service | Framework | Command |
|---------|-----------|---------|
| Gateway | Vitest | `cd src/services/gateway && npx vitest run` |
| Processor | pytest | `cd src/services/processor && pytest tests/ -v` |
| Metrics-Engine | Go test | `cd src/services/metrics-engine && go test -v` |
| Read-Model | Go test | `cd src/services/read-model && go test -v` |
| TUI | Rust test | `cd src/interfaces/tui && cargo test` |

---

## 📦 Version Governance (Phase 11)

### Service Versions
Each microservice has a `VERSION` file containing SemVer string.

### Verification
```powershell
python scripts/check-service-versions.py
```
Ensures K8s manifests match service VERSION files.

### Schema Compatibility
```powershell
python scripts/check-schema-compat.py
```
Validates schema `$id` and `$version` fields.

---

## 🔒 Consumer Validation (Phase 12)

### Schema Validation
Python processor validates incoming jobs against JSON schema:
- Rejects malformed messages
- Publishes to dead-letter queue on failure

### Dead-Letter Queue
Invalid messages routed to `jobs.dead-letter` for debugging.

---

## 🎉 Project Complete

The Distributed Task Observatory demonstrates:
- **Polyglot microservices** (Node.js, Python, Go, Rust)
- **Event-driven architecture** with RabbitMQ
- **Multi-database strategy** (PostgreSQL, Redis, MongoDB)
- **Production observability** (Prometheus, Grafana, Alertmanager)
- **One-click deployment** via TUI launcher
- **Comprehensive testing** at unit, contract, and integration levels

> [!IMPORTANT]
> All 12 phases complete. The system is production-ready for demonstration purposes.
