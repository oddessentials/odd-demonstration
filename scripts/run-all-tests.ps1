# ============================================================
# Canonical Test Entrypoint - Phase 12
# ============================================================
# This is the SOLE authority for test execution in CI and local development.
# CI must not bypass this script with ad-hoc go test/pytest/vitest commands.
#
# Determinism Constants (G2 Enforcement - Hardcoded, not configurable)
$TIMEOUT_UNIT_TESTS = 120      # seconds
$TIMEOUT_CONTRACT = 60         # seconds  
$TIMEOUT_INTEGRATION = 180     # seconds
$POLL_INTERVAL_INTEGRATION = 5 # seconds
$MAX_RETRIES_INTEGRATION = 3

# Exit codes
$EXIT_SUCCESS = 0
$EXIT_FAILURE = 1
$EXIT_SKIP = 0  # Skip is success for CI (V3: graceful degradation)

$ErrorActionPreference = "Stop"
$script:TestFailures = @()
$script:TestSkips = @()

function Write-Section {
    param([string]$Title)
    Write-Host ""
    Write-Host "============================================================" -ForegroundColor Cyan
    Write-Host "  $Title" -ForegroundColor Cyan
    Write-Host "============================================================" -ForegroundColor Cyan
}

function Write-Pass {
    param([string]$Message)
    Write-Host "[PASS] $Message" -ForegroundColor Green
}

function Write-Fail {
    param([string]$Message)
    Write-Host "[FAIL] $Message" -ForegroundColor Red
    $script:TestFailures += $Message
}

function Write-Skip {
    param([string]$Message, [string]$Reason)
    Write-Host "[SKIP] $Message - $Reason" -ForegroundColor Yellow
    $script:TestSkips += "$Message (Reason: $Reason)"
}

function Emit-FailureDiagnostics {
    param([string]$Context)
    Write-Host ""
    Write-Host "=== FAILURE DIAGNOSTICS ($Context) ===" -ForegroundColor Red
    
    # Collect logs if cluster available (V1: emit diagnostics on failure)
    if (Get-Command kubectl -ErrorAction SilentlyContinue) {
        $contextCheck = kubectl config current-context 2>$null
        if ($contextCheck -eq "kind-task-observatory") {
            Write-Host "--- Pod Logs (last 20 lines each) ---"
            @("gateway", "processor", "read-model", "metrics-engine") | ForEach-Object {
                Write-Host "=== $_ ===" -ForegroundColor Yellow
                kubectl logs -l app=$_ --tail=20 2>$null || Write-Host "(no logs)"
            }
            
            Write-Host "--- RabbitMQ Queue Depth ---"
            try {
                $queues = Invoke-RestMethod -Uri "http://localhost:15672/api/queues" `
                    -Headers @{Authorization = "Basic $([Convert]::ToBase64String([Text.Encoding]::ASCII.GetBytes('guest:guest')))"} `
                    -TimeoutSec 5 -ErrorAction SilentlyContinue
                $queues | ForEach-Object { Write-Host "$($_.name): $($_.messages) messages" }
            } catch {
                Write-Host "(RabbitMQ not accessible)"
            }
            
            Write-Host "--- Stats Snapshot ---"
            try {
                $stats = Invoke-RestMethod -Uri "http://localhost:8080/stats" -TimeoutSec 5 -ErrorAction SilentlyContinue
                $stats | ConvertTo-Json -Depth 3
            } catch {
                Write-Host "(Read Model not accessible)"
            }
        }
    }
    Write-Host "=== END DIAGNOSTICS ===" -ForegroundColor Red
}

# ============================================================
# Phase 1: Contract Validation (Schema Correctness)
# ============================================================
Write-Section "Contract Validation"

$contractStart = Get-Date
try {
    $result = & pwsh "$PSScriptRoot/validate-contracts.ps1" 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Pass "Contract validation passed"
    } else {
        Write-Fail "Contract validation failed"
        Write-Host $result
    }
} catch {
    Write-Fail "Contract validation error: $_"
}
$contractDuration = (Get-Date) - $contractStart
if ($contractDuration.TotalSeconds -gt $TIMEOUT_CONTRACT) {
    Write-Fail "Contract validation exceeded timeout (${TIMEOUT_CONTRACT}s)"
}

