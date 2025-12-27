# start-all.ps1 - One-click cluster startup for Distributed Task Observatory
# Usage: .\scripts\start-all.ps1
#        .\scripts\start-all.ps1 -OutputJson  (for TUI parsing)

param(
    [switch]$OutputJson,      # Output JSON for TUI parsing
    [switch]$SkipBuild,       # Skip Docker image builds
    [switch]$SkipPortForward, # Don't start port-forwards
    [int]$Timeout = 600       # Pod readiness timeout in seconds (increased for slow machines)
)

$ErrorActionPreference = "Stop"

# ============================================================================
# Hardened Root Resolution Pattern (v3.1.7)
# Handles the "$PSScriptRoot Invocation Hazard" when spawned by foreign shells
# ============================================================================

# Primary: Resolve from actual script file path
if ($PSScriptRoot) {
    # Strip Windows extended path prefix (\\?\) if present - it breaks Split-Path
    $scriptRoot = $PSScriptRoot -replace '^\\\\\?\\', ''
    $script:ProjectRoot = Split-Path -Parent $scriptRoot
} else {
    # Secondary: Fall back to CWD and walk upward to find project root
    # Note: Get-Location returns PathInfo, use .Path for string
    $script:ProjectRoot = (Get-Location).Path
    while ($script:ProjectRoot -and -not (Test-Path (Join-Path $script:ProjectRoot "infra"))) {
        $parent = Split-Path -Parent $script:ProjectRoot
        if ($parent -eq $script:ProjectRoot) { $script:ProjectRoot = $null; break }
        $script:ProjectRoot = $parent
    }
}

# Fail-fast guard: Validate project root contains the canonical marker
if (-not $script:ProjectRoot -or -not (Test-Path (Join-Path $script:ProjectRoot "infra"))) {
    # Output as JSON for TUI to capture
    if ($OutputJson) {
        Write-Output (@{ step = "prereqs"; status = "error"; message = "FATAL: Project root could not be resolved. PSScriptRoot='$PSScriptRoot' CWD='$(Get-Location)'" } | ConvertTo-Json -Compress)
    }
    Write-Host "FATAL: Project root could not be resolved." -ForegroundColor Red
    Write-Host "  PSScriptRoot: '$PSScriptRoot'" -ForegroundColor Red
    Write-Host "  CWD: '$(Get-Location)'" -ForegroundColor Red
    Write-Host "  Resolved: '$script:ProjectRoot'" -ForegroundColor Red
    Write-Host "  The 'infra/' directory was not found in any parent directory." -ForegroundColor Red
    exit 1
}

# Colors for non-JSON output
$Colors = @{
    Info    = "Cyan"
    Success = "Green"
    Warning = "Yellow"
    Error   = "Red"
    Step    = "Magenta"
}

function Write-Progress-Step {
    param(
        [string]$Step,
        [string]$Status,  # starting, complete, error, skipped
        [string]$Message
    )
    
    if ($OutputJson) {
        @{
            step    = $Step
            status  = $Status
            message = $Message
        } | ConvertTo-Json -Compress | Write-Host
    } else {
        $icon = switch ($Status) {
            "starting" { "[..]" }
            "complete" { "[OK]" }
            "error"    { "[!!]" }
            "skipped"  { "[--]" }
            default    { "[  ]" }
        }
        $color = switch ($Status) {
            "complete" { $Colors.Success }
            "error"    { $Colors.Error }
            "skipped"  { $Colors.Warning }
            default    { $Colors.Info }
        }
        Write-Host "$icon [$Step] $Message" -ForegroundColor $color
    }
}

