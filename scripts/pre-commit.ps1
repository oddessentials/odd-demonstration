# Pre-commit hook for Windows (PowerShell)
# Runs contract and CI validation before allowing commit

Write-Host "[CHECK] Running pre-commit checks..." -ForegroundColor Cyan

# Run contract validation
Write-Host ">> Contract Validation" -ForegroundColor Yellow
& "$PSScriptRoot\validate-contracts.ps1"

if ($LASTEXITCODE -ne 0) {
    Write-Host "[FAIL] Contract validation failed" -ForegroundColor Red
    exit 1
}

# Run CI workflow validation
Write-Host ">> CI Workflow Validation" -ForegroundColor Yellow
& "$PSScriptRoot\validate-ci.ps1"

if ($LASTEXITCODE -ne 0) {
    Write-Host "[FAIL] CI workflow validation failed" -ForegroundColor Red
    exit 1
}
# Run version sync validation (includes Cargo.lock)
Write-Host ">> Version Sync Validation" -ForegroundColor Yellow
& "$PSScriptRoot\verify-version-sync.ps1"

if ($LASTEXITCODE -ne 0) {
    Write-Host "[FAIL] Version sync validation failed" -ForegroundColor Red
    Write-Host "  Run 'cargo update --workspace' in src/interfaces/tui to fix Cargo.lock" -ForegroundColor Yellow
    exit 1
}

Write-Host "[PASS] All pre-commit checks passed" -ForegroundColor Green
exit 0


