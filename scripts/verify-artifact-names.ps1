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

# Check install.sh - uses dynamic pattern: odd-dashboard-${PLATFORM}
$InstallSh = Join-Path $Root "install.sh"
if (Test-Path $InstallSh) {
    $Checked++
    $content = Get-Content $InstallSh -Raw
    
    # install.sh uses: ARTIFACT="odd-dashboard-${PLATFORM}"
    # and sets PLATFORM to linux-x64, linux-arm64, macos-x64, macos-arm64
    # So we verify the pattern and platform mappings exist
    $hasPattern = $content -match 'ARTIFACT="odd-dashboard-\$\{PLATFORM\}"' -or $content -match "ARTIFACT=.*odd-dashboard.*PLATFORM"
    $hasLinuxX64 = $content -match 'PLATFORM="linux-x64"'
    $hasLinuxArm64 = $content -match 'PLATFORM="linux-arm64"'
    $hasMacosX64 = $content -match 'PLATFORM="macos-x64"'
    $hasMacosArm64 = $content -match 'PLATFORM="macos-arm64"'
    
    $missing = @()
    if (-not $hasPattern) { $missing += "artifact pattern" }
    if (-not $hasLinuxX64) { $missing += "linux-x64" }
    if (-not $hasLinuxArm64) { $missing += "linux-arm64" }
    if (-not $hasMacosX64) { $missing += "macos-x64" }
    if (-not $hasMacosArm64) { $missing += "macos-arm64" }
    
    if ($missing.Count -gt 0) {
        $Errors += "install.sh: missing: $($missing -join ', ')"
    } else {
        Write-Host "  [OK] install.sh: artifact pattern and all platforms present" -ForegroundColor Green
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
