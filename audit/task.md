# Distributed Task Observatory Implementation Task List

## Phase 0: Foundation & Contracts [x]
- [x] Initialize Bazel workspace and build rules
- [x] Define canonical event schemas and domain models (JSON Schema/OpenAPI)
- [x] Set up local development environment (kind cluster, kubectl)

## Phase 1: Infrastructure & Platform [x]
- [x] Deploy RabbitMQ with Management UI
- [x] Deploy PostgreSQL with pgAdmin
- [x] Deploy NoSQL (Redis/MongoDB) with Management UI
- [x] Configure Kubernetes Ingress and Service routing

## Phase 2: Core Service Implementation [x]
- [x] Implement Node.js API Gateway (Job submission, RabbitMQ producer)
- [x] Implement Python Job Processor (RabbitMQ consumer, PostgreSQL persistence)
- [x] Verify core job lifecycle (Submit -> Queue -> Process -> Store)

## Phase 3: Observability Stack [x]
- [x] Deploy Prometheus and Alertmanager
- [x] Provision Grafana with initial dashboards
- [x] Instrument Node.js and Python services with Prometheus metrics
- [x] Configure standard alerts (Service down, Failure rate)

## Phase 4: Aggregation & Read Model [x]
- [x] Implement Go Metrics Engine (RabbitMQ consumer, Redis/MongoDB persistence)
- [x] Implement Read Model API (Single source for UIs)
- [x] Verify metrics aggregation and read model consistency

## Phase 5: Interface Layer [x]
- [x] Implement Rust TUI (ratatui) for real-time monitoring
- [x] Implement Web Mirror UI
- [x] Connect UIs to Read Model API and Prometheus

## Phase 6: Hardening & Verification [x]
- [x] Execute Integration Gate (End-to-end proof)
- [x] Validate contract enforcement in CI
- [x] Final documentation and walkthrough

## Phase 7: Testing & Determinism [x]
- [x] Add unit tests to all services
- [x] Implement schema-compliant mock data
- [x] Enforce deterministic test fixtures
- [x] Add Bazel build rules for Go services

## Phase 8: Production-Grade Observability [x]
- [x] Add MongoDB for event sourcing audit trail
- [x] Create Grafana dashboard with 6 panels
- [x] Configure Prometheus scrape targets
- [x] Add CORS support to Read Model API

## Phase 9: Message Filtering & Event Sourcing [x]
- [x] Implement job type filtering
- [x] Add MongoDB event persistence
- [x] Update Read Model to query MongoDB
- [x] Add /events endpoint to Read Model API

## Phase 10: Startup Automation [x]
- [x] Create `scripts/start-all.ps1` one-click startup
- [x] Add TUI cluster launcher mode
- [x] Add cluster status detection
- [x] Implement setup progress view in TUI
- [x] Create Web launcher.html for offline bootstrap

## Phase 11: Version Governance [x  ]
- [x] Add VERSION files to all services
- [x] Create check-service-versions.py script
- [x] Implement schema compatibility checking
- [x] Document versioning governance in README

## Phase 12: Consumer Validation & TUI Enhancements [x]
- [x] Implement schema validator for Python processor
- [x] Add dead-letter queue for invalid messages
- [x] Add TUI loading splash with animation
- [x] Add TUI alerts panel
- [x] Add TUI task creation placeholder
- [x] Update Web Mirror for feature parity
- [x] Add unit tests for TUI components
