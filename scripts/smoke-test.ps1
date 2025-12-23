# smoke-test.ps1
# Fast Smoke Test - Core Services Only
# Target: <30 seconds, assumes cluster running and port-forwards established

param(
    [string]$GatewayUrl = "http://localhost:3000",
    [string]$ReadModelUrl = "http://localhost:8080",
    [string]$RabbitMQUrl = "http://localhost:15672",
    [int]$TimeoutSeconds = 5
)

$ErrorActionPreference = "Stop"
$startTime = Get-Date
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

Write-Host "=" * 50 -ForegroundColor Cyan
Write-Host "  SMOKE TEST - Core Services" -ForegroundColor Cyan
Write-Host "=" * 50 -ForegroundColor Cyan
Write-Host ""

# Pre-flight: Verify kubectl context
Write-Host ">> Pre-flight: Kubectl Context" -ForegroundColor Yellow
$context = kubectl config current-context 2>$null
if ($context -eq "kind-task-observatory") {
    Write-Test "Kubectl Context" $true
} else {
    Write-Test "Kubectl Context" $false "Expected 'kind-task-observatory', got '$context'"
}

# Pre-flight: Verify pods are Ready
Write-Host ">> Pre-flight: Pod Readiness" -ForegroundColor Yellow
try {
    $waitResult = kubectl wait --for=condition=Ready pods --all --timeout=10s 2>&1
    Write-Test "Pods Ready" $true
} catch {
    Write-Test "Pods Ready" $false $_.Exception.Message
}

# Test 1: Gateway Health
Write-Host ">> Gateway Health" -ForegroundColor Yellow
try {
    $health = Invoke-RestMethod -Uri "$GatewayUrl/healthz" -TimeoutSec $TimeoutSeconds
    Write-Test "Gateway /healthz" ($health -eq "OK")
} catch {
    Write-Test "Gateway /healthz" $false $_.Exception.Message
}

# Test 2: Read Model Health
Write-Host ">> Read Model Health" -ForegroundColor Yellow
try {
    $health = Invoke-RestMethod -Uri "$ReadModelUrl/health" -TimeoutSec $TimeoutSeconds
    Write-Test "Read Model /health" ($health -eq "OK")
} catch {
    Write-Test "Read Model /health" $false $_.Exception.Message
}

# Test 3: RabbitMQ Health (optional - may not have port-forward)
Write-Host ">> RabbitMQ Health (optional)" -ForegroundColor Yellow
try {
    $health = Invoke-RestMethod -Uri "$RabbitMQUrl/api/healthchecks/node" -TimeoutSec $TimeoutSeconds -Credential (New-Object PSCredential("guest", (ConvertTo-SecureString "guest" -AsPlainText -Force)))
    Write-Test "RabbitMQ Health" ($health.status -eq "ok")
} catch {
    Write-Host "  [SKIP] RabbitMQ - Port-forward not active or auth failed" -ForegroundColor Yellow
}

# Summary
$elapsed = (Get-Date) - $startTime
Write-Host ""
Write-Host "=" * 50 -ForegroundColor Cyan
Write-Host "  SMOKE TEST RESULTS" -ForegroundColor Cyan
Write-Host "  Passed: $passed | Failed: $failed" -ForegroundColor $(if ($failed -eq 0) { "Green" } else { "Red" })
Write-Host "  Duration: $($elapsed.TotalSeconds.ToString('F1'))s" -ForegroundColor Cyan
Write-Host "=" * 50 -ForegroundColor Cyan

if ($elapsed.TotalSeconds -gt 30) {
    Write-Host "  [WARN] Smoke test exceeded 30s target" -ForegroundColor Yellow
}

if ($failed -eq 0) {
    Write-Host "  [OK] SMOKE TEST PASSED" -ForegroundColor Green
    exit 0
} else {
    Write-Host "  [X] SMOKE TEST FAILED" -ForegroundColor Red
    exit 1
}