function Test-Prerequisites {
    Write-Progress-Step -Step "prereqs" -Status "starting" -Message "Checking prerequisites..."
    
    $missing = @()
    
    # Check Docker
    try {
        $dockerInfo = docker info 2>&1
        if ($LASTEXITCODE -ne 0) {
            $missing += "Docker Desktop (not running)"
        }
    } catch {
        $missing += "Docker (not installed)"
    }
    
    # Check kind
    $kindPath = "kind"
    if (-not (Get-Command "kind" -ErrorAction SilentlyContinue)) {
        $wingetKind = Get-ChildItem -Path "$env:LOCALAPPDATA\Microsoft\WinGet\Packages" -Recurse -Filter "kind.exe" -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($wingetKind) {
            $kindPath = $wingetKind.FullName
        } else {
            $missing += "kind (Kubernetes in Docker)"
        }
    }
    
    # Check kubectl
    if (-not (Get-Command "kubectl" -ErrorAction SilentlyContinue)) {
        $missing += "kubectl"
    }
    
    if ($missing.Count -gt 0) {
        Write-Progress-Step -Step "prereqs" -Status "error" -Message "Missing: $($missing -join ', ')"
        return $false
    }
    
    Write-Progress-Step -Step "prereqs" -Status "complete" -Message "All prerequisites found"
    return $true
}

function Initialize-Cluster {
    param([string]$KindPath = "kind")
    
    $clusterName = "task-observatory"
    
    Write-Progress-Step -Step "cluster" -Status "starting" -Message "Checking cluster status..."
    
    # Check if cluster exists (suppress stderr since "No kind clusters found" goes there)
    try {
        $clusters = & $KindPath get clusters 2>&1 | Where-Object { $_ -notmatch "No kind clusters found" }
    } catch {
        $clusters = @()
    }
    
    if ($clusters -contains $clusterName) {
        Write-Progress-Step -Step "cluster" -Status "complete" -Message "Cluster '$clusterName' already exists"
        
        # Set kubectl context
        kubectl config use-context "kind-$clusterName" 2>&1 | Out-Null
        return $true
    }
    
    Write-Progress-Step -Step "cluster" -Status "starting" -Message "Creating Kind cluster '$clusterName'..."
    
    # Create kind config
    $kindConfig = @"
kind: Cluster
apiVersion: kind.x-k8s.io/v1alpha4
nodes:
- role: control-plane
  kubeadmConfigPatches:
  - |
    kind: InitConfiguration
    nodeRegistration:
      kubeletExtraArgs:
        node-labels: "ingress-ready=true"
  extraPortMappings:
  - containerPort: 80
    hostPort: 80
    protocol: TCP
  - containerPort: 443
    hostPort: 443
    protocol: TCP
"@
    
    $configPath = Join-Path $env:TEMP "kind-config-$([guid]::NewGuid().ToString().Substring(0,8)).yaml"
    $kindConfig | Out-File -FilePath $configPath -Encoding utf8
    
    try {
        # kind outputs progress to stderr, so we capture all output and check exit code
        $ErrorActionPreference = "Continue"
        $output = & $KindPath create cluster --name $clusterName --config $configPath 2>&1
        $exitCode = $LASTEXITCODE
        $ErrorActionPreference = "Stop"
        
        if ($exitCode -ne 0) {
            $errorMsg = ($output | Where-Object { $_ -is [System.Management.Automation.ErrorRecord] }) -join "; "
            if (-not $errorMsg) { $errorMsg = "Kind cluster creation failed with exit code $exitCode" }
            throw $errorMsg
        }
        Write-Progress-Step -Step "cluster" -Status "complete" -Message "Cluster '$clusterName' created"
        return $true
    } catch {
        Write-Progress-Step -Step "cluster" -Status "error" -Message $_.Exception.Message
        return $false
    } finally {
        Remove-Item $configPath -ErrorAction SilentlyContinue
    }
}