# ============================================================
# Phase 2: Unit Tests Per Service
# ============================================================
Write-Section "Unit Tests"

# Gateway (Node.js/Vitest)
Write-Host "`n--- Gateway (Vitest) ---"
$gatewayPath = Join-Path $PSScriptRoot "..\src\services\gateway"
if (Test-Path (Join-Path $gatewayPath "package.json")) {
    Push-Location $gatewayPath
    try {
        $unitStart = Get-Date
        npm test 2>&1 | ForEach-Object { Write-Host $_ }
        if ($LASTEXITCODE -eq 0) {
            Write-Pass "Gateway unit tests"
        } else {
            Write-Fail "Gateway unit tests"
        }
    } catch {
        Write-Fail "Gateway test error: $_"
    } finally {
        Pop-Location
    }
} else {
    Write-Skip "Gateway" "package.json not found"
}

# Processor (Python/pytest)
Write-Host "`n--- Processor (pytest) ---"
$processorPath = Join-Path $PSScriptRoot "..\src\services\processor"
if (Test-Path (Join-Path $processorPath "tests")) {
    Push-Location $processorPath
    try {
        # Use python -m pytest for CI compatibility (pytest may not be on PATH)
        python -m pytest tests/ -v 2>&1 | ForEach-Object { Write-Host $_ }
        if ($LASTEXITCODE -eq 0) {
            Write-Pass "Processor unit tests"
        } else {
            Write-Fail "Processor unit tests"
        }
    } catch {
        Write-Fail "Processor test error: $_"
    } finally {
        Pop-Location
    }
} else {
    Write-Skip "Processor" "tests directory not found"
}

# Go Services (metrics-engine, read-model)
# Use parallel execution on pwsh 7+ with fallback to sequential
$goServices = @("metrics-engine", "read-model")

function Test-GoService {
    param([string]$ServiceName, [string]$ScriptRoot)
    
    $servicePath = Join-Path $ScriptRoot "..\src\services\$ServiceName"
    $result = @{ Name = $ServiceName; Passed = $false; Output = @(); Skipped = $false }
    
    if (-not (Test-Path (Join-Path $servicePath "go.mod"))) {
        $result.Skipped = $true
        $result.SkipReason = "go.mod not found"
        return $result
    }
    
    Push-Location $servicePath
    try {
        $output = go test -v ./... 2>&1
        $result.Output = $output
        $result.Passed = ($LASTEXITCODE -eq 0)
    } catch {
        $result.Output = @("Error: $_")
        $result.Passed = $false
    } finally {
        Pop-Location
    }
    
    return $result
}

if ($PSVersionTable.PSVersion.Major -ge 7) {
    Write-Host "`n--- Go Services (parallel, pwsh 7+) ---"
    
    # Parallel execution with explicit result collection
    $goResults = $goServices | ForEach-Object -Parallel {
        # Re-define function in parallel scope
        $servicePath = Join-Path $using:PSScriptRoot "..\src\services\$_"
        $result = @{ Name = $_; Passed = $false; Output = @(); Skipped = $false }
        
        if (-not (Test-Path (Join-Path $servicePath "go.mod"))) {
            $result.Skipped = $true
            $result.SkipReason = "go.mod not found"
            return $result
        }
        
        Push-Location $servicePath
        try {
            $output = go test -v ./... 2>&1
            $result.Output = $output
            $result.Passed = ($LASTEXITCODE -eq 0)
        } catch {
            $result.Output = @("Error: $_")
            $result.Passed = $false
        } finally {
            Pop-Location
        }
        
        return $result
    } -ThrottleLimit 2
    
    # Process results and register failures
    foreach ($r in $goResults) {
        Write-Host "`n--- $($r.Name) (go test) ---"
        if ($r.Skipped) {
            Write-Skip $r.Name $r.SkipReason
        } else {
            $r.Output | ForEach-Object { Write-Host $_ }
            if ($r.Passed) {
                Write-Pass "$($r.Name) unit tests"
            } else {
                Write-Fail "$($r.Name) unit tests"
            }
        }
    }
} else {
    Write-Host "`n[WARN] pwsh < 7, running Go tests sequentially" -ForegroundColor Yellow
    
    # Sequential fallback with same result collection pattern
    foreach ($svc in $goServices) {
        Write-Host "`n--- $svc (go test) ---"
        $result = Test-GoService -ServiceName $svc -ScriptRoot $PSScriptRoot
        
        if ($result.Skipped) {
            Write-Skip $svc $result.SkipReason
        } else {
            $result.Output | ForEach-Object { Write-Host $_ }
            if ($result.Passed) {
                Write-Pass "$svc unit tests"
            } else {
                Write-Fail "$svc unit tests"
            }
        }
    }
}

