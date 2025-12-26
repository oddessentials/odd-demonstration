flowchart TB
subgraph Interfaces
Browser["Browser (xterm.js)"]
WebUI["web-ui-http (nginx)"]
TUI["odd-dashboard TUI (Rust/ratatui)"]
end

    subgraph EdgeServices["Edge & Access"]
        WebPTY["web-pty-ws (Rust)"]
        Gateway["Gateway API (Node.js)"]
        ReadModel["Read Model API (Go)"]
    end

    subgraph CoreServices["Core Services"]
        Processor["Processor (Python)"]
        Metrics["Metrics Engine (Go)"]
    end

    subgraph Data["Data & Messaging"]
        RabbitMQ["RabbitMQ (event spine)"]
        Postgres["PostgreSQL (authoritative)"]
        Mongo["MongoDB (event store)"]
        Redis["Redis (cache)"]
    end

    subgraph Observability
        Prometheus["Prometheus"]
        Grafana["Grafana"]
    end

    subgraph Testing["Test Framework"]
        Unit["Unit Tests\n(vitest / go test / pytest / cargo)"]
        Contracts["Contract Validator\nscripts/validate-contracts.ps1"]
        Smoke["Smoke Test\nscripts/smoke-test.ps1"]
        Integration["Integration Gate/Harness\nscripts/integration-gate.ps1\nscripts/integration-harness.ps1"]
        Visual["Playwright Visual Tests\ntests/visual"]
    end

    Browser --> WebUI
    WebUI -- WebSocket --> WebPTY
    WebPTY --> TUI
    WebUI -- /api --> ReadModel
    TUI --> Gateway
    TUI --> ReadModel

    Gateway --> RabbitMQ
    Processor --> RabbitMQ
    Metrics --> RabbitMQ

    Processor --> Postgres
    ReadModel --> Postgres

    ReadModel --> Mongo
    Metrics --> Mongo

    ReadModel --> Redis
    Metrics --> Redis

    Prometheus --> Metrics
    Grafana --> Prometheus

    Unit -.-> Gateway
    Unit -.-> Processor
    Unit -.-> Metrics
    Unit -.-> ReadModel
    Unit -.-> WebPTY
    Contracts -.-> Gateway
    Contracts -.-> Processor
    Smoke -.-> Gateway
    Smoke -.-> ReadModel
    Integration -.-> Gateway
    Integration -.-> ReadModel
    Integration -.-> Processor
    Integration -.-> Metrics
    Visual -.-> WebUI