function Build-DockerImages {
    Write-Progress-Step -Step "images" -Status "starting" -Message "Building Docker images..."
    
    # Define images with their VERSION file paths
    # Context must match the COPY paths in each Dockerfile:
    #   - Gateway/Processor: Dockerfiles require repo root context for contracts/ directory
    #   - Web-pty-server: Dockerfile uses repo-root paths (e.g., COPY src/services/...)
    $images = @(
        @{ Name = "gateway"; Dockerfile = "src/services/gateway/Dockerfile"; Context = "."; VersionFile = "src/services/gateway/VERSION" }
        @{ Name = "processor"; Dockerfile = "src/services/processor/Dockerfile"; Context = "."; VersionFile = "src/services/processor/VERSION" }
        @{ Name = "metrics-engine"; Dockerfile = "src/services/metrics-engine/Dockerfile"; Context = "src/services/metrics-engine"; VersionFile = "src/services/metrics-engine/VERSION" }
        @{ Name = "read-model"; Dockerfile = "src/services/read-model/Dockerfile"; Context = "src/services/read-model"; VersionFile = "src/services/read-model/VERSION" }
        @{ Name = "web-ui"; Dockerfile = "src/interfaces/web/Dockerfile"; Context = "src/interfaces/web"; VersionFile = "src/interfaces/web/VERSION" }
        @{ Name = "web-pty-server"; Dockerfile = "src/services/web-pty-server/Dockerfile"; Context = "."; VersionFile = "src/services/web-pty-server/VERSION" }
    )
    
    $failedImages = @()
    $script:ImageVersions = @{}  # Store name:version for later use
    
    foreach ($img in $images) {
        $dockerfilePath = Join-Path $ProjectRoot $img.Dockerfile
        $contextPath = Join-Path $ProjectRoot $img.Context
        
        # Read version from VERSION file - FAIL FAST if missing (no :latest fallback)
        $version = $null
        if ($img.VersionFile) {
            $versionFilePath = Join-Path $ProjectRoot $img.VersionFile
            if (Test-Path $versionFilePath) {
                $version = (Get-Content $versionFilePath -Raw).Trim()
            } else {
                Write-Progress-Step -Step "images" -Status "error" -Message "VERSION file missing: $($img.VersionFile)"
                Write-Host "FATAL: VERSION file not found: $versionFilePath" -ForegroundColor Red
                Write-Host "  This likely indicates a broken project root resolution." -ForegroundColor Red
                Write-Host "  Current ProjectRoot: $ProjectRoot" -ForegroundColor Red
                return $false
            }
        } else {
            Write-Progress-Step -Step "images" -Status "error" -Message "Image $($img.Name) has no VersionFile configured"
            return $false
        }
        
        $imageTag = "$($img.Name):$version"
        $script:ImageVersions[$img.Name] = $version
        
        Write-Progress-Step -Step "images" -Status "starting" -Message "Building $imageTag..."
        
        # Build sequentially for more reliable error handling
        $ErrorActionPreference = "Continue"
        docker build -t $imageTag -f $dockerfilePath $contextPath 2>&1 | Out-Null
        $exitCode = $LASTEXITCODE
        $ErrorActionPreference = "Stop"
        
        if ($exitCode -ne 0) {
            $failedImages += $img.Name
            Write-Progress-Step -Step "images" -Status "error" -Message "Failed to build $imageTag"
        }
    }
    
    if ($failedImages.Count -gt 0) {
        Write-Progress-Step -Step "images" -Status "error" -Message "Failed to build: $($failedImages -join ', ')"
        return $false
    }
    
    Write-Progress-Step -Step "images" -Status "complete" -Message "All images built successfully"
    return $true
}

