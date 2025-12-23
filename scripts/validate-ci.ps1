# CI Workflow Validation Script
# Run before commits to catch CI issues early

param(
    [switch]$Verbose
)

$ErrorActionPreference = "Stop"
$script:Errors = @()

Write-Host "=== CI Workflow Validation ===" -ForegroundColor Cyan

# 1. Validate YAML syntax
Write-Host "`n>> Checking YAML syntax..." -ForegroundColor Yellow
$ciPath = "$PSScriptRoot/../.github/workflows/ci.yml"
if (Test-Path $ciPath) {
    try {
        $result = python -c "import yaml; yaml.safe_load(open(r'$ciPath'))" 2>&1
        if ($LASTEXITCODE -eq 0) {
            Write-Host "[OK] ci.yml is valid YAML" -ForegroundColor Green
        } else {
            Write-Host "[FAIL] YAML syntax error in ci.yml" -ForegroundColor Red
            $script:Errors += "YAML syntax error"
        }
    } catch {
        Write-Host "[WARN] Could not validate YAML (Python/PyYAML required)" -ForegroundColor Yellow
    }
} else {
    Write-Host "[FAIL] ci.yml not found at $ciPath" -ForegroundColor Red
    $script:Errors += "ci.yml not found"
}

# 2. Check for known invalid action references
Write-Host "`n>> Checking for known invalid GitHub Actions..." -ForegroundColor Yellow
$content = Get-Content $ciPath -Raw

$invalidActions = @(
    @{ Pattern = "dtolnay/rust-action"; Correct = "dtolnay/rust-toolchain" },
    @{ Pattern = "actions-rs/toolchain"; Correct = "dtolnay/rust-toolchain (actions-rs is unmaintained)" }
)

foreach ($check in $invalidActions) {
    if ($content -match $check.Pattern) {
        Write-Host "[FAIL] Invalid action: $($check.Pattern) - use $($check.Correct)" -ForegroundColor Red
        $script:Errors += "Invalid action: $($check.Pattern)"
    }
}

if ($script:Errors.Count -eq 0) {
    Write-Host "[OK] No known invalid actions found" -ForegroundColor Green
}

# 3. Summary
Write-Host "`n=== Results ===" -ForegroundColor Cyan
if ($script:Errors.Count -gt 0) {
    Write-Host "FAILED: $($script:Errors.Count) issue(s) found" -ForegroundColor Red
    $script:Errors | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
    exit 1
} else {
    Write-Host "PASSED: CI workflow validation complete" -ForegroundColor Green
    exit 0
}
