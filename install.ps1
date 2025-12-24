# odd-dashboard installation script for Windows
# Downloads the appropriate binary from GitHub Releases
# Verifies checksum before installation

#Requires -Version 5.1

param(
    [string]$Version = "latest",
    [string]$InstallDir = "$env:LOCALAPPDATA\OddDashboard"
)

$ErrorActionPreference = "Stop"

# Configuration
$Repo = "oddessentials/odd-demonstration"
$Artifact = "odd-dashboard-windows-x64.exe"

function Write-Status {
    param([string]$Message, [string]$Color = "White")
    Write-Host $Message -ForegroundColor $Color
}

function Resolve-Version {
    if ($Version -eq "latest") {
        Write-Status "Fetching latest version..."
        try {
            $release = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
            $script:Version = $release.tag_name -replace '^v', ''
        }
        catch {
            Write-Status "Failed to determine latest version: $_" -Color Red
            exit 1
        }
    }
    Write-Status "Version: v$Version"
    
    $script:BaseUrl = "https://github.com/$Repo/releases/download/v$Version"
}

function Download-AndVerify {
    $tempDir = New-Item -ItemType Directory -Path ([System.IO.Path]::GetTempPath()) -Name "odd-dashboard-install-$(Get-Random)"
    
    try {
        # Download binary
        Write-Status "Downloading $Artifact..."
        $binaryPath = Join-Path $tempDir $Artifact
        Invoke-WebRequest -Uri "$BaseUrl/$Artifact" -OutFile $binaryPath -UseBasicParsing
        
        # Download checksums
        Write-Status "Downloading checksums..."
        $checksumPath = Join-Path $tempDir "SHA256SUMS"
        Invoke-WebRequest -Uri "$BaseUrl/SHA256SUMS" -OutFile $checksumPath -UseBasicParsing
        
        # Verify checksum
        Write-Status "Verifying checksum..."
        
        # Parse checksum file - exact match on filename
        $checksumContent = Get-Content $checksumPath
        $artifactLine = $checksumContent | Where-Object { 
            $_ -match "^([a-fA-F0-9]{64})\s+$([regex]::Escape($Artifact))$" 
        }
        
        if (-not $artifactLine) {
            Write-Status "Artifact '$Artifact' not found in SHA256SUMS" -Color Red
            exit 1
        }
        
        $expected = ($artifactLine -split '\s+')[0].ToLower()
        $actual = (Get-FileHash -Algorithm SHA256 $binaryPath).Hash.ToLower()
        
        if ($expected -ne $actual) {
            Write-Status "Checksum mismatch!" -Color Red
            Write-Status "  Expected: $expected"
            Write-Status "  Actual:   $actual"
            exit 1
        }
        
        Write-Status "Checksum verified" -Color Green
        
        # Install
        Write-Status "Installing to $InstallDir..."
        if (-not (Test-Path $InstallDir)) {
            New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
        }
        
        $destPath = Join-Path $InstallDir "odd-dashboard.exe"
        Move-Item $binaryPath $destPath -Force
    }
    finally {
        # Cleanup temp directory
        if (Test-Path $tempDir) {
            Remove-Item $tempDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

function Verify-Installation {
    $exePath = Join-Path $InstallDir "odd-dashboard.exe"
    
    if (Test-Path $exePath) {
        Write-Status ""
        Write-Status "Successfully installed odd-dashboard!" -Color Green
        Write-Status ""
        
        & $exePath --version
        
        Write-Status ""
        
        # Check if install dir is in PATH
        $envPath = [Environment]::GetEnvironmentVariable("Path", "User")
        if ($envPath -notlike "*$InstallDir*") {
            Write-Status "Note: $InstallDir is not in your PATH" -Color Yellow
            Write-Status ""
            Write-Status "To add it, run:"
            Write-Status ""
            Write-Status "  `$env:Path += `";$InstallDir`""
            Write-Status "  [Environment]::SetEnvironmentVariable('Path', `$env:Path + ';$InstallDir', 'User')"
            Write-Status ""
        }
        
        Write-Status "Run 'odd-dashboard doctor' to check prerequisites."
    }
    else {
        Write-Status "Installation failed" -Color Red
        exit 1
    }
}

# Main
function Main {
    Write-Status "odd-dashboard installer"
    Write-Status "======================="
    Write-Status ""
    Write-Status "Platform: windows-x64"
    
    Resolve-Version
    Download-AndVerify
    Verify-Installation
}

Main
