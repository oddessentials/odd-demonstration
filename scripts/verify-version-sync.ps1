#!/usr/bin/env pwsh
# Cross-platform version sync verification
# Verifies all version references match the authoritative VERSION file

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot

$VersionFile = Join-Path $Root "src/interfaces/tui/VERSION"
if (-not (Test-Path $VersionFile)) {
    Write-Host "ERROR: VERSION file not found at $VersionFile" -ForegroundColor Red
    exit 1
}

$Authoritative = (Get-Content $VersionFile -Raw).Trim()
Write-Host "Authoritative version: $Authoritative" -ForegroundColor Cyan

$Errors = @()
$Checked = 0

# 1. Cargo.toml
$CargoToml = Join-Path $Root "src/interfaces/tui/Cargo.toml"
if (Test-Path $CargoToml) {
    $Checked++
    $content = Get-Content $CargoToml -Raw
    if ($content -match 'version\s*=\s*"([^"]+)"') {
        $cargoVersion = $Matches[1]
        if ($cargoVersion -ne $Authoritative) {
            $Errors += "Cargo.toml: $cargoVersion (expected $Authoritative)"
        } else {
            Write-Host "  [OK] Cargo.toml: $cargoVersion" -ForegroundColor Green
        }
    }
}

# 2. npm shim package.json
$NpmPkg = Join-Path $Root "packages/npm-shim/package.json"
if (Test-Path $NpmPkg) {
    $Checked++
    $npm = Get-Content $NpmPkg | ConvertFrom-Json
    if ($npm.version -ne $Authoritative) {
        $Errors += "npm package.json: $($npm.version) (expected $Authoritative)"
    } else {
        Write-Host "  [OK] npm package.json: $($npm.version)" -ForegroundColor Green
    }
}

# Summary
Write-Host ""
Write-Host "Checked $Checked version sources" -ForegroundColor Cyan

if ($Errors.Count -gt 0) {
    Write-Host ""
    Write-Host "VERSION MISMATCH:" -ForegroundColor Red
    $Errors | ForEach-Object { Write-Host "  - $_" -ForegroundColor Yellow }
    exit 1
}

Write-Host "[OK] All version sources match: $Authoritative" -ForegroundColor Green
exit 0