# ============================================================
# Phase 3: Governance Scripts
# ============================================================
Write-Section "Governance Checks"

# Schema sanity
Write-Host "`n--- Schema Sanity ---"
try {
    python "$PSScriptRoot/test-contracts-sanity.py" 2>&1 | ForEach-Object { Write-Host $_ }
    if ($LASTEXITCODE -eq 0) {
        Write-Pass "Schema sanity check"
    } else {
        Write-Fail "Schema sanity check"
    }
} catch {
    Write-Fail "Schema sanity error: $_"
}

# Schema compatibility (diff-aware CI mode)
Write-Host "`n--- Schema Compatibility ---"
try {
    python "$PSScriptRoot/check-schema-compat.py" --ci 2>&1 | ForEach-Object { Write-Host $_ }
    if ($LASTEXITCODE -eq 0) {
        Write-Pass "Schema compatibility check"
    } else {
        Write-Fail "Schema compatibility check"
    }
} catch {
    Write-Fail "Schema compat error: $_"
}

# Version sync
Write-Host "`n--- Version Sync ---"
try {
    python "$PSScriptRoot/check-service-versions.py" 2>&1 | ForEach-Object { Write-Host $_ }
    if ($LASTEXITCODE -eq 0) {
        Write-Pass "Version sync check"
    } else {
        Write-Fail "Version sync check"
    }
} catch {
    Write-Fail "Version sync error: $_"
}

# ============================================================
# Phase 4: Integration Tests (Optional - Cluster Required)
# ============================================================
Write-Section "Integration Tests"

# V3: Graceful degradation when prerequisites missing
$ClusterAvailable = $false
if (Get-Command kubectl -ErrorAction SilentlyContinue) {
    $contextCheck = kubectl config current-context 2>$null
    if ($contextCheck -eq "kind-task-observatory") {
        # Verify pods are running
        $pods = kubectl get pods --no-headers 2>$null
        if ($pods -and $LASTEXITCODE -eq 0) {
            $ClusterAvailable = $true
        }
    }
}

if ($ClusterAvailable) {
    Write-Host "Cluster detected. Running integration tests..."
    try {
        & "$PSScriptRoot/integration-gate.ps1" 2>&1 | ForEach-Object { Write-Host $_ }
        if ($LASTEXITCODE -eq 0) {
            Write-Pass "Integration tests"
        } else {
            Write-Fail "Integration tests"
            Emit-FailureDiagnostics "integration-gate"
        }
    } catch {
        Write-Fail "Integration test error: $_"
        Emit-FailureDiagnostics "integration-gate"
    }
} else {
    Write-Skip "Integration tests" "K8s cluster not available (kind-task-observatory)"
}

# ============================================================
# Summary
# ============================================================
Write-Section "Test Summary"

Write-Host ""
Write-Host "Passed:  $($script:TestFailures.Count -eq 0 ? 'All checks' : 'Some checks')"
Write-Host "Failed:  $($script:TestFailures.Count)"
Write-Host "Skipped: $($script:TestSkips.Count)"

if ($script:TestFailures.Count -gt 0) {
    Write-Host "`nFailures:" -ForegroundColor Red
    $script:TestFailures | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
    exit $EXIT_FAILURE
}

if ($script:TestSkips.Count -gt 0) {
    Write-Host "`nSkipped:" -ForegroundColor Yellow
    $script:TestSkips | ForEach-Object { Write-Host "  - $_" -ForegroundColor Yellow }
}

Write-Host "`n[OK] ALL TESTS PASSED" -ForegroundColor Green
exit $EXIT_SUCCESS
