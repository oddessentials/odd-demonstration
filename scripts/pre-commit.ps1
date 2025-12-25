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

# Check for new .js files in TypeScript directories (should be .ts)
Write-Host ">> TypeScript Enforcement" -ForegroundColor Yellow
$stagedJsFiles = git diff --cached --name-only --diff-filter=A | Where-Object { 
    $_ -match '\.(js|mjs)$' -and 
    $_ -match '(src/services/gateway|tests|packages)' -and
    $_ -notmatch '(node_modules|dist|build|\.config\.|eslint\.config)'
}

if ($stagedJsFiles) {
    Write-Host "[FAIL] New .js files detected in TypeScript directories:" -ForegroundColor Red
    $stagedJsFiles | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
    Write-Host "  Use .ts extension instead. If intentional, use 'git commit --no-verify'" -ForegroundColor Yellow
    exit 1
}
Write-Host "  [OK] No new .js files in TypeScript directories" -ForegroundColor Green

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


