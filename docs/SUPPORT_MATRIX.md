# Support Matrix

## Officially Supported Platforms

| Platform | Architecture | Min Version | Binary Name | Notes |
|----------|-------------|-------------|-------------|-------|
| Windows | x86_64 | Windows 10 1903+ | `odd-dashboard-windows-x64.exe` | Requires Docker Desktop |
| macOS | x86_64 (Intel) | macOS 11 (Big Sur) | `odd-dashboard-macos-x64` | |
| macOS | aarch64 (Apple Silicon) | macOS 11 (Big Sur) | `odd-dashboard-macos-arm64` | Native ARM binary |
| Linux | x86_64 | glibc 2.31+ | `odd-dashboard-linux-x64` | Ubuntu 20.04+, Debian 11+ |
| Linux | aarch64 (ARM64) | glibc 2.31+ | `odd-dashboard-linux-arm64` | Raspberry Pi 4+, AWS Graviton |

## Prerequisites

All platforms require:

| Prerequisite | Required For | Install Command |
|-------------|--------------|-----------------|
| Docker Desktop | Full mode | [docker.com/desktop](https://docker.com/products/docker-desktop) |
| PowerShell Core | Scripts | See below |
| kubectl | Full mode | See below |
| kind | Full mode | See below |

### PowerShell Core Installation

| Platform | Command |
|----------|---------|
| Windows | Pre-installed, or: `winget install Microsoft.PowerShell` |
| macOS | `brew install powershell` |
| Linux (Ubuntu/Debian) | See [Microsoft docs](https://learn.microsoft.com/en-us/powershell/scripting/install/installing-powershell-on-linux) |

### kubectl Installation

| Platform | Command |
|----------|---------|
| Windows | `winget install Kubernetes.kubectl` |
| macOS | `brew install kubectl` |
| Linux | See [kubernetes.io/docs](https://kubernetes.io/docs/tasks/tools/install-kubectl-linux/) |

### kind Installation

| Platform | Command |
|----------|---------|
| Windows | `winget install Kubernetes.kind` |
| macOS | `brew install kind` |
| Linux | See [kind.sigs.k8s.io](https://kind.sigs.k8s.io/docs/user/quick-start/#installation) |

## Demo Mode (Docker-Only)

Demo mode requires only Docker and runs without kubectl/kind:

```bash
docker compose -f docker-compose.demo.yml up
```

Access points in demo mode use alternate ports:
- Web Dashboard: http://localhost:18081
- Gateway API: http://localhost:13000
- Read Model: http://localhost:18080

## Unsupported Platforms

The following platforms are **not supported**:

- Windows on ARM (arm64)
- Linux on 32-bit architectures (x86, arm)
- FreeBSD, OpenBSD, NetBSD
- Solaris
- Container-only environments without Docker socket access

Running on unsupported platforms will result in:
```
ERROR: Unsupported platform: {os}-{arch}
See supported configurations: https://github.com/oddessentials/odd-demonstration/blob/main/docs/SUPPORT_MATRIX.md
```

## Checking Platform Support

Run the doctor command to verify your environment:

```bash
odd-dashboard doctor
```

This checks:
- Platform is in support matrix
- Docker Desktop is running
- PowerShell Core is available
- kubectl is installed
- kind is installed
- Required ports are available
