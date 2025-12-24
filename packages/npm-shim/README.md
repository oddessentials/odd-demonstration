# npm Shim for odd-dashboard

This package provides a convenient way to install `odd-dashboard` via npm.

## Installation

```bash
npm install -g @oddessentials/odd-dashboard
```

## How it works

The npm package is a thin wrapper that:

1. Downloads the appropriate native binary for your platform during `npm install`
2. Verifies the binary's checksum against the signed SHA256SUMS file
3. Provides a Node.js shim that forwards all arguments to the native binary

## Supported Platforms

- Windows x64
- macOS x64 (Intel)
- macOS arm64 (Apple Silicon)
- Linux x64
- Linux arm64

## Troubleshooting

### Binary not installed

If you see "odd-dashboard binary not installed" when running the command:

1. Check if your platform is supported
2. Try reinstalling: `npm install -g @oddessentials/odd-dashboard`
3. Download manually from [GitHub Releases](https://github.com/oddessentials/odd-demonstration/releases)

### Checksum verification failed

This indicates the downloaded binary may have been tampered with. Please:

1. Do NOT use the downloaded file
2. Report the issue to the maintainers
3. Download directly from GitHub Releases and verify manually

## Alternative Installation Methods

- **Linux/macOS**: `curl -fsSL https://raw.githubusercontent.com/oddessentials/odd-demonstration/main/install.sh | sh`
- **Windows**: `iwr -useb https://raw.githubusercontent.com/oddessentials/odd-demonstration/main/install.ps1 | iex`
- **Homebrew**: `brew install oddessentials/tap/odd-dashboard`
