# Distributed Task Observatory Implementation Task List

## Phase 0: Foundation & Contracts [x]
- [x] Initialize Bazel workspace and build rules
- [x] Define canonical event schemas and domain models (JSON Schema/OpenAPI)
- [x] Set up local development environment (kind cluster, kubectl)

## Phase 1: Infrastructure & Platform [x]
- [x] Deploy RabbitMQ with Management UI
- [x] Deploy PostgreSQL with pgAdmin
- [x] Deploy NoSQL (Redis/Mongo) with Management UI
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
- [x] Implement Go Metrics Engine (RabbitMQ consumer, NoSQL persistence)
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
