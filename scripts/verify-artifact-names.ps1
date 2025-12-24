#!/usr/bin/env pwsh
# Cross-platform artifact name consistency verification
# Verifies canonical artifact naming is consistent across all sources

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot

$CanonicalNames = @{
    "windows-x64" = "odd-dashboard-windows-x64.exe"
    "macos-x64" = "odd-dashboard-macos-x64"
    "macos-arm64" = "odd-dashboard-macos-arm64"
    "linux-x64" = "odd-dashboard-linux-x64"
    "linux-arm64" = "odd-dashboard-linux-arm64"
}

Write-Host "Verifying artifact name consistency..." -ForegroundColor Cyan

$Errors = @()
$Checked = 0

# Check install.sh
$InstallSh = Join-Path $Root "install.sh"
if (Test-Path $InstallSh) {
    $Checked++
    $content = Get-Content $InstallSh -Raw
    $missing = @()
    foreach ($entry in $CanonicalNames.GetEnumerator()) {
        if ($content -notmatch [regex]::Escape($entry.Value)) {
            $missing += $entry.Value
        }
    }
    if ($missing.Count -gt 0) {
        $Errors += "install.sh: missing artifact names: $($missing -join ', ')"
    } else {
        Write-Host "  [OK] install.sh: all artifact names present" -ForegroundColor Green
    }
}

# Check install.ps1
$InstallPs1 = Join-Path $Root "install.ps1"
if (Test-Path $InstallPs1) {
    $Checked++
    $content = Get-Content $InstallPs1 -Raw
    if ($content -notmatch "odd-dashboard-windows-x64\.exe") {
        $Errors += "install.ps1: missing canonical Windows artifact name"
    } else {
        Write-Host "  [OK] install.ps1: Windows artifact name present" -ForegroundColor Green
    }
}

# Check release workflow
$ReleaseYml = Join-Path $Root ".github/workflows/release.yml"
if (Test-Path $ReleaseYml) {
    $Checked++
    $content = Get-Content $ReleaseYml -Raw
    $missing = @()
    foreach ($entry in $CanonicalNames.GetEnumerator()) {
        if ($content -notmatch [regex]::Escape($entry.Value)) {
            $missing += $entry.Value
        }
    }
    if ($missing.Count -gt 0) {
        $Errors += "release.yml: missing artifact names: $($missing -join ', ')"
    } else {
        Write-Host "  [OK] release.yml: all artifact names present" -ForegroundColor Green
    }
}

# Summary
Write-Host ""
Write-Host "Checked $Checked sources for artifact name consistency" -ForegroundColor Cyan

if ($Errors.Count -gt 0) {
    Write-Host ""
    Write-Host "ARTIFACT NAME INCONSISTENCIES:" -ForegroundColor Red
    foreach ($e in $Errors) {
        Write-Host "  - $e" -ForegroundColor Yellow
    }
    exit 1
}

Write-Host "[OK] All artifact names consistent" -ForegroundColor Green
exit 0
