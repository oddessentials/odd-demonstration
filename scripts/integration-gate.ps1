# Integration Gate - End-to-End Proof (v2)
# EXECUTION ASSUMPTIONS:
# - kubectl context: kind-task-observatory
# - Port-forwards: Gateway (3000), Read Model (8080)
# - Optional: RabbitMQ (15672) for queue checks
# - All pods must be in Ready state

param(
    [int]$JobCount = 3,
    [string]$GatewayUrl = "http://localhost:3000",
    [string]$ReadModelUrl = "http://localhost:8080",
    [string]$RabbitMQUrl = "http://localhost:15672",
    [int]$TimeoutSec = 10,
    [int]$MaxRetries = 3,
    [int]$RetryDelayMs = 2000
)

$ErrorActionPreference = "Stop"
$passed = 0
$failed = 0
$testJobIds = @()

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

function Invoke-WithRetry {
    param(
        [scriptblock]$Action,
        [int]$MaxRetries = 3,
        [int]$DelayMs = 2000
    )
    
    for ($i = 1; $i -le $MaxRetries; $i++) {
        try {
            return & $Action
        } catch {
            if ($i -eq $MaxRetries) { throw }
            Start-Sleep -Milliseconds $DelayMs
        }
    }
}

Write-Host ("=" * 60) -ForegroundColor Cyan
Write-Host "  INTEGRATION GATE v2 - Deterministic Tests" -ForegroundColor Cyan
Write-Host ("=" * 60) -ForegroundColor Cyan
Write-Host ""

# PRE-FLIGHT: Verify Execution Assumptions
Write-Host ">> Pre-flight: Execution Assumptions" -ForegroundColor Yellow

# Check kubectl context
$context = kubectl config current-context 2>$null
if ($context -ne "kind-task-observatory") {
    Write-Test "Kubectl Context" $false "Expected 'kind-task-observatory', got '$context'"
    Write-Host "  [ABORT] Cannot continue without correct context" -ForegroundColor Red
    exit 1
}
Write-Test "Kubectl Context" $true

# Check pods are Ready (not just Running)
Write-Host "  Waiting for pods to be Ready..." -ForegroundColor Gray
try {
    kubectl wait --for=condition=Ready pods --all --timeout=60s 2>&1 | Out-Null
    Write-Test "Pods Ready" $true
} catch {
    Write-Test "Pods Ready" $false $_.Exception.Message
    exit 1
}

# TEST 1: Gateway Health with retry
Write-Host ">> Test 1 - Gateway Health (with retry)" -ForegroundColor Yellow
try {
    $health = Invoke-WithRetry -MaxRetries $MaxRetries -DelayMs $RetryDelayMs -Action {
        Invoke-RestMethod -Uri "$GatewayUrl/healthz" -TimeoutSec $TimeoutSec
    }
    # Health endpoint may return JSON {"status":"ok"} or plain "OK"
    $isHealthy = ($health -eq "OK") -or ($health.status -eq "ok")
    Write-Test "Gateway Health" $isHealthy
} catch {
    Write-Test "Gateway Health" $false $_.Exception.Message
}

# TEST 2: Read Model Health with retry
Write-Host ">> Test 2 - Read Model Health (with retry)" -ForegroundColor Yellow
try {
    $health = Invoke-WithRetry -MaxRetries $MaxRetries -DelayMs $RetryDelayMs -Action {
        Invoke-RestMethod -Uri "$ReadModelUrl/health" -TimeoutSec $TimeoutSec
    }
    # Health endpoint may return JSON {"status":"ok"} or plain "OK"
    $isHealthy = ($health -eq "OK") -or ($health.status -eq "ok")
    Write-Test "Read Model Health" $isHealthy
} catch {
    Write-Test "Read Model Health" $false $_.Exception.Message
}

# TEST 3: Submit Jobs and verify response structure
Write-Host ">> Test 3 - Submit $JobCount Jobs" -ForegroundColor Yellow
$submitSuccess = $true
$testRunId = [guid]::NewGuid().ToString().Substring(0, 8)

for ($i = 1; $i -le $JobCount; $i++) {
    try {
        $jobId = [guid]::NewGuid().ToString()
        $job = @{
            id = $jobId
            type = "integration-test-$testRunId"
            status = "PENDING"
            createdAt = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
            payload = @{
                testRun = $testRunId
                iteration = $i
            }
        } | ConvertTo-Json
        
        $result = Invoke-RestMethod -Uri "$GatewayUrl/jobs" -Method Post -Body $job -ContentType "application/json" -TimeoutSec $TimeoutSec
        
        # Verify response has both jobId and eventId
        if (-not $result.jobId -or -not $result.eventId) {
            throw "Response missing jobId or eventId"
        }
        
        $testJobIds += $result.jobId
        Write-Host "  Job $i - $($result.jobId.Substring(0, 8))... (event: $($result.eventId.Substring(0, 8))...)" -ForegroundColor Gray
    } catch {
        $submitSuccess = $false
        Write-Host "  Failed job $i - $($_.Exception.Message)" -ForegroundColor Red
    }
}
Write-Test "Job Submission" $submitSuccess

