# bump-version.ps1 - Atomically bump version across all services
# Usage: .\scripts\bump-version.ps1 -NewVersion 0.2.0

param(
    [Parameter(Mandatory=$true)]
    [string]$NewVersion,
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $PSScriptRoot

# Validate SemVer format
if ($NewVersion -notmatch '^\d+\.\d+\.\d+$') {
    Write-Error "Invalid version format. Use SemVer: X.Y.Z (e.g., 0.2.0)"
    exit 1
}

Write-Host "Bumping version to $NewVersion" -ForegroundColor Cyan
if ($DryRun) {
    Write-Host "(Dry run - no files will be modified)" -ForegroundColor Yellow
}

# Files to update
$versionFiles = @(
    "src/services/gateway/VERSION",
    "src/services/processor/VERSION",
    "src/services/metrics-engine/VERSION",
    "src/services/read-model/VERSION"
)

$jsonFiles = @(
    @{ Path = "package.json"; Pattern = '"version":\s*"[^"]*"'; Replacement = '"version": "{0}"' }
)

$tomlFiles = @(
    @{ Path = "src/interfaces/tui/Cargo.toml"; Pattern = '^version\s*=\s*"[^"]*"'; Replacement = 'version = "{0}"' }
)

$bazelFiles = @(
    @{ Path = "MODULE.bazel"; Pattern = 'version\s*=\s*"[^"]*"'; Replacement = 'version = "{0}"' }
)

$k8sFiles = @(
    "infra/k8s/gateway.yaml",
    "infra/k8s/processor.yaml",
    "infra/k8s/metrics-engine.yaml",
    "infra/k8s/read-model.yaml"
)

$updatedFiles = @()

# Update VERSION files
foreach ($file in $versionFiles) {
    $fullPath = Join-Path $ProjectRoot $file
    if (Test-Path $fullPath) {
        Write-Host "  Updating $file" -ForegroundColor Green
        if (-not $DryRun) {
            $NewVersion | Out-File -FilePath $fullPath -Encoding utf8 -NoNewline
        }
        $updatedFiles += $file
    } else {
        Write-Host "  SKIP $file (not found)" -ForegroundColor Yellow
    }
}

# Update JSON files (package.json)
foreach ($item in $jsonFiles) {
    $fullPath = Join-Path $ProjectRoot $item.Path
    if (Test-Path $fullPath) {
        Write-Host "  Updating $($item.Path)" -ForegroundColor Green
        if (-not $DryRun) {
            $content = Get-Content $fullPath -Raw
            $replacement = $item.Replacement -f $NewVersion
            $content = $content -replace $item.Pattern, $replacement
            $content | Out-File -FilePath $fullPath -Encoding utf8 -NoNewline
        }
        $updatedFiles += $item.Path
    }
}

# Update TOML files (Cargo.toml)
foreach ($item in $tomlFiles) {
    $fullPath = Join-Path $ProjectRoot $item.Path
    if (Test-Path $fullPath) {
        Write-Host "  Updating $($item.Path)" -ForegroundColor Green
        if (-not $DryRun) {
            $content = Get-Content $fullPath -Raw
            $replacement = $item.Replacement -f $NewVersion
            $content = $content -replace $item.Pattern, $replacement
            $content | Out-File -FilePath $fullPath -Encoding utf8 -NoNewline
        }
        $updatedFiles += $item.Path
    }
}

# Update Bazel files (MODULE.bazel)
foreach ($item in $bazelFiles) {
    $fullPath = Join-Path $ProjectRoot $item.Path
    if (Test-Path $fullPath) {
        Write-Host "  Updating $($item.Path)" -ForegroundColor Green
        if (-not $DryRun) {
            $content = Get-Content $fullPath -Raw
            $replacement = $item.Replacement -f $NewVersion
            $content = $content -replace $item.Pattern, $replacement
            $content | Out-File -FilePath $fullPath -Encoding utf8 -NoNewline
        }
        $updatedFiles += $item.Path
    }
}

# Update K8s manifest version labels
foreach ($file in $k8sFiles) {
    $fullPath = Join-Path $ProjectRoot $file
    if (Test-Path $fullPath) {
        Write-Host "  Updating $file" -ForegroundColor Green
        if (-not $DryRun) {
            $content = Get-Content $fullPath -Raw
            $content = $content -replace 'app\.kubernetes\.io/version:\s*"[^"]*"', "app.kubernetes.io/version: `"$NewVersion`""
            $content | Out-File -FilePath $fullPath -Encoding utf8 -NoNewline
        }
        $updatedFiles += $file
    }
}

Write-Host ""
Write-Host "Updated $($updatedFiles.Count) files" -ForegroundColor Cyan

if (-not $DryRun) {
    Write-Host ""
    Write-Host "Next steps:" -ForegroundColor Yellow
    Write-Host "  1. Review changes: git diff"
    Write-Host "  2. Update CHANGELOG.md with release notes"
    Write-Host "  3. Commit: git commit -am 'chore: bump version to $NewVersion'"
    Write-Host "  4. Tag: git tag -a v$NewVersion -m 'Release v$NewVersion'"
    Write-Host "  5. Push: git push && git push --tags"
}