function Import-ImagesToKind {
    param([string]$KindPath = "kind")
    
    $clusterName = "task-observatory"
    
    # Version file mapping - authoritative source for all image versions
    $versionFiles = @{
        "gateway" = "src/services/gateway/VERSION"
        "processor" = "src/services/processor/VERSION"
        "metrics-engine" = "src/services/metrics-engine/VERSION"
        "read-model" = "src/services/read-model/VERSION"
        "web-ui" = "src/interfaces/web/VERSION"
        "web-pty-server" = "src/services/web-pty-server/VERSION"
    }
    
    Write-Progress-Step -Step "load" -Status "starting" -Message "Loading images into Kind cluster..."
    
    foreach ($imgName in $versionFiles.Keys) {
        # Read version from VERSION file - FAIL FAST if missing (no :latest fallback)
        $versionFilePath = Join-Path $ProjectRoot $versionFiles[$imgName]
        if (-not (Test-Path $versionFilePath)) {
            Write-Progress-Step -Step "load" -Status "error" -Message "VERSION file missing: $($versionFiles[$imgName])"
            Write-Host "FATAL: VERSION file not found: $versionFilePath" -ForegroundColor Red
            Write-Host "  Current ProjectRoot: $ProjectRoot" -ForegroundColor Red
            return $false
        }
        $version = (Get-Content $versionFilePath -Raw).Trim()
        $imageTag = "${imgName}:${version}"
        
        Write-Progress-Step -Step "load" -Status "starting" -Message "Loading $imageTag..."
        
        # kind load outputs progress to stderr, so use same pattern as cluster creation
        $ErrorActionPreference = "Continue"
        $output = & $KindPath load docker-image $imageTag --name $clusterName 2>&1
        $exitCode = $LASTEXITCODE
        $ErrorActionPreference = "Stop"
        
        if ($exitCode -ne 0) {
            $errorMsg = ($output | Where-Object { $_ -is [System.Management.Automation.ErrorRecord] }) -join "; "
            Write-Progress-Step -Step "load" -Status "error" -Message "Failed to load $imageTag`: $errorMsg"
            return $false
        }
    }
    
    Write-Progress-Step -Step "load" -Status "complete" -Message "All images loaded into cluster"
    return $true
}

function Deploy-Manifests {
    Write-Progress-Step -Step "deploy" -Status "starting" -Message "Deploying Kubernetes manifests..."
    
    $k8sPath = Join-Path $ProjectRoot "infra/k8s"
    
    kubectl apply -f $k8sPath 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Write-Progress-Step -Step "deploy" -Status "error" -Message "Failed to apply manifests"
        return $false
    }
    
    Write-Progress-Step -Step "deploy" -Status "complete" -Message "Manifests applied successfully"
    return $true
}

function Wait-ForPods {
    param([int]$TimeoutSeconds = 300)
    
    Write-Progress-Step -Step "pods" -Status "starting" -Message "Waiting for pods to be ready..."
    
    $startTime = Get-Date
    $allReady = $false
    
    while (-not $allReady -and ((Get-Date) - $startTime).TotalSeconds -lt $TimeoutSeconds) {
        $pods = kubectl get pods -o json 2>$null | ConvertFrom-Json
        
        if ($pods -and $pods.items) {
            $total = $pods.items.Count
            $ready = ($pods.items | Where-Object { 
                $_.status.phase -eq "Running" -and 
                ($_.status.conditions | Where-Object { $_.type -eq "Ready" -and $_.status -eq "True" })
            }).Count
            
            Write-Progress-Step -Step "pods" -Status "starting" -Message "Pods ready: $ready/$total"
            
            if ($ready -eq $total -and $total -gt 0) {
                $allReady = $true
            }
        }
        
        if (-not $allReady) {
            Start-Sleep -Seconds 5
        }
    }
    
    if ($allReady) {
        Write-Progress-Step -Step "pods" -Status "complete" -Message "All pods are ready"
        return $true
    } else {
        Write-Progress-Step -Step "pods" -Status "error" -Message "Timeout waiting for pods"
        return $false
    }
}

