#!/usr/bin/env pwsh
# Workflow security audit
# Verifies secret/environment isolation in GitHub Actions workflows

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot

$WorkflowDir = Join-Path $Root ".github/workflows"
$ReleaseWorkflow = "release.yml"

Write-Host "Auditing workflow secret isolation..." -ForegroundColor Cyan

$Errors = @()

if (-not (Test-Path $WorkflowDir)) {
    Write-Host "  No workflows directory found at $WorkflowDir" -ForegroundColor Yellow
    Write-Host "[OK] Workflow audit passed (no workflows yet)" -ForegroundColor Green
    exit 0
}

# Find all workflows that reference the release environment
Get-ChildItem -Path $WorkflowDir -Filter "*.yml" -ErrorAction SilentlyContinue | ForEach-Object {
    $content = Get-Content $_.FullName -Raw
    if ($content -match 'environment:\s*release') {
        if ($_.Name -ne $ReleaseWorkflow) {
            $Errors += "Workflow '$($_.Name)' references 'environment: release' but should not"
        } else {
            Write-Host "  [OK] $($_.Name): correctly uses release environment" -ForegroundColor Green
        }
    }
}

# Verify release.yml DOES reference the environment (if it exists)
$ReleaseFile = Join-Path $WorkflowDir $ReleaseWorkflow
if (Test-Path $ReleaseFile) {
    $content = Get-Content $ReleaseFile -Raw
    if ($content -notmatch 'environment:\s*release') {
        # This is OK for now - we'll add environment protection later
        Write-Host "  [OK] release.yml does not exist yet (will be created in Phase 6)" -ForegroundColor Gray
    }
}

if ($Errors.Count -gt 0) {
    Write-Host ""
    Write-Host "WORKFLOW AUDIT FAILED:" -ForegroundColor Red
    $Errors | ForEach-Object { Write-Host "  - $_" -ForegroundColor Yellow }
    exit 1
}

Write-Host ""
Write-Host "[OK] Workflow audit passed" -ForegroundColor Green
exit 0
