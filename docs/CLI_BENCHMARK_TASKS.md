# CLI Agent Benchmark Tasks

Deterministic benchmark tasks for evaluating AI-backed CLI tools against the Distributed Task Observatory polyglot codebase.

---

## How This Benchmark Works

Each task is given to the AI CLI tool as a prompt. The tool works autonomously. After completion, a human or automated script grades the output against the rubric defined for that task. Every grading criterion is binary (pass/fail) unless otherwise noted. The final score for each task is the percentage of rubric items passed.

**Environment**: The tool is given a clean checkout of the `main` branch with all build tooling installed (Node 20, Python 3.11, Go 1.21, Rust 1.83, pnpm, Bazel).

**Time Limit**: Each task has a wall-clock limit. The tool may use any strategy within that limit.

**Isolation**: Each task runs in a fresh clone. No state carries over between tasks.

---

## Task 1: Cross-Service Schema Evolution

**Category**: Multi-language change propagation
**Difficulty**: Hard
**Time Limit**: 30 minutes

### Prompt Given to Agent

> Add a new optional field `retryCount` (integer, minimum 0) to the `event-envelope.json` schema. Then update every service that produces or consumes event envelopes — Gateway (TypeScript), Processor (Python), Metrics Engine (Go), and Read Model (Go) — so they correctly handle the new field. The Gateway should set `retryCount: 0` when creating new events. The Processor should increment `retryCount` when it re-publishes a completion event. Existing tests must continue to pass, and add at least one new test per service that exercises the new field. Do not bump the major schema version (this is a backward-compatible addition).

### Grading Rubric (12 points)

| # | Criterion | How to Verify | Pass/Fail |
|---|-----------|---------------|-----------|
| 1 | `contracts/schemas/event-envelope.json` contains `retryCount` as an optional integer with `minimum: 0` | `python -c "import json; s=json.load(open('contracts/schemas/event-envelope.json')); p=s['properties']['retryCount']; assert p['type']=='integer' and p.get('minimum',1)==0 and 'retryCount' not in s.get('required',[])"` | |
| 2 | Schema `$version` bumped to `1.1.0` (minor bump, not major) | `python -c "import json; s=json.load(open('contracts/schemas/event-envelope.json')); assert s['$version']=='1.1.0'"` | |
| 3 | `contracts/VERSIONS.md` updated to document `1.1.0` | `grep -q '1.1.0' contracts/VERSIONS.md` | |
| 4 | Gateway (TS) sets `retryCount: 0` in event envelope builder | `grep -q 'retryCount' src/services/gateway/lib/builders.ts` | |
| 5 | Gateway has a new test for `retryCount` | `grep -rq 'retryCount' src/services/gateway/__tests__/` | |
| 6 | Processor (Python) increments `retryCount` on completion event | `grep -q 'retryCount' src/services/processor/main.py` | |
| 7 | Processor has a new test for `retryCount` | `grep -rq 'retryCount' src/services/processor/tests/` | |
| 8 | Metrics Engine (Go) `EventEnvelope` struct has `RetryCount` field | `grep -q 'RetryCount' src/services/metrics-engine/main.go` | |
| 9 | Metrics Engine has a new test for `RetryCount` | `grep -rq 'RetryCount\|retryCount' src/services/metrics-engine/*_test.go` | |
| 10 | Gateway tests pass | `cd src/services/gateway && npx vitest run` exits 0 | |
| 11 | Processor tests pass | `cd src/services/processor && python -m pytest tests/` exits 0 | |
| 12 | Contract validation passes | `python -c "import json,jsonschema; s=json.load(open('contracts/schemas/event-envelope.json')); jsonschema.Draft7Validator.check_schema(s)"` exits 0 | |

---

## Task 2: Find and Fix Real Bugs

**Category**: Bug detection and repair
**Difficulty**: Medium
**Time Limit**: 20 minutes

### Prompt Given to Agent

> Audit the codebase for bugs. Focus on the Processor service (`src/services/processor/main.py`) and the Read Model service (`src/services/read-model/main.go`). Find at least 3 actual bugs or correctness issues, explain each one clearly, and fix them. Run tests after your fixes to confirm nothing is broken.

### Known Bugs for Grading

These bugs exist in the codebase. The agent gets credit for each one it finds and fixes:

| # | Bug | Location | Description | How to Verify Fix |
|---|-----|----------|-------------|-------------------|
| 1 | `'NOW()'` string literal in SQL | `processor/main.py:100` | `updated_at = 'NOW()'` passes the literal string `'NOW()'` to PostgreSQL instead of calling the `NOW()` SQL function. Should be `updated_at = NOW()` (unquoted in SQL) or use a parameterized timestamp. | `! grep -n "= 'NOW()'" src/services/processor/main.py` returns no matches |
| 2 | State name mismatch | `processor/main.py:90` vs `processor/tests/test_main.py:105` | The processor uses status `'EXECUTING'` but the test state machine defines `'PROCESSING'` as the valid intermediate state. These are inconsistent with each other. | Agent identifies and explains the discrepancy (either the test or the code is authoritative) |
| 3 | Health endpoint path inconsistency | `read-model/main.go:301` | Read Model registers `/health` but K8s gateway manifest and the Gateway service use `/healthz`. The read-model K8s manifest (`infra/k8s/read-model.yaml`) may probe a path that doesn't match. | Agent identifies the `/health` vs `/healthz` inconsistency |
| 4 | Connection leak on error | `processor/main.py:83-103` | If the SQL UPDATE at line 100 fails, the `conn`/`cur` objects are never closed because there is no `finally` block. The `except Exception` at line 124 catches it but doesn't clean up the DB connection. | Agent identifies or fixes the missing cleanup |
| 5 | Missing `ServiceVersion` in DLQ message | `metrics-engine/main.go:167-173` | The `DLQMessage` struct lacks a `ServiceVersion` field, even though the Processor includes `service_version` in its DLQ messages. This makes DLQ messages from the two services structurally inconsistent. | Agent identifies the missing field |

### Grading Rubric (10 points)