function Start-PortForwards {
    Write-Progress-Step -Step "ports" -Status "starting" -Message "Starting port-forwards..."
    
    $forwards = @(
        @{ Service = "gateway"; LocalPort = 3000; RemotePort = 3000 }
        @{ Service = "read-model"; LocalPort = 8080; RemotePort = 8080 }
        @{ Service = "web-ui-http"; LocalPort = 8081; RemotePort = 80 }  # R11: Single access URL
        @{ Service = "rabbitmq"; LocalPort = 15672; RemotePort = 15672 }
        @{ Service = "grafana"; LocalPort = 3002; RemotePort = 3000 }
        @{ Service = "prometheus"; LocalPort = 9090; RemotePort = 9090 }
        @{ Service = "pgadmin"; LocalPort = 5050; RemotePort = 80 }
        @{ Service = "mongo-express"; LocalPort = 8082; RemotePort = 8081 }
        @{ Service = "redisinsight"; LocalPort = 8001; RemotePort = 8001 }
    )
    
    $pids = @()
    foreach ($fwd in $forwards) {
        Write-Progress-Step -Step "ports" -Status "starting" -Message "Forwarding $($fwd.Service):$($fwd.LocalPort)..."
        
        # Use Start-Process to create persistent background processes
        $proc = Start-Process -FilePath "kubectl" `
            -ArgumentList "port-forward", "svc/$($fwd.Service)", "$($fwd.LocalPort):$($fwd.RemotePort)" `
            -WindowStyle Hidden `
            -PassThru
        
        if ($proc) {
            $pids += $proc.Id
        }
    }
    
    # Give port-forwards time to start
    Start-Sleep -Seconds 3
    
    Write-Progress-Step -Step "ports" -Status "complete" -Message "Port-forwards active ($($pids.Count) processes)"
    
    # Return process IDs for reference
    return $pids
}

function Test-Connectivity {
    param(
        [int]$MaxWaitSeconds = 15,
        [int]$PollIntervalSeconds = 2
    )
    
    Write-Progress-Step -Step "verify" -Status "starting" -Message "Verifying connectivity..."
    
    $endpoints = @(
        @{ Name = "Gateway"; Url = "http://localhost:3000/healthz" }
        @{ Name = "Read Model"; Url = "http://localhost:8080/health" }
        @{ Name = "Web Terminal"; Url = "http://localhost:8081/health" }
    )
    
    $deadline = (Get-Date).AddSeconds($MaxWaitSeconds)
    $attempt = 0
    
    while ((Get-Date) -lt $deadline) {
        $attempt++
        $allOk = $true
        $failedEndpoints = @()
        
        foreach ($ep in $endpoints) {
            try {
                $response = Invoke-WebRequest -Uri $ep.Url -TimeoutSec 3 -UseBasicParsing -ErrorAction Stop
                if ($response.StatusCode -ne 200) {
                    $allOk = $false
                    $failedEndpoints += $ep.Name
                }
            } catch {
                $allOk = $false
                $failedEndpoints += $ep.Name
            }
        }
        
        if ($allOk) {
            Write-Progress-Step -Step "verify" -Status "complete" -Message "All services reachable"
            return $true
        }
        
        # Not ready yet - wait and retry
        Write-Progress-Step -Step "verify" -Status "starting" -Message "Waiting for: $($failedEndpoints -join ', ')..."
        Start-Sleep -Seconds $PollIntervalSeconds
    }
    
    # Timed out - report which endpoints failed
    Write-Progress-Step -Step "verify" -Status "error" -Message "$($failedEndpoints -join ', ') not reachable after ${MaxWaitSeconds}s"
    return $false
}

function Show-AccessInfo {
    if (-not $OutputJson) {
        Write-Host ""
        Write-Host "============================================================" -ForegroundColor Cyan
        Write-Host "  Distributed Task Observatory is READY!" -ForegroundColor Green
        Write-Host "============================================================" -ForegroundColor Cyan
        Write-Host ""
        Write-Host "  Access Points:" -ForegroundColor Yellow
        Write-Host "  • Web Dashboard:  http://localhost:8081" -ForegroundColor White
        Write-Host "  • Gateway API:    http://localhost:3000" -ForegroundColor White
        Write-Host "    └─ API Docs:    http://localhost:3000/docs" -ForegroundColor Gray
        Write-Host "  • Read Model API: http://localhost:8080/stats" -ForegroundColor White
        Write-Host "    └─ API Docs:    http://localhost:8080/docs" -ForegroundColor Gray
        Write-Host "  • RabbitMQ:       http://localhost:15672 (guest/guest)" -ForegroundColor White
        Write-Host "  • Grafana:        http://localhost:3002 (admin/admin)" -ForegroundColor White
        Write-Host "  • Prometheus:     http://localhost:9090" -ForegroundColor White
        Write-Host ""
        Write-Host "  Database Admin UIs:" -ForegroundColor Yellow
        Write-Host "  • pgAdmin:        http://localhost:5050 (admin@example.com/admin)" -ForegroundColor White
        Write-Host "  • Mongo Express:  http://localhost:8082 (admin/password123)" -ForegroundColor White
        Write-Host "  • RedisInsight:   http://localhost:8001" -ForegroundColor White
        Write-Host ""
        Write-Host "  To run the TUI:" -ForegroundColor Yellow
        Write-Host "  cd src/interfaces/tui && cargo run --release" -ForegroundColor White
        Write-Host ""
        Write-Host "  To stop port-forwards: Get-Job | Stop-Job | Remove-Job" -ForegroundColor Gray
        Write-Host "  To delete cluster: kind delete cluster --name task-observatory" -ForegroundColor Gray
        Write-Host ""
    } else {
        @{
            step    = "ready"
            status  = "complete"
            message = "System ready"
            urls    = @{
                webDashboard  = "http://localhost:8081"
                gateway       = "http://localhost:3000"
                gatewayDocs   = "http://localhost:3000/docs"
                readModel     = "http://localhost:8080"
                readModelDocs = "http://localhost:8080/docs"
                rabbitmq      = "http://localhost:15672"
                grafana       = "http://localhost:3002"
                prometheus    = "http://localhost:9090"
                pgadmin       = "http://localhost:5050"
                mongoExpress  = "http://localhost:8082"
                redisinsight  = "http://localhost:8001"
            }
        } | ConvertTo-Json -Compress | Write-Host
    }
}

# ═══════════════════════════════════════════════════════════
#  MAIN EXECUTION
# ═══════════════════════════════════════════════════════════

if (-not $OutputJson) {
    Write-Host ""
    Write-Host "============================================================" -ForegroundColor Cyan
    Write-Host "  Distributed Task Observatory - Startup Script" -ForegroundColor Cyan
    Write-Host "============================================================" -ForegroundColor Cyan
    Write-Host ""
}

# Step 1: Prerequisites
if (-not (Test-Prerequisites)) {
    exit 1
}

# Step 2: Cluster
$kindPath = "kind"
if (-not (Get-Command "kind" -ErrorAction SilentlyContinue)) {
    $wingetKind = Get-ChildItem -Path "$env:LOCALAPPDATA\Microsoft\WinGet\Packages" -Recurse -Filter "kind.exe" -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($wingetKind) { $kindPath = $wingetKind.FullName }
}

if (-not (Initialize-Cluster -KindPath $kindPath)) {
    exit 1
}

# Step 3: Build images
if (-not $SkipBuild) {
    if (-not (Build-DockerImages)) {
        exit 1
    }
} else {
    Write-Progress-Step -Step "images" -Status "skipped" -Message "Skipping image builds"
}

# Step 4: Load images
if (-not (Import-ImagesToKind -KindPath $kindPath)) {
    exit 1
}

# Step 5: Deploy
if (-not (Deploy-Manifests)) {
    exit 1
}

# Step 6: Wait for pods
if (-not (Wait-ForPods -TimeoutSeconds $Timeout)) {
    exit 1
}

# Step 7: Port forwards
if (-not $SkipPortForward) {
    $portForwardJobs = Start-PortForwards
} else {
    Write-Progress-Step -Step "ports" -Status "skipped" -Message "Skipping port-forwards"
}

# Step 8: Verify (with built-in retry - non-fatal if it fails)
if (-not (Test-Connectivity -MaxWaitSeconds 15)) {
    Write-Progress-Step -Step "verify" -Status "warning" -Message "Some services may not be reachable yet"
}

# Step 9: Done!
Show-AccessInfo

exit 0
