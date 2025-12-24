#!/usr/bin/env pwsh
# Cross-platform naming consistency audit
# Verifies no residual references to old binary names exist
# Runs identically on Windows, macOS, and Linux with PowerShell Core

$ErrorActionPreference = "Stop"
$OldNames = @("observatory-tui", "observatory_tui")
$ExcludePaths = @("target", ".git", "node_modules", "bazel-", "*.lock", "*.log", "test_output.txt", "MODULE.bazel.lock", "audit-naming-consistency.ps1")

$Root = Split-Path -Parent $PSScriptRoot
$Errors = @()

Write-Host "Auditing for residual references to old binary names..." -ForegroundColor Cyan
Write-Host "Root: $Root"

foreach ($name in $OldNames) {
    Write-Host "  Checking for '$name'..." -ForegroundColor Gray
    
    # Use Get-ChildItem + Select-String for cross-platform compatibility
    $files = Get-ChildItem -Path $Root -Recurse -File -ErrorAction SilentlyContinue |
        Where-Object { 
            $path = $_.FullName
            $excluded = $false
            foreach ($exclude in $ExcludePaths) {
                if ($path -like "*$exclude*") {
                    $excluded = $true
                    break
                }
            }
            -not $excluded
        } |
        Select-String -Pattern $name -List -ErrorAction SilentlyContinue
    
    if ($files) {
        foreach ($file in $files) {
            $relativePath = $file.Path.Replace($Root, "").TrimStart("\", "/")
            $Errors += "Found '$name' in: $relativePath"
        }
    }
}

if ($Errors.Count -gt 0) {
    Write-Host ""
    Write-Host "ERROR: Residual references to old binary name found:" -ForegroundColor Red
    $Errors | ForEach-Object { Write-Host "  $_" -ForegroundColor Yellow }
    Write-Host ""
    Write-Host "These must be updated before proceeding with the rename." -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "[OK] No residual references to old binary names" -ForegroundColor Green
exit 0