| # | Criterion | Points |
|---|-----------|--------|
| 1 | Identified bug #1 (`'NOW()'` string literal) | 2 |
| 2 | Fixed bug #1 correctly | 1 |
| 3 | Identified bug #2 (state name mismatch) or bug #3 (health path) | 2 |
| 4 | Fixed the identified bug correctly | 1 |
| 5 | Identified any additional bug from the table (#4 or #5) | 2 |
| 6 | All fixes compile/parse without syntax errors | 1 |
| 7 | Existing tests still pass after fixes | 1 |

---

## Task 3: Upgrade Outdated Go Dependencies

**Category**: Dependency management
**Difficulty**: Medium
**Time Limit**: 15 minutes

### Prompt Given to Agent

> The Go services (metrics-engine and read-model) have outdated indirect dependencies. In particular, `golang.org/x/crypto` is pinned to a 2022 version with known CVEs. Upgrade the Go dependencies in both `src/services/metrics-engine/go.mod` and `src/services/read-model/go.mod` to their latest compatible versions. Make sure both services still build and their tests pass. Also update `go.sum` files accordingly.

### Grading Rubric (8 points)

| # | Criterion | How to Verify |
|---|-----------|---------------|
| 1 | `metrics-engine/go.mod` `golang.org/x/crypto` version is newer than `v0.0.0-20220622213112` | `grep 'golang.org/x/crypto' src/services/metrics-engine/go.mod` shows a newer version |
| 2 | `read-model/go.mod` `golang.org/x/crypto` version is newer than `v0.0.0-20220622213112` | `grep 'golang.org/x/crypto' src/services/read-model/go.mod` shows a newer version |
| 3 | `go.sum` files are updated (not stale) | `cd src/services/metrics-engine && go mod verify` exits 0 |
| 4 | `golang.org/x/text` and `golang.org/x/sync` are also updated | Both show versions newer than 2022 |
| 5 | Metrics Engine builds | `cd src/services/metrics-engine && go build ./...` exits 0 |
| 6 | Read Model builds | `cd src/services/read-model && go build ./...` exits 0 |
| 7 | Metrics Engine tests pass | `cd src/services/metrics-engine && go test ./...` exits 0 |
| 8 | Read Model tests pass | `cd src/services/read-model && go test ./...` exits 0 |

---

## Task 4: Add a New Field End-to-End Across the Data Pipeline

**Category**: Cross-service feature implementation
**Difficulty**: Hard
**Time Limit**: 30 minutes

### Prompt Given to Agent

> Add a `priority` field to the job schema (`contracts/schemas/job.json`). The field should be an optional string enum with values `"low"`, `"normal"`, `"high"`, defaulting to `"normal"`. Propagate this field through the full pipeline:
>
> 1. **Schema**: Add `priority` to `contracts/schemas/job.json` as an optional enum field.
> 2. **Gateway (TypeScript)**: Accept `priority` in the POST /jobs request body. Pass it through to the event payload. Add a test.
> 3. **Processor (Python)**: Read `priority` from the job payload and store it in PostgreSQL (add it to the CREATE TABLE and INSERT statements). Add a test.
> 4. **Read Model (Go)**: Include `priority` in the `Job` struct and the `/jobs/recent` SQL query. Add a test.
> 5. **Bump** the job schema version to `1.1.0` (minor, backward-compatible).

### Grading Rubric (14 points)

| # | Criterion | How to Verify |
|---|-----------|---------------|
| 1 | `job.json` has `priority` field with enum `["low","normal","high"]` | Parse schema, check `properties.priority.enum` |
| 2 | `priority` is NOT in `required` array | Parse schema, check `required` array |
| 3 | `priority` has `default: "normal"` | Parse schema, check `properties.priority.default` |
| 4 | `job.json` `$version` bumped to `1.1.0` | Parse schema, check `$version` |
| 5 | `VERSIONS.md` updated with job `1.1.0` entry | `grep` for `1.1.0` in VERSIONS.md |
| 6 | Gateway accepts `priority` in request body | `grep -q 'priority' src/services/gateway/lib/builders.ts` or types file |
| 7 | Gateway has a new test for `priority` | `grep -rq 'priority' src/services/gateway/__tests__/` |
| 8 | Processor SQL includes `priority` column | `grep -q 'priority' src/services/processor/main.py` |
| 9 | Processor has a new test for `priority` | `grep -rq 'priority' src/services/processor/tests/` |
| 10 | Read Model `Job` struct has `Priority` field | `grep -q 'Priority' src/services/read-model/main.go` |
| 11 | Read Model SQL query includes `priority` | `grep -q 'priority' src/services/read-model/main.go` |
| 12 | Gateway tests pass | `cd src/services/gateway && npx vitest run` exits 0 |
| 13 | Processor tests pass | `cd src/services/processor && python -m pytest tests/` exits 0 |
| 14 | Read Model tests pass | `cd src/services/read-model && go test ./...` exits 0 |

---

## Task 5: Write Tests for Untested Code Paths

**Category**: Test authoring
**Difficulty**: Medium
**Time Limit**: 20 minutes

### Prompt Given to Agent

> The Read Model service (`src/services/read-model/`) has only 18% test coverage. Write unit tests for the following functions in `main.go`:
>
> 1. `statsHandler` — Test that it returns valid JSON with the expected fields (`totalJobs`, `completedJobs`, `failedJobs`, `lastEventTime`).
> 2. `recentJobsHandler` — Test that it returns a JSON array.
> 3. `corsMiddleware` — Test that it sets CORS headers and handles OPTIONS requests with 200.
> 4. `healthHandler` — Test that it returns `{"status":"ok","version":"..."}`.
>
> Use the standard Go `testing` package and `net/http/httptest`. Mock external dependencies (Redis, PostgreSQL, MongoDB) where necessary. All tests must pass.

### Grading Rubric (10 points)

| # | Criterion | How to Verify |
|---|-----------|---------------|
| 1 | Test file exists | `ls src/services/read-model/*_test.go` finds test file(s) |
| 2 | Test for `healthHandler` exists | `grep -q 'TestHealth\|Test_health' src/services/read-model/*_test.go` |
| 3 | `healthHandler` test checks JSON response shape | Test verifies `status` and `version` fields |
| 4 | Test for `corsMiddleware` exists | `grep -q 'TestCors\|Test_cors\|CORS' src/services/read-model/*_test.go` |
| 5 | `corsMiddleware` test checks headers | Test verifies `Access-Control-Allow-Origin` header |
| 6 | `corsMiddleware` test checks OPTIONS method | Test sends OPTIONS request and verifies 200 |
| 7 | Test for `statsHandler` exists | `grep -q 'TestStats\|Test_stats' src/services/read-model/*_test.go` |
| 8 | Test for `recentJobsHandler` exists | `grep -q 'TestRecent\|Test_recent' src/services/read-model/*_test.go` |
| 9 | All tests compile | `cd src/services/read-model && go vet ./...` exits 0 |
| 10 | All tests pass | `cd src/services/read-model && go test ./...` exits 0 |

---

## Task 6: Self-Review a Complex Change

**Category**: Code review quality
**Difficulty**: Medium
**Time Limit**: 25 minutes

### Prompt Given to Agent

> First, implement the following change: Add rate limiting to the Gateway's POST /jobs endpoint. Use a simple in-memory token bucket: max 100 requests per minute per IP. Return HTTP 429 with a JSON body `{"error": "Rate limit exceeded", "retryAfter": <seconds>}` when the limit is hit. Add the `X-RateLimit-Remaining` header to successful responses.
>
> After implementing, review your own diff critically. Identify at least 3 potential issues, edge cases, or improvements in your own implementation.

### Grading Rubric (12 points)

| # | Criterion | How to Verify |
|---|-----------|---------------|
| 1 | Rate limiter is implemented (file exists or code added) | `grep -rq 'rate.*limit\|rateLimit\|token.*bucket\|rateLimiter' src/services/gateway/` |
| 2 | Returns HTTP 429 when limit exceeded | `grep -q '429' src/services/gateway/lib/app.ts` or new middleware file |
| 3 | Response body has `error` and `retryAfter` fields | Code review of the 429 response |
| 4 | `X-RateLimit-Remaining` header is set on success | `grep -q 'RateLimit-Remaining\|ratelimit-remaining' src/services/gateway/` |
| 5 | Rate limit is per-IP (uses `req.ip` or similar) | Code review confirms per-IP tracking |
| 6 | At least 1 new test for the rate limiter | `grep -rq 'rate\|429\|limit' src/services/gateway/__tests__/` |
| 7 | Tests pass after change | `cd src/services/gateway && npx vitest run` exits 0 |
| 8 | Self-review identifies a real concern (e.g., memory leak from unbounded IP map) | Review text mentions memory/cleanup |
| 9 | Self-review identifies a testing gap or edge case | Review text mentions proxy/X-Forwarded-For, concurrent access, or similar |
| 10 | Self-review identifies a production concern | Review text mentions distributed rate limiting, persistence, or horizontal scaling |
| 11 | Self-review is actionable (not just vague observations) | Each concern has a suggested fix or mitigation |
| 12 | No security vulnerabilities introduced (no eval, injection, etc.) | Manual code review |

---

## Task 7: Create a Feature Implementation Plan

**Category**: Architecture and planning
**Difficulty**: Medium
**Time Limit**: 15 minutes

### Prompt Given to Agent

> Create a detailed implementation plan for adding "Job Cancellation" to the system. Users should be able to cancel a PENDING or EXECUTING job via a new `DELETE /jobs/:id` endpoint on the Gateway. The cancellation must propagate through the event spine (RabbitMQ) so the Processor stops work and the Metrics Engine and Read Model reflect the cancelled state.
>
> Your plan should cover: schema changes needed, API changes per service, new RabbitMQ events/queues, database migrations, error handling (what if the job is already completed?), test strategy, and a suggested implementation order. Write the plan as a markdown file.

### Grading Rubric (10 points)

| # | Criterion | How to Verify |
|---|-----------|---------------|
| 1 | Plan proposes adding `CANCELLED` to the job status enum in `job.json` | Text mentions schema change with `CANCELLED` status |
| 2 | Plan proposes a new event type (e.g., `job.cancelled`) in `event-envelope.json` | Text mentions new event type |
| 3 | Plan describes Gateway changes (new DELETE endpoint) | Text covers Gateway API change |
| 4 | Plan describes Processor changes (stop in-flight work, handle cancellation event) | Text covers Processor behavior |
| 5 | Plan describes Metrics Engine changes (consume cancellation events, update counters) | Text covers Metrics Engine |
| 6 | Plan describes Read Model changes (new status in SQL query, API response) | Text covers Read Model |
| 7 | Plan addresses the race condition (job completes before cancel arrives) | Text discusses idempotency or ordering |
| 8 | Plan includes a test strategy (unit + integration) | Text has a testing section |
| 9 | Plan specifies implementation order across services | Text has an ordered implementation sequence |
| 10 | Plan is technically accurate (no hallucinated APIs, correct queue names, etc.) | Manual review against actual codebase |

---

## Task 8: Fix All Linting Violations

**Category**: Code quality and tooling
**Difficulty**: Easy
**Time Limit**: 15 minutes

### Prompt Given to Agent

> Run all configured linters across the codebase and fix every violation. The linters are:
>
> - **Gateway (TypeScript)**: `cd src/services/gateway && npx eslint . --ext .ts,.js` and `npx prettier --check .`
> - **Go services**: `cd src/services/metrics-engine && golangci-lint run` and `cd src/services/read-model && golangci-lint run`
> - **Rust services**: `cd src/services/web-pty-server && cargo clippy -- -D warnings` and `cd src/interfaces/tui && cargo clippy -- -D warnings`
>
> Fix all warnings and errors. Do not disable rules — fix the underlying code. After fixing, re-run each linter to confirm zero violations.

### Grading Rubric (8 points)

| # | Criterion | How to Verify |
|---|-----------|---------------|
| 1 | ESLint runs clean on Gateway | `cd src/services/gateway && npx eslint . --ext .ts,.js` exits 0 with no output |
| 2 | Prettier runs clean on Gateway | `cd src/services/gateway && npx prettier --check .` exits 0 |
| 3 | golangci-lint runs clean on Metrics Engine | `cd src/services/metrics-engine && golangci-lint run` exits 0 |
| 4 | golangci-lint runs clean on Read Model | `cd src/services/read-model && golangci-lint run` exits 0 |
| 5 | Cargo clippy clean on Web PTY Server | `cd src/services/web-pty-server && cargo clippy -- -D warnings` exits 0 |
| 6 | Cargo clippy clean on TUI | `cd src/interfaces/tui && cargo clippy -- -D warnings` exits 0 |
| 7 | No linter rules were disabled or suppressed | `! grep -rn 'nolint\|eslint-disable\|allow(clippy' --include='*.ts' --include='*.go' --include='*.rs'` finds no new suppressions (diff against main) |
| 8 | All existing tests still pass | Run test suite for each service; all exit 0 |

---

## Task 9: Standardize Health Check Endpoints

**Category**: Cross-service consistency
**Difficulty**: Medium
**Time Limit**: 20 minutes

### Prompt Given to Agent

> The health check endpoints are inconsistent across services:
> - Gateway uses `/healthz` (readiness) and `/readyz` (liveness)
> - Read Model uses `/health` (no `/healthz` or `/readyz`)
> - Metrics Engine has no HTTP health endpoint at all
> - Web PTY Server uses `/health` on a separate metrics port
>
> Standardize all services to expose both `/healthz` (readiness, checks dependencies) and `/readyz` (liveness, always 200 if process is running). Update the Kubernetes manifests in `infra/k8s/` to use the correct paths. Update the Docker Compose health checks in `docker-compose.integration.yml` if they reference health endpoints. Add or update tests for the new endpoints.

### Grading Rubric (12 points)

| # | Criterion | How to Verify |
|---|-----------|---------------|
| 1 | Read Model registers `/healthz` endpoint | `grep -q '/healthz' src/services/read-model/main.go` |
| 2 | Read Model registers `/readyz` endpoint | `grep -q '/readyz' src/services/read-model/main.go` |
| 3 | Read Model `/healthz` checks at least one dependency (Redis, Postgres, or MongoDB) | Code review shows dependency check |
| 4 | Read Model `/readyz` returns 200 unconditionally | Code review confirms simple liveness |
| 5 | K8s `read-model.yaml` uses `/healthz` for readinessProbe | `grep -A2 'readinessProbe' infra/k8s/read-model.yaml \| grep '/healthz'` |
| 6 | K8s `read-model.yaml` uses `/readyz` for livenessProbe | `grep -A2 'livenessProbe' infra/k8s/read-model.yaml \| grep '/readyz'` |
| 7 | Read Model tests cover `/healthz` | `grep -q 'healthz' src/services/read-model/*_test.go` |
| 8 | Read Model tests cover `/readyz` | `grep -q 'readyz' src/services/read-model/*_test.go` |
| 9 | Read Model builds | `cd src/services/read-model && go build ./...` exits 0 |
| 10 | Read Model tests pass | `cd src/services/read-model && go test ./...` exits 0 |
| 11 | No existing Gateway tests broken | `cd src/services/gateway && npx vitest run` exits 0 |
| 12 | K8s manifests have consistent probe paths across all services | Manual review of all `infra/k8s/*.yaml` files |

---

## Task 10: Explain the Architecture from Code

**Category**: Code comprehension
**Difficulty**: Easy
**Time Limit**: 10 minutes

### Prompt Given to Agent

> Without reading any README or documentation files (only source code, configuration, and infrastructure files), answer these specific questions:
>
> 1. What happens when a user submits a job via POST /jobs? Trace the full data flow through every service, naming the specific queues, databases, and collections involved.
> 2. What are the exact RabbitMQ queue names used by the system?
> 3. Which services write to PostgreSQL and which write to MongoDB?
> 4. What is the contract validation strategy — which library does each language use for JSON Schema validation?
> 5. What schema version is currently deployed and how is version compatibility tracked?

### Grading Rubric (10 points)

| # | Criterion | Points |
|---|-----------|--------|
| 1 | Correctly traces: Gateway validates → publishes to `jobs.created` queue | 1 |
| 2 | Correctly identifies: Processor consumes from `jobs.created`, writes to PostgreSQL, publishes to `jobs.completed` | 1 |
| 3 | Correctly identifies: Metrics Engine consumes from `jobs.completed`, writes to MongoDB (`observatory.job_events`) and Redis | 1 |
| 4 | Correctly identifies: Read Model reads from PostgreSQL, Redis, and MongoDB | 1 |
| 5 | Lists all 3 queue names: `jobs.created`, `jobs.completed`, `jobs.failed.validation` | 1 |
| 6 | Correctly assigns PostgreSQL writers (Processor) and MongoDB writers (Metrics Engine) | 1 |
| 7 | Names AJV (Gateway/TS), jsonschema (Processor/Python), gojsonschema (Metrics Engine/Go) | 1 |
| 8 | Identifies schema version as `1.0.0` for all schemas | 1 |
| 9 | Mentions `contracts/VERSIONS.md` or schema `$version` field for version tracking | 1 |
| 10 | No factual errors in the explanation | 1 |

---

## Scoring Summary

| Task | Category | Max Points | Difficulty |
|------|----------|------------|------------|
| 1 | Cross-Service Schema Evolution | 12 | Hard |
| 2 | Find and Fix Real Bugs | 10 | Medium |
| 3 | Upgrade Outdated Go Dependencies | 8 | Medium |
| 4 | Add Field End-to-End | 14 | Hard |
| 5 | Write Tests for Untested Code | 10 | Medium |
| 6 | Self-Review a Complex Change | 12 | Medium |
| 7 | Feature Implementation Plan | 10 | Medium |
| 8 | Fix All Linting Violations | 8 | Easy |
| 9 | Standardize Health Check Endpoints | 12 | Medium |
| 10 | Explain Architecture from Code | 10 | Easy |
| **Total** | | **106** | |

### Capability Matrix

Each task primarily tests one or more agent capabilities:

| Capability | Tasks |
|------------|-------|
| Multi-language code editing | 1, 4, 9 |
| Schema/contract understanding | 1, 4, 7, 10 |
| Bug detection | 2 |
| Dependency management | 3 |
| Test authoring | 5, 6 |
| Self-critique / review | 6 |
| Architecture comprehension | 7, 10 |
| Tooling / linter fluency | 8 |
| K8s / infrastructure | 9 |
| Code reading (no writing) | 10 |

---

## Automation Notes

### Running the Grading Script

Most rubric items can be automated with a shell script. The pattern for each check:

```bash
# Example: Check that event-envelope.json has retryCount field
check() {
    local description="$1"
    local command="$2"
    if eval "$command" > /dev/null 2>&1; then
        echo "PASS: $description"
        return 0
    else
        echo "FAIL: $description"
        return 1
    fi
}

# Task 1, Criterion 1
check "event-envelope.json has retryCount field" \
    "python3 -c \"import json; s=json.load(open('contracts/schemas/event-envelope.json')); assert 'retryCount' in s['properties']\""
```

### Items Requiring Manual Review

The following rubric items cannot be fully automated and require human judgment:

- **Task 2, #2**: Whether the state-name mismatch explanation is correct
- **Task 6, #8-11**: Quality of self-review observations
- **Task 7, all items**: Technical accuracy of the plan
- **Task 10, #10**: Absence of factual errors

All other items are fully automatable via `grep`, `python -c`, or exit-code checks.
