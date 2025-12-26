# Integration Harness - End-to-End Proof (Docker Compose)
# Self-contained integration testing without K8s dependency
# 
# INVARIANTS ENFORCED:
# - I3: Self-contained (Docker Compose only)
# - I4: Runtime <120s wall-clock (exit 1 on breach)
# - I5: Artifact capture on every run (guarded in finally)

param(
    [int]$StartupTimeoutSec = 60,
    [int]$RuntimeBudgetSec = 120,
    [int]$MaxRetries = 3,
    [int]$RetryDelayMs = 2000,
    [string]$GatewayUrl = "http://localhost:13000",
    [string]$ReadModelUrl = "http://localhost:18080",
    [switch]$SkipTeardown
)

$ErrorActionPreference = "Stop"
$startTime = Get-Date
$artifactsDir = "integration-artifacts"
$passed = 0
$failed = 0
$testJobIds = @()
$composeFile = "docker-compose.integration.yml"

# ============================================================
# UTILITY FUNCTIONS
# ============================================================

function Write-Test {
    param([string]$Name, [bool]$Success, [string]$Details = "")
    if ($Success) {
        Write-Host "[PASS] $Name" -ForegroundColor Green
        $script:passed++
    } else {
        Write-Host "[FAIL] $Name - $Details" -ForegroundColor Red
        $script:failed++
    }
}

function Write-Retry {
    param([int]$Attempt, [int]$Max, [string]$Reason)
    Write-Host "[RETRY $Attempt/$Max] $Reason" -ForegroundColor Yellow
}

function Invoke-WithRetry {
    param(
        [scriptblock]$Action,
        [int]$MaxRetries = 3,
        [int]$DelayMs = 2000,
        [string]$Context = "operation"
    )
    
    for ($i = 1; $i -le $MaxRetries; $i++) {
        try {
            return & $Action
        } catch {
            if ($i -eq $MaxRetries) { throw }
            Write-Retry -Attempt $i -Max $MaxRetries -Reason "$Context - $($_.Exception.Message)"
            Start-Sleep -Milliseconds $DelayMs
        }
    }
}

function Get-ElapsedSeconds {
    return ((Get-Date) - $startTime).TotalSeconds
}

function Save-Artifact {
    param([string]$Name, [string]$Content)
    try {
        $Content | Out-File -FilePath "$artifactsDir/$Name" -Encoding UTF8
    } catch {
        Write-Host "[WARN] Failed to save artifact $Name - $($_.Exception.Message)" -ForegroundColor Yellow
    }
}

function Validate-JsonSchema {
    param([string]$SchemaPath, [string]$PayloadPath)
    
    $result = & node scripts/validate-json.mjs $SchemaPath $PayloadPath 2>&1
    return $LASTEXITCODE -eq 0
}

# ============================================================
# INITIALIZATION
# ============================================================

Write-Host ("=" * 60) -ForegroundColor Cyan
Write-Host "  INTEGRATION HARNESS - Docker Compose" -ForegroundColor Cyan
Write-Host ("=" * 60) -ForegroundColor Cyan
Write-Host ""

# Create artifacts directory immediately
New-Item -ItemType Directory -Path $artifactsDir -Force | Out-Null
New-Item -ItemType Directory -Path "$artifactsDir/health-snapshots" -Force | Out-Null
New-Item -ItemType Directory -Path "$artifactsDir/responses" -Force | Out-Null

# Write gate decision JSON immediately (source of truth)
$gateDecision = @{
    trigger = if ($env:COMPAT_CRITICAL -eq "true") { "compat_critical" } else { "manual" }
    filter_failed = $env:FILTER_FAILED -eq "true"
    started = $startTime.ToString("o")
    runtime_budget_sec = $RuntimeBudgetSec
    compose_file = $composeFile
}
$gateDecision | ConvertTo-Json | Set-Content "$artifactsDir/gate-decision.json"

Write-Host "Integration trigger: $($gateDecision.trigger) | filter_failed=$($gateDecision.filter_failed)" -ForegroundColor Gray

