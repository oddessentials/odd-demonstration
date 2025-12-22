## 🧩 Architecture Blueprint

### 🧱 Build & Runtime

- **Bazel**

  - Builds all services (Node, Python, Go, Rust CLI)
  - Produces OCI images
  - Enforces hermetic, reproducible builds

- **kind**

  - Local Kubernetes cluster
  - Zero cloud dependency
  - Mirrors real cluster topology

- **Kubernetes**

  - Deployments for each microservice
  - StatefulSets for databases
  - Services + Ingress for routing

---

### 🔁 Message Bus (Event Spine)

- **RabbitMQ**

  - All jobs move through queues
  - Each microservice subscribes to specific routing keys
  - Prometheus scrapes RabbitMQ metrics directly

**Example queues**

```
jobs.created
jobs.validated
jobs.executed
jobs.failed
jobs.completed
```

---

### 🧠 Microservices (Polyglot on Purpose)

#### Node.js Microservice

- **API Gateway**
- Accepts job submissions
- Publishes messages to RabbitMQ
- Exposes OpenAPI docs

> Demonstrates async I/O + orchestration glue

---

#### Python Microservice

- **Job Processor**
- Pulls jobs from RabbitMQ
- Performs simulated “work” (delay, failure, retry)
- Writes results to PostgreSQL

> Demonstrates worker patterns + reliability

---

#### Go Microservice

- **Metrics + Aggregation Engine**
- Consumes job lifecycle events
- Writes time-series summaries to NoSQL
- Exposes `/metrics` endpoint for Prometheus

> Demonstrates high-performance services + metrics-first design

---

### 🗄️ Data Layer

#### PostgreSQL (Relational)

- Job metadata
- Status history
- Foreign-keyed lifecycle data
- **UI**: `pgAdmin`

#### NoSQL (Choose One)

- Redis → job state cache

- MongoDB → event documents

- **UI**: RedisInsight or Mongo Express

---

### 📊 Observability (The Star of the Show)

#### **Prometheus**

- Scrapes:

  - All microservices
  - RabbitMQ
  - Kubernetes nodes/pods

- Custom app metrics:

  - Job duration
  - Failure rate
  - Queue depth

---

#### **Alertmanager**

- Fires alerts like:

  - “Job failure rate > 5%”
  - “Queue backlog growing”
  - “Service down”

Alerts are visible in:

- CLI
- Web UI
- Grafana annotations

---

#### **Grafana**

- Dashboards:

  - System overview
  - Per-service health
  - Queue health
  - Job SLA timelines

- Embedded into Web UI via iframe or link

---

### 🖥️ Interface Layer

#### Rust CLI (Primary UI)

- Built with `ratatui`
- Real-time dashboards:

  - Job list
  - Alerts
  - Metrics summaries

- Talks to backend APIs + Prometheus

> This is your **“wow” factor**

---

#### Web Mirror UI

- Read-only mirror of CLI state
- Uses same APIs
- Confirms single source of truth

---

### 📘 Documentation & Management (Hard Requirements Met)

| Component  | Solution                     |
| ---------- | ---------------------------- |
| APIs       | OpenAPI / Swagger            |
| PostgreSQL | pgAdmin                      |
| NoSQL      | Mongo Express / RedisInsight |
| RabbitMQ   | RabbitMQ Management UI       |
| Prometheus | Built-in UI                  |
| Grafana    | Dashboards + alert UI        |

---
