# Integration Gate - End-to-End Proof
# This script validates the entire Distributed Task Observatory system

param(
    [int]$JobCount = 5,
    [string]$GatewayUrl = "http://localhost:3001",
    [string]$ReadModelUrl = "http://localhost:8080"
)

$ErrorActionPreference = "Stop"
$passed = 0
$failed = 0

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

Write-Host ("=" * 60) -ForegroundColor Cyan
Write-Host "  DISTRIBUTED TASK OBSERVATORY - INTEGRATION GATE" -ForegroundColor Cyan
Write-Host ("=" * 60) -ForegroundColor Cyan
Write-Host ""

# Test 1: Gateway Health
Write-Host ">> Test 1 - Gateway Health Check" -ForegroundColor Yellow
try {
    $health = Invoke-RestMethod -Uri "$GatewayUrl/healthz" -TimeoutSec 5
    Write-Test "Gateway Health" ($health -eq "OK")
} catch {
    Write-Test "Gateway Health" $false $_.Exception.Message
}

# Test 2: Read Model Health
Write-Host ">> Test 2 - Read Model Health Check" -ForegroundColor Yellow
try {
    $health = Invoke-RestMethod -Uri "$ReadModelUrl/health" -TimeoutSec 5
    Write-Test "Read Model Health" ($health -eq "OK")
} catch {
    Write-Test "Read Model Health" $false $_.Exception.Message
}

# Test 3: Submit Jobs
Write-Host ">> Test 3 - Submit $JobCount Jobs" -ForegroundColor Yellow
$jobIds = @()
$submitSuccess = $true

for ($i = 1; $i -le $JobCount; $i++) {
    try {
        $jobId = [guid]::NewGuid().ToString()
        $job = @{
            id = $jobId
            type = "integration-test"
            status = "PENDING"
            createdAt = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
            payload = @{
                testRun = "integration-gate"
                iteration = $i
            }
        } | ConvertTo-Json
        
        $result = Invoke-RestMethod -Uri "$GatewayUrl/jobs" -Method Post -Body $job -ContentType "application/json"
        $jobIds += $result.jobId
        $shortId = $result.jobId.Substring(0, 8)
        Write-Host "  Submitted job ${i} - ${shortId}..." -ForegroundColor Gray
    } catch {
        $submitSuccess = $false
        $errMsg = $_.Exception.Message
        Write-Host "  Failed to submit job ${i} - ${errMsg}" -ForegroundColor Red
    }
}
Write-Test "Job Submission ($JobCount jobs)" $submitSuccess

# Test 4: Wait for Processing
Write-Host ">> Test 4 - Wait for Processing (10s)" -ForegroundColor Yellow
Start-Sleep -Seconds 10

# Test 5: Check PostgreSQL via Read Model
Write-Host ">> Test 5 - Verify Jobs in Read Model" -ForegroundColor Yellow
try {
    $jobs = Invoke-RestMethod -Uri "$ReadModelUrl/jobs/recent"
    $completedCount = ($jobs | Where-Object { $_.status -eq "COMPLETED" }).Count
    Write-Test "Jobs Processed" ($completedCount -ge $JobCount) "Found $completedCount completed jobs"
} catch {
    Write-Test "Jobs Processed" $false $_.Exception.Message
}

# Test 6: Check Stats
Write-Host ">> Test 6 - Verify Aggregated Stats" -ForegroundColor Yellow
try {
    $stats = Invoke-RestMethod -Uri "$ReadModelUrl/stats"
    $statsValid = ($stats.totalJobs -ge $JobCount) -and ($stats.completedJobs -ge $JobCount)
    $totalJobs = $stats.totalJobs
    $completedJobs = $stats.completedJobs
    Write-Test "Stats Aggregation" $statsValid "Total - $totalJobs, Completed - $completedJobs"
} catch {
    Write-Test "Stats Aggregation" $false $_.Exception.Message
}

# Test 7: Metrics Endpoint
Write-Host ">> Test 7 - Gateway Metrics Exposed" -ForegroundColor Yellow
try {
    $metrics = Invoke-RestMethod -Uri "$GatewayUrl/metrics"
    $hasMetrics = $metrics -match "gateway_jobs_submitted_total"
    Write-Test "Gateway Metrics" $hasMetrics
} catch {
    Write-Test "Gateway Metrics" $false $_.Exception.Message
}

# Summary
Write-Host ""
Write-Host ("=" * 60) -ForegroundColor Cyan
Write-Host "  INTEGRATION GATE RESULTS" -ForegroundColor Cyan
Write-Host ("=" * 60) -ForegroundColor Cyan
Write-Host "  Passed - $passed" -ForegroundColor Green
Write-Host "  Failed - $failed" -ForegroundColor Red
Write-Host ""

if ($failed -eq 0) {
    Write-Host "  [OK] ALL TESTS PASSED - SYSTEM VERIFIED" -ForegroundColor Green
    exit 0
} else {
    Write-Host "  [X] INTEGRATION GATE FAILED" -ForegroundColor Red
    exit 1
}
