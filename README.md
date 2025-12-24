# 📡 Distributed Task Observatory

A self-contained, local-first demonstration platform showcasing modern, production-grade distributed systems.

![Architecture](https://img.shields.io/badge/Architecture-Microservices-blue)
![Stack](https://img.shields.io/badge/Stack-Polyglot-green)
![Platform](https://img.shields.io/badge/Platform-Kubernetes-326CE5)

> **Quick Start:** Clone → Install Rust → Run TUI → Press `L`
> ```powershell
> cd src/interfaces/tui && cargo run --release
> ```

---

## 🚀 Quick Start

### Prerequisites
- **Docker Desktop** (running)
- **Rust** (for TUI) - [Install](https://rustup.rs)
- **kubectl** and **kind** (auto-installed if missing via Chocolatey)

### Option 1: TUI Launcher (Recommended)
```powershell
cd src/interfaces/tui
cargo run --release
# Press 'L' when prompted to launch the cluster
```

### Option 2: Script
```powershell
.\scripts\start-all.ps1
```

### Option 3: Manual Setup
See [README_beginner.md](./README_beginner.md) for step-by-step instructions.

---

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
              ┌──────────────────────┼──────────────────────┐
              │                      │                      │
       ┌──────▼──────┐       ┌───────▼───────┐      ┌───────▼───────┐
       │   Redis     │       │  PostgreSQL   │      │   RabbitMQ    │
       │  (Cache)    │       │ (Authoritative)│      │ (Event Spine) │
       └─────────────┘       └───────────────┘      └───────┬───────┘
                                                           │
                              ┌─────────────────────────────┤
                              │                             │
                       ┌──────▼──────┐              ┌───────▼───────┐
                       │  Processor  │              │ Metrics Engine│
                       │  (Python)   │              │     (Go)      │
                       └─────────────┘              └───────────────┘
```

---

## 🔗 Access Points

After startup, access services via port-forwards:

| Service | URL | Credentials |
|---------|-----|-------------|
| **Web Dashboard** | http://localhost:8081 | - |
| **Gateway API** | http://localhost:3000 | - |
| **Read Model API** | http://localhost:8080/stats | - |
| **RabbitMQ** | http://localhost:15672 | guest / guest |
| **Grafana** | http://localhost:3002 | admin / admin |
| **Prometheus** | http://localhost:9090 | - |

---

## 🖥️ Interfaces

### Rust TUI
Terminal dashboard with:
- **Cluster launcher** - One-key cluster startup
- **Real-time stats** - Jobs, completions, failures
- **Alerts panel** - Active Prometheus alerts
- **Jobs table** - Recent job status

**Keyboard:**
| Key | Action |
|-----|--------|
| `L` | Launch cluster (launcher mode) |
| `Q` | Quit |
| `R` | Refresh |
| `N` | New Task (placeholder) |

### Web Dashboard
Glassmorphic UI with loading animation, stats, alerts, and job tables.

---

## 🧪 Testing

### Run All Tests
```powershell
.\scripts\run-all-tests.ps1
```

### Integration Gate
```powershell
.\scripts\integration-gate.ps1
```

### Per-Service Tests
| Service | Command |
|---------|---------|
| Gateway | `cd src/services/gateway && npx vitest run` |
| Processor | `cd src/services/processor && pytest tests/ -v` |
| Metrics-Engine | `cd src/services/metrics-engine && go test -v` |
| Read-Model | `cd src/services/read-model && go test -v` |
| TUI | `cd src/interfaces/tui && cargo test` |

---

## 📁 Project Structure

```
odd-demonstration/
├── audit/               # Session documentation
├── contracts/           # JSON schemas and versioning
├── infra/
│   ├── k8s/            # Kubernetes manifests
│   └── grafana/        # Grafana dashboards
├── scripts/             # Automation scripts
│   ├── start-all.ps1   # One-click startup
│   └── integration-gate.ps1
└── src/
    ├── interfaces/
    │   ├── tui/        # Rust TUI with launcher
    │   └── web/        # Web dashboard
    └── services/
        ├── gateway/    # Node.js API
        ├── processor/  # Python worker
        ├── metrics-engine/  # Go aggregator
        └── read-model/      # Go API
```

---

## 🛑 Cleanup

```powershell
# Stop port-forwards
Get-Job | Stop-Job | Remove-Job

# Delete cluster
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
