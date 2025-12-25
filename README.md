# 📡 Distributed Task Observatory

A self-contained, local-first demonstration platform showcasing modern, production-grade distributed systems.

![Architecture](https://img.shields.io/badge/Architecture-Microservices-blue)
![Stack](https://img.shields.io/badge/Stack-Polyglot-green)
![Platform](https://img.shields.io/badge/Platform-Kubernetes-326CE5)

## 🚀 Quick Start

### Option 1: TUI Launcher (Recommended)

The TUI automatically checks prerequisites and guides you through installation:

```bash
cd src/interfaces/tui
cargo run --release
```

**What happens:**
1. ✅ Checks for Docker, PowerShell, kubectl, kind
2. 📋 Shows missing tools with install commands
3. 📎 Press **C** to copy install command to clipboard
4. 🚀 Press **L** to launch the cluster

### Option 2: Script (if prerequisites installed)

```bash
# Windows (PowerShell)
.\scripts\start-all.ps1

# macOS/Linux
pwsh ./scripts/start-all.ps1
```

### Prerequisites

> 💡 **Tip:** The TUI will detect and help you install these!

- **Docker Desktop** - [Install](https://docs.docker.com/desktop/)
- **PowerShell Core** - `brew install powershell` (macOS) / `winget install Microsoft.PowerShell` (Windows)
- **kubectl** - `brew install kubectl` or `winget install Kubernetes.kubectl`
- **kind** - `brew install kind` or `winget install Kubernetes.kind`
- **Rust** - [rustup.rs](https://rustup.rs) (only needed for building TUI from source)

---

## 📦 Installation (Binary Release)

> **Note:** v0.1.x are unsigned bootstrap releases. Your OS may show security prompts.
> See [Verifying Releases](./docs/VERIFYING_RELEASES.md) for checksum verification.

### Quick Install

**Linux/macOS:**
```bash
curl -fsSL https://raw.githubusercontent.com/oddessentials/odd-demonstration/main/install.sh | sh
```

**Windows (PowerShell):**
```powershell
iwr -useb https://raw.githubusercontent.com/oddessentials/odd-demonstration/main/install.ps1 | iex
```

**npm:**
```bash
npm install -g @oddessentials/odd-dashboard
```

### Verify Installation

```bash
odd-dashboard --version
# Shows version, commit, build time, and rustc version

odd-dashboard doctor
# Checks: Docker, PowerShell, kubectl, kind
```

### Supported Platforms

| OS | Architecture | Artifact |
|----|--------------|----------|
| Windows | x64 | `odd-dashboard-windows-x64.exe` |
| macOS | Intel | `odd-dashboard-macos-x64` |
| macOS | Apple Silicon | `odd-dashboard-macos-arm64` |
| Linux | x64 | `odd-dashboard-linux-x64` |
| Linux | ARM64 | `odd-dashboard-linux-arm64` |

**System Requirements:** 8GB RAM minimum (16GB recommended), 4+ CPU cores, 15GB disk.
See [Support Matrix](./docs/SUPPORT_MATRIX.md) for full hardware requirements and Docker Desktop configuration.

## 🏗️ Architecture

![Architecture diagram](./mermaid-diagram.svg)

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Web UI    │     │  Rust TUI   │     │   Gateway   │
│  (Nginx)    │     │  (ratatui)  │     │  (Node.js)  │
└──────┬──────┘     └──────┬──────┘     └──────┬──────┘
       │                   │                   │
       └───────────────────┴─────────┬─────────┘
                                     │
                              ┌──────▼──────┐
                              │ Read Model  │
                              │    (Go)     │
                              └──────┬──────┘
                                     │
       ┌─────────────────────────────┼─────────────────────────────┐
       │                  │          │          │                  │
┌──────▼──────┐   ┌───────▼───────┐  │  ┌───────▼───────┐  ┌───────▼───────┐
│   Redis     │   │   MongoDB     │  │  │  PostgreSQL   │  │   RabbitMQ    │
│  (Cache)    │   │ (Event Store) │  │  │(Authoritative)│  │ (Event Spine) │
└─────────────┘   └───────────────┘  │  └───────────────┘  └───────┬───────┘
                                     │                             │
                              ┌──────┴─────────────────────────────┤
                              │                                    │
                       ┌──────▼──────┐                     ┌───────▼───────┐
                       │  Processor  │                     │ Metrics Engine│
                       │  (Python)   │                     │     (Go)      │
                       └─────────────┘                     └───────────────┘
```

---

## 🔗 Access Points

After startup, access services via port-forwards:

| Service            | URL                         | Credentials               |
| ------------------ | --------------------------- | ------------------------- |
| **Web Dashboard**  | http://localhost:8081       | -                         |
| **Gateway API**    | http://localhost:3000       | -                         |
| ↳ API Docs         | http://localhost:3000/docs  | -                         |
| **Read Model API** | http://localhost:8080/stats | -                         |
| ↳ API Docs         | http://localhost:8080/docs  | -                         |
| **RabbitMQ**       | http://localhost:15672      | guest / guest             |
| **Grafana**        | http://localhost:3002       | admin / admin             |
| **Prometheus**     | http://localhost:9090       | -                         |
| **pgAdmin**        | http://localhost:5050       | admin@example.com / admin |
| **Mongo Express**  | http://localhost:8082       | admin / password123       |
| **RedisInsight**   | http://localhost:8001       | -                         |

---

## 🖥️ Interfaces

### Rust TUI

Terminal dashboard with:

- **Guided Setup** - Automatic prerequisite checking with clipboard copy
- **Cluster Launcher** - One-key cluster startup
- **Real-time Stats** - Jobs, completions, failures
- **Alerts Panel** - Active Prometheus alerts
- **UI Launcher** - Quick access to all web interfaces

**Keyboard:**
| Key | Action |
|-----|--------|
| `L` | Launch cluster (launcher mode) |
| `Q` | Quit |
| `R` | Refresh |
| `N` | New Task |
| `U` | UI Launcher |

### Web Dashboard

Glassmorphic UI with loading animation, stats, alerts, and job tables.

---

## 🧪 Testing

### Run All Tests

```bash
# Windows
.\scripts\run-all-tests.ps1

# macOS/Linux
pwsh ./scripts/run-all-tests.ps1
```

### Integration Gate

```bash
# Windows
.\scripts\integration-gate.ps1

# macOS/Linux
pwsh ./scripts/integration-gate.ps1
```

### Per-Service Tests

| Service        | Command                                         |
| -------------- | ----------------------------------------------- |
| Gateway        | `cd src/services/gateway && npx vitest run`     |
| Processor      | `cd src/services/processor && pytest tests/ -v` |
| Metrics-Engine | `cd src/services/metrics-engine && go test -v`  |
| Read-Model     | `cd src/services/read-model && go test -v`      |
| TUI            | `cd src/interfaces/tui && cargo test`           |

---

## 📁 Project Structure

```
odd-demonstration/
├── .github/             # CI workflows (GitHub Actions)
├── contracts/           # Event-driven contract layer
│   ├── schemas/         # JSON schemas (event-envelope, job, etc.)
│   ├── fixtures/        # Test fixtures for validation
│   └── VERSIONS.md      # Schema version registry
├── docs/                # Additional documentation
├── infra/
│   ├── k8s/             # Kubernetes manifests (services, mongo, redis, etc.)
│   └── grafana/         # Grafana dashboard JSON
├── scripts/             # Automation & CI scripts
│   ├── start-all.ps1    # One-click cluster startup
│   ├── run-all-tests.ps1
│   ├── integration-gate.ps1
│   ├── check-service-versions.py
│   └── check-schema-compat.py
├── src/
│   ├── interfaces/
│   │   ├── tui/         # Rust TUI (ratatui) with cluster launcher
│   │   └── web/         # Glassmorphic web dashboard (Nginx)
│   └── services/
│       ├── gateway/     # Node.js - API ingress, schema validation
│       ├── processor/   # Python - Job execution worker
│       ├── metrics-engine/  # Go - Event aggregation, MongoDB writer
│       └── read-model/      # Go - Query API (Postgres, MongoDB, Redis)
├── tests/               # Integration test fixtures & determinism docs
├── audit/               # Session artifacts & implementation walkthroughs
└── MODULE.bazel         # Bazel workspace (polyglot build)
```

---

## 🛑 Cleanup

```bash
# Stop port-forwards (Windows PowerShell)
Get-Job | Stop-Job | Remove-Job

# Stop port-forwards (macOS/Linux - if running in background)
pkill -f "kubectl port-forward"

# Delete cluster (all platforms)
kind delete cluster --name task-observatory
```

---

## 📚 Documentation

- [Beginner Setup Guide](./README_beginner.md) - Step-by-step with prerequisites
- [Contributing](./CONTRIBUTING.md) - Development guidelines
- [Audit](./audit/) - Implementation details and walkthroughs

---

## 📝 License

MIT
