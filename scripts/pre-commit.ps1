# Pre-commit hook for Windows (PowerShell)
# Runs contract validation before allowing commit

Write-Host "[CHECK] Running pre-commit checks..." -ForegroundColor Cyan

# Run contract validation
Write-Host ">> Contract Validation" -ForegroundColor Yellow
& "$PSScriptRoot\validate-contracts.ps1"

if ($LASTEXITCODE -ne 0) {
    Write-Host "[FAIL] Contract validation failed" -ForegroundColor Red
    exit 1
}

Write-Host "[PASS] All pre-commit checks passed" -ForegroundColor Green
exit 0

