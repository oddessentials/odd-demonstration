# Changelog

All notable changes to the Distributed Task Observatory will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Version file reading in Go services (metrics-engine, read-model)
- Version bump script (`scripts/bump-version.ps1`)
- Versioned Docker image builds with :latest alias for local dev
- This CHANGELOG file

### Changed
- Services now log their version at startup
- Go services fail-fast if VERSION file is missing or invalid
- Health endpoints now return service version

## [0.1.0] - 2024-12-23

### Added
- Initial release of Distributed Task Observatory
- Gateway service (Node.js/TypeScript) - REST API and RabbitMQ producer
- Processor service (Python) - Job processor with PostgreSQL storage
- Metrics Engine service (Go) - Event consumer with MongoDB storage
- Read Model service (Go) - Query API with Redis caching
- TUI interface (Rust) - Terminal dashboard with cluster launcher
- Web interface - Browser-based dashboard
- Contract schemas with JSON Schema validation
- Kubernetes deployment manifests
- Kind cluster setup automation
- Prometheus metrics collection
- Grafana dashboards
- GitHub Actions CI pipeline
- Bazel build system

### Contract Versions
- event-envelope.json: 1.0.0
- job.json: 1.0.0

[Unreleased]: https://github.com/oddessentials/odd-demonstration/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/oddessentials/odd-demonstration/releases/tag/v0.1.0
