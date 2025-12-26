#!/usr/bin/env pwsh
# validate-compose-k8s-parity.ps1
# Drift prevention: ensures docker-compose.integration.yml has all critical k8s services
#
# This prevents the scenario where k8s manifests add a service but docker-compose
# doesn't get updated, breaking integration tests.

$ErrorActionPreference = "Stop"

Write-Host "=== Docker Compose / K8s Parity Check ===" -ForegroundColor Cyan

# Critical services that MUST exist in both k8s and docker-compose
# This is enforced as invariant I7 in docs/INVARIANTS.md
$criticalServices = @(
    "postgres",
    "mongodb", 
    "redis",
    "rabbitmq",
    "prometheus",
    "grafana",
    "gateway",
    "processor",
    "metrics-engine",
    "read-model",
    "web-pty-server",
    "web-ui"
)

# Parse docker-compose services
$composeFile = "docker-compose.integration.yml"
if (-not (Test-Path $composeFile)) {
    Write-Host "[FAIL] $composeFile not found" -ForegroundColor Red
    exit 1
}

$composeContent = Get-Content $composeFile -Raw
$composeServices = @()

# Extract service names from compose file (lines that start with 2 spaces followed by service name and colon)
foreach ($line in (Get-Content $composeFile)) {
    if ($line -match "^  ([a-z][a-z0-9-]+):$") {
        $composeServices += $matches[1]
    }
}

Write-Host "`nDocker Compose services found: $($composeServices.Count)"
foreach ($svc in $composeServices) {
    Write-Host "  - $svc" -ForegroundColor DarkGray
}

# Check for missing critical services
$missing = @()
foreach ($required in $criticalServices) {
    $found = $false
    foreach ($svc in $composeServices) {
        # Allow variations like "mongodb" vs "mongo", "web-pty-server" vs "web-pty"
        if ($svc -like "*$required*" -or $required -like "*$svc*") {
            $found = $true
            break
        }
    }
    if (-not $found) {
        $missing += $required
    }
}

Write-Host ""
if ($missing.Count -gt 0) {
    Write-Host "[FAIL] Missing critical services in docker-compose:" -ForegroundColor Red
    foreach ($svc in $missing) {
        Write-Host "  - $svc" -ForegroundColor Red
    }
    Write-Host ""
    Write-Host "These services exist in k8s but not in docker-compose.integration.yml" -ForegroundColor Yellow
    Write-Host "Please add them to prevent integration test drift." -ForegroundColor Yellow
    exit 1
} else {
    Write-Host "[PASS] All critical services present in docker-compose" -ForegroundColor Green
}

# Check k8s manifests for new services not in critical list
$k8sDir = "infra/k8s"
if (Test-Path $k8sDir) {
    $k8sManifests = Get-ChildItem -Path $k8sDir -Filter "*.yaml" | Select-Object -ExpandProperty BaseName
    Write-Host "`nK8s manifests found: $($k8sManifests.Count)"
    
    $unmapped = @()
    foreach ($manifest in $k8sManifests) {
        # Skip config maps and rules
        if ($manifest -like "*-config*" -or $manifest -like "*-rules*" -or $manifest -like "*-datasource*" -or $manifest -like "*-dashboards*" -or $manifest -like "*-ingress*") {
            continue
        }
        $found = $false
        foreach ($critical in $criticalServices) {
            if ($manifest -like "*$critical*" -or $critical -like "*$manifest*") {
                $found = $true
                break
            }
        }
        if (-not $found) {
            $unmapped += $manifest
        }
    }
    
    if ($unmapped.Count -gt 0) {
        Write-Host "`n[WARN] K8s services not in critical list (may be optional):" -ForegroundColor Yellow
        foreach ($svc in $unmapped) {
            Write-Host "  - $svc" -ForegroundColor Yellow
        }
        Write-Host "If these are required for integration tests, add them to `$criticalServices" -ForegroundColor DarkGray
    }
}

Write-Host "`n=== Parity check complete ===" -ForegroundColor Cyan
exit 0