# TEST 4: Wait and verify jobs completed
Write-Host ">> Test 4 - Wait for Processing (30s max)" -ForegroundColor Yellow
$processingSuccess = $false
for ($wait = 1; $wait -le 6; $wait++) {
    Start-Sleep -Seconds 5
    try {
        $jobs = Invoke-RestMethod -Uri "$ReadModelUrl/jobs/recent" -TimeoutSec $TimeoutSec
        $testJobs = $jobs | Where-Object { $testJobIds -contains $_.id }
        $completed = ($testJobs | Where-Object { $_.status -eq "COMPLETED" }).Count
        Write-Host "  Check $wait - $completed/$JobCount completed" -ForegroundColor Gray
        if ($completed -ge $JobCount) {
            $processingSuccess = $true
            break
        }
    } catch {
        Write-Host "  Check $wait - Error: $($_.Exception.Message)" -ForegroundColor Yellow
    }
}
Write-Test "Jobs Processed" $processingSuccess "Expected $JobCount, check logs"

# TEST 5: MongoDB Event Persistence (scoped to test jobs)
Write-Host ">> Test 5 - MongoDB Event Persistence" -ForegroundColor Yellow
$eventSuccess = $true
$eventErrors = @()

# Get all events and check for test run jobs
try {
    $allEvents = Invoke-RestMethod -Uri "$ReadModelUrl/events" -TimeoutSec $TimeoutSec
    
    # The response may be in BSON Key/Value format or standard JSON
    # Check if any events contain our test job IDs
    $eventsJson = $allEvents | ConvertTo-Json -Depth 10
    
    foreach ($jobId in $testJobIds) {
        if ($eventsJson -notmatch $jobId) {
            $eventErrors += "No event references job $($jobId.Substring(0, 8))"
            $eventSuccess = $false
        }
    }
    
    # Check for unique event IDs (handle BSON format)
    $eventIdPattern = '"eventid":\s*"([^"]+)"'
    $matches = [regex]::Matches($eventsJson, $eventIdPattern, 'IgnoreCase')
    $eventIds = $matches | ForEach-Object { $_.Groups[1].Value }
    
    if ($eventIds.Count -gt 0) {
        $uniqueIds = $eventIds | Select-Object -Unique
        if ($eventIds.Count -ne $uniqueIds.Count) {
            $eventErrors += "Duplicate event IDs detected ($($eventIds.Count) total, $($uniqueIds.Count) unique)"
            $eventSuccess = $false
        }
    }
} catch {
    $eventErrors += "Query failed: $($_.Exception.Message)"
    $eventSuccess = $false
}

Write-Test "MongoDB Events" $eventSuccess ($eventErrors -join "; ")


# TEST 6: Stats Aggregation
Write-Host ">> Test 6 - Stats Aggregation" -ForegroundColor Yellow
try {
    $stats = Invoke-RestMethod -Uri "$ReadModelUrl/stats" -TimeoutSec $TimeoutSec
    Write-Test "Stats Aggregation" ($stats.totalJobs -ge $JobCount) "Total: $($stats.totalJobs), Completed: $($stats.completedJobs)"
} catch {
    Write-Test "Stats Aggregation" $false $_.Exception.Message
}

# TEST 7: Gateway Metrics
Write-Host ">> Test 7 - Gateway Metrics" -ForegroundColor Yellow
try {
    $metrics = Invoke-RestMethod -Uri "$GatewayUrl/metrics" -TimeoutSec $TimeoutSec
    Write-Test "Gateway Metrics" ($metrics -match "gateway_jobs_submitted_total")
} catch {
    Write-Test "Gateway Metrics" $false $_.Exception.Message
}

# SUMMARY
Write-Host ""
Write-Host ("=" * 60) -ForegroundColor Cyan
Write-Host "  INTEGRATION GATE RESULTS" -ForegroundColor Cyan
Write-Host ("=" * 60) -ForegroundColor Cyan
Write-Host "  Passed: $passed" -ForegroundColor Green
Write-Host "  Failed: $failed" -ForegroundColor Red
Write-Host "  Test Run ID: $testRunId" -ForegroundColor Gray
Write-Host ""

if ($failed -eq 0) {
    Write-Host "  [OK] ALL TESTS PASSED" -ForegroundColor Green
    exit 0
} else {
    Write-Host "  [X] INTEGRATION GATE FAILED" -ForegroundColor Red
    exit 1
}
