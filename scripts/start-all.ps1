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
$script:ProjectRoot = Split-Path -Parent $PSScriptRoot

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
    $images = @(
        @{ Name = "gateway"; Dockerfile = "src/services/gateway/Dockerfile"; Context = "."; VersionFile = "src/services/gateway/VERSION" }
        @{ Name = "processor"; Dockerfile = "src/services/processor/Dockerfile"; Context = "."; VersionFile = "src/services/processor/VERSION" }
        @{ Name = "metrics-engine"; Dockerfile = "src/services/metrics-engine/Dockerfile"; Context = "src/services/metrics-engine"; VersionFile = "src/services/metrics-engine/VERSION" }
        @{ Name = "read-model"; Dockerfile = "src/services/read-model/Dockerfile"; Context = "src/services/read-model"; VersionFile = "src/services/read-model/VERSION" }
        @{ Name = "web-ui"; Dockerfile = "src/interfaces/web/Dockerfile"; Context = "src/interfaces/web"; VersionFile = $null }
        @{ Name = "web-pty-server"; Dockerfile = "src/services/web-pty-server/Dockerfile"; Context = "src/services/web-pty-server"; VersionFile = "src/services/web-pty-server/VERSION" }
    )
    
    $failedImages = @()
    $script:ImageVersions = @{}  # Store name:version for later use
    
    foreach ($img in $images) {
        $dockerfilePath = Join-Path $ProjectRoot $img.Dockerfile
        $contextPath = Join-Path $ProjectRoot $img.Context
        
        # Read version from VERSION file, default to "latest" if not found
        $version = "latest"
        if ($img.VersionFile) {
            $versionFilePath = Join-Path $ProjectRoot $img.VersionFile
            if (Test-Path $versionFilePath) {
                $version = (Get-Content $versionFilePath -Raw).Trim()
            }
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
    $images = @("gateway", "processor", "metrics-engine", "read-model", "web-ui", "web-pty-server")
    
    Write-Progress-Step -Step "load" -Status "starting" -Message "Loading images into Kind cluster..."
    
    foreach ($img in $images) {
        # Get version from the build step, default to "latest"
        $version = if ($script:ImageVersions -and $script:ImageVersions[$img]) { $script:ImageVersions[$img] } else { "latest" }
        $imageTag = "${img}:${version}"
        
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
    Write-Progress-Step -Step "verify" -Status "starting" -Message "Verifying connectivity..."
    
    $endpoints = @(
        @{ Name = "Gateway"; Url = "http://localhost:3000/healthz" }
        @{ Name = "Read Model"; Url = "http://localhost:8080/health" }
        @{ Name = "Web Terminal"; Url = "http://localhost:8081/health" }
    )
    
    $allOk = $true
    foreach ($ep in $endpoints) {
        try {
            $response = Invoke-WebRequest -Uri $ep.Url -TimeoutSec 5 -UseBasicParsing -ErrorAction Stop
            if ($response.StatusCode -ne 200) {
                $allOk = $false
                Write-Progress-Step -Step "verify" -Status "error" -Message "$($ep.Name) returned $($response.StatusCode)"
            }
        } catch {
            $allOk = $false
            Write-Progress-Step -Step "verify" -Status "error" -Message "$($ep.Name) not reachable"
        }
    }
    
    if ($allOk) {
        Write-Progress-Step -Step "verify" -Status "complete" -Message "All services reachable"
    }
    
    return $allOk
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

# Step 8: Verify
Start-Sleep -Seconds 2  # Give port-forwards a moment
if (-not (Test-Connectivity)) {
    Write-Progress-Step -Step "verify" -Status "warning" -Message "Some services may not be ready yet"
}

# Step 9: Done!
Show-AccessInfo

exit 0