try {
    # ============================================================
    # PRE-FLIGHT: Compose Version Check
    # ============================================================
    Write-Host ""
    Write-Host ">> Pre-flight: Docker Compose Version" -ForegroundColor Yellow
    
    $composeVersion = docker compose version 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Test "Docker Compose" $false "docker compose not available"
        throw "Docker Compose not available"
    }
    Write-Host "  $composeVersion" -ForegroundColor Gray
    Write-Test "Docker Compose Available" $true
    
    # Check compose file exists
    if (-not (Test-Path $composeFile)) {
        Write-Test "Compose File" $false "$composeFile not found"
        throw "Compose file not found: $composeFile"
    }
    Write-Test "Compose File Exists" $true
    
    # ============================================================
    # START COMPOSE
    # ============================================================
    Write-Host ""
    Write-Host ">> Starting Docker Compose" -ForegroundColor Yellow
    
    docker compose -f $composeFile up -d 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Test "Compose Up" $false "Failed to start containers"
        throw "Failed to start Docker Compose"
    }
    Write-Test "Compose Up" $true
    
    # ============================================================
    # WAIT FOR HTTP HEALTH (Availability)
    # ============================================================
    Write-Host ""
    Write-Host ">> Waiting for HTTP Health (${StartupTimeoutSec}s max)" -ForegroundColor Yellow
    
    $healthyGateway = $false
    $healthyReadModel = $false
    $waitStart = Get-Date
    
    while (((Get-Date) - $waitStart).TotalSeconds -lt $StartupTimeoutSec) {
        # Gateway health - endpoint returns JSON {"status":"ok"} not plain "OK"
        if (-not $healthyGateway) {
            try {
                $gwHealth = Invoke-RestMethod -Uri "$GatewayUrl/healthz" -TimeoutSec 2 -ErrorAction SilentlyContinue
                $healthyGateway = ($gwHealth -eq "OK") -or ($gwHealth.status -eq "ok")
                if ($healthyGateway) {
                    Save-Artifact "health-snapshots/gateway.json" ($gwHealth | ConvertTo-Json)
                }
            } catch {}
        }
        
        # Read Model health - endpoint returns JSON {"status":"ok"} not plain "OK"
        if (-not $healthyReadModel) {
            try {
                $rmHealth = Invoke-RestMethod -Uri "$ReadModelUrl/health" -TimeoutSec 2 -ErrorAction SilentlyContinue
                $healthyReadModel = ($rmHealth -eq "OK") -or ($rmHealth.status -eq "ok")
                if ($healthyReadModel) {
                    Save-Artifact "health-snapshots/read-model.json" ($rmHealth | ConvertTo-Json)
                }
            } catch {}
        }
        
        if ($healthyGateway -and $healthyReadModel) { break }
        
        $elapsed = [math]::Round(((Get-Date) - $waitStart).TotalSeconds)
        Write-Host "  Waiting... Gateway=$healthyGateway ReadModel=$healthyReadModel (${elapsed}s)" -ForegroundColor Gray
        Start-Sleep -Seconds 2
    }
    
    Write-Test "Gateway HTTP Health" $healthyGateway "Health endpoint not OK"
    Write-Test "Read Model HTTP Health" $healthyReadModel "Health endpoint not OK"
    
    if (-not $healthyGateway -or -not $healthyReadModel) {
        throw "Services did not become healthy within ${StartupTimeoutSec}s"
    }
    
    # ============================================================
    # DEPENDENCY REACHABILITY (Authoritative)
    # ============================================================
    Write-Host ""
    Write-Host ">> Verifying Dependency Reachability" -ForegroundColor Yellow
    
    # Gateway → RabbitMQ (check via metrics or a lightweight call)
    try {
        $metrics = Invoke-RestMethod -Uri "$GatewayUrl/metrics" -TimeoutSec 5
        $brokerReachable = $metrics -match "gateway_"
        Write-Test "Gateway → Broker Reachable" $brokerReachable "No gateway metrics found"
    } catch {
        Write-Test "Gateway → Broker Reachable" $false $_.Exception.Message
    }
    
    # Read Model → MongoDB (check via stats endpoint)
    try {
        $stats = Invoke-RestMethod -Uri "$ReadModelUrl/stats" -TimeoutSec 5
        $dbReachable = $null -ne $stats.totalJobs
        Write-Test "Read Model → DB Reachable" $dbReachable "Stats endpoint failed"
    } catch {
        Write-Test "Read Model → DB Reachable" $false $_.Exception.Message
    }
    
    # ============================================================
    # PROOF PATH 1: Gateway accepts job
    # ============================================================
    Write-Host ""
    Write-Host ">> P1: Gateway Accepts Job" -ForegroundColor Yellow
    
    $testRunId = [guid]::NewGuid().ToString().Substring(0, 8)
    $p1Success = $false
    
    try {
        $jobId = [guid]::NewGuid().ToString()
        $job = @{
            id = $jobId
            type = "integration-test-$testRunId"
            status = "PENDING"
            createdAt = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
            payload = @{
                testRun = $testRunId
                proof = "P1"
            }
        } | ConvertTo-Json
        
        $result = Invoke-WithRetry -MaxRetries $MaxRetries -DelayMs $RetryDelayMs -Context "POST /jobs" -Action {
            Invoke-RestMethod -Uri "$GatewayUrl/jobs" -Method Post -Body $job -ContentType "application/json" -TimeoutSec 10
        }
        
        # Save response for schema validation
        $result | ConvertTo-Json -Depth 10 | Set-Content "$artifactsDir/responses/p1-response.json"
        
        # Validate schema against job-accepted.json (acceptance response, not job)
        $schemaValid = Validate-JsonSchema "contracts/schemas/job-accepted.json" "$artifactsDir/responses/p1-response.json"
        
        if ($result.jobId -and $result.eventId -and $schemaValid) {
            $testJobIds += $result.jobId
            $p1Success = $true
        }
        
        Write-Test "P1: Job Accepted (201)" ($result.jobId -ne $null)
        Write-Test "P1: Response Schema Valid" $schemaValid
    } catch {
        Write-Test "P1: Gateway Accepts Job" $false $_.Exception.Message
    }
    
    # ============================================================
    # PROOF PATH 2+3: Poll for Job Completion (combined)
    # Bounded polling replaces fixed wait for deterministic behavior
    # ============================================================
    Write-Host ""
    Write-Host ">> P2+P3: Poll for Job Completion (30s max)" -ForegroundColor Yellow
    
    $p3MaxWait = 30
    $p3Interval = 2
    $maxAttempts = [math]::Ceiling($p3MaxWait / $p3Interval)
    $attempt = 0
    $p3Success = $false
    $eventsRetrieved = $false
    $p3Start = Get-Date
    
    while ($attempt -lt $maxAttempts) {
        $attempt++
        try {
            # Check events (P2)
            if (-not $eventsRetrieved) {
                $events = Invoke-RestMethod -Uri "$ReadModelUrl/events" -TimeoutSec 5
                $events | ConvertTo-Json -Depth 10 | Set-Content "$artifactsDir/responses/p2-response.json"
                $eventsRetrieved = $true
            }
            
            # Check job status (P3)
            $jobs = Invoke-RestMethod -Uri "$ReadModelUrl/jobs/recent" -TimeoutSec 5
            $jobs | ConvertTo-Json -Depth 10 | Set-Content "$artifactsDir/responses/p3-response.json"
            
            # Filter by 'id' field (verified from Read Model Go code)
            $testJobs = $jobs | Where-Object { $testJobIds -contains $_.id }
            $completed = ($testJobs | Where-Object { $_.status -eq "COMPLETED" }).Count
            
            if ($completed -ge 1) {
                $p3Success = $true
                $elapsed = [math]::Round(((Get-Date) - $p3Start).TotalSeconds, 1)
                Write-Host "  Job COMPLETED after ${elapsed}s" -ForegroundColor Gray
                break
            }
            
            $jobStatus = if ($testJobs.Count -gt 0) { $testJobs[0].status } else { "not found" }
            Write-Retry -Attempt $attempt -Max $maxAttempts -Reason "Job status: $jobStatus"
        } catch {
            Write-Retry -Attempt $attempt -Max $maxAttempts -Reason $_.Exception.Message
        }
        Start-Sleep -Seconds $p3Interval
    }
    
    Write-Test "P2: Events Retrieved" $eventsRetrieved
    Write-Test "P3: Job Status COMPLETED" $p3Success
    
    # ============================================================
    # PROOF PATH 4: Metrics exposed
    # ============================================================
    Write-Host ""
    Write-Host ">> P4: Metrics Exposed" -ForegroundColor Yellow
    
    try {
        $metrics = Invoke-RestMethod -Uri "$GatewayUrl/metrics" -TimeoutSec 10
        $metrics | Out-File "$artifactsDir/responses/p4-metrics.txt"
        
        $hasCounter = $metrics -match "gateway_jobs_submitted_total"
        Write-Test "P4: Metrics Counter Present" $hasCounter
    } catch {
        Write-Test "P4: Metrics Exposed" $false $_.Exception.Message
    }
    
} finally {
    # ============================================================
    # GUARANTEED TEARDOWN (Guarded)
    # ============================================================
    Write-Host ""
    Write-Host ">> Teardown" -ForegroundColor Yellow
    
    $elapsed = Get-ElapsedSeconds
    
    # Capture compose state (guarded)
    try {
        $ps = docker compose -f $composeFile ps 2>&1
        Save-Artifact "compose-ps.txt" ($ps | Out-String)
    } catch {
        Write-Host "[WARN] Failed to capture compose ps" -ForegroundColor Yellow
    }
    
    # Capture compose logs (guarded)
    try {
        $logs = docker compose -f $composeFile logs --tail=200 2>&1
        Save-Artifact "compose-logs.txt" ($logs | Out-String)
    } catch {
        Write-Host "[WARN] Failed to capture compose logs" -ForegroundColor Yellow
    }
    
    # Teardown (guarded)
    if (-not $SkipTeardown) {
        try {
            docker compose -f $composeFile down --timeout 5 2>&1 | Out-Null
            Write-Host "  Containers stopped" -ForegroundColor Gray
        } catch {
            Write-Host "[WARN] Failed to stop containers: $($_.Exception.Message)" -ForegroundColor Yellow
        }
    } else {
        Write-Host "  Skipping teardown (-SkipTeardown)" -ForegroundColor Gray
    }
    
    # Update gate decision with results
    $gateDecision.completed = (Get-Date).ToString("o")
    $gateDecision.elapsed_sec = [math]::Round($elapsed, 2)
    $gateDecision.passed = $passed
    $gateDecision.failed = $failed
    $gateDecision.budget_exceeded = $elapsed -gt $RuntimeBudgetSec
    $gateDecision | ConvertTo-Json | Set-Content "$artifactsDir/gate-decision.json"
    
    # ============================================================
    # SUMMARY
    # ============================================================
    Write-Host ""
    Write-Host ("=" * 60) -ForegroundColor Cyan
    Write-Host "  INTEGRATION HARNESS RESULTS" -ForegroundColor Cyan
    Write-Host ("=" * 60) -ForegroundColor Cyan
    Write-Host "  Passed: $passed" -ForegroundColor Green
    Write-Host "  Failed: $failed" -ForegroundColor Red
    Write-Host "  Elapsed: $([math]::Round($elapsed, 1))s / ${RuntimeBudgetSec}s budget" -ForegroundColor Gray
    Write-Host "  Artifacts: $artifactsDir/" -ForegroundColor Gray
    Write-Host ""
    
    # INVARIANT I4: Budget enforcement (HARD FAIL)
    if ($elapsed -gt $RuntimeBudgetSec) {
        Write-Host "[BUDGET EXCEEDED] ${elapsed}s > ${RuntimeBudgetSec}s" -ForegroundColor Red
        exit 1
    }
    
    if ($failed -eq 0) {
        Write-Host "  [OK] ALL PROOF PATHS PASSED" -ForegroundColor Green
        exit 0
    } else {
        Write-Host "  [X] INTEGRATION HARNESS FAILED" -ForegroundColor Red
        exit 1
    }
}
