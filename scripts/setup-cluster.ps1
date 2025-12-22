# setup-cluster.ps1
$clusterName = "task-observatory"

# Try to find kind
$kindPath = "kind"
if (-not (Get-Command "kind" -ErrorAction SilentlyContinue)) {
    $wingetKind = Get-ChildItem -Path "$env:LOCALAPPDATA\Microsoft\WinGet\Packages" -Recurse -Filter "kind.exe" | Select-Object -First 1
    if ($wingetKind) {
        $kindPath = $wingetKind.FullName
        Write-Host "Found kind at: $kindPath"
    } else {
        Write-Error "kind is not installed."
        exit 1
    }
}

# Check if cluster already exists
$clusters = & $kindPath get clusters
if ($clusters -contains $clusterName) {
    Write-Host "Cluster '$clusterName' already exists. Skipping creation."
} else {
    Write-Host "Creating kind cluster '$clusterName'..."
    
    # Kind config with ingress extraPortMappings
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

    $kindConfig | Out-File -FilePath "kind-config.yaml" -Encoding utf8
    & $kindPath create cluster --name $clusterName --config kind-config.yaml
    Remove-Item "kind-config.yaml"
}

# Set context
kubectl cluster-info --context kind-$clusterName

Write-Host "Cluster setup complete."
