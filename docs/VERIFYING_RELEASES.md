# Verifying Release Integrity

## Bootstrap Releases (v0.1.x) — Unsigned

> [!IMPORTANT]
> **v0.1.x releases are unsigned bootstrap releases.**
> 
> - Integrity is guaranteed via **SHA256 checksums** and **HTTPS transport**.
> - Binary releases are **not code-signed** (no Authenticode, notarization, or GPG).
> - Your OS may show security prompts when running the binary — this is expected.
> - Signing will be introduced in a future release.

## Trust Model

| Layer | Guarantee | Verified By |
|-------|-----------|-------------|
| Transport | HTTPS encryption | GitHub infrastructure |
| Integrity | SHA256 checksums | Install scripts, manual verification |
| Authenticity | *Not yet implemented* | Future: code signing, GPG |

**For v0.1.x bootstrap releases:**
- Always verify checksums before running downloaded binaries.
- Install scripts (`install.sh`, `install.ps1`, npm shim) will hard-fail on checksum mismatch.
- If your workflow requires signed binaries, wait for a future signed release.

---

## Quick Verification

### Linux/macOS
```bash
VERSION="0.1.0"
BASE_URL="https://github.com/oddessentials/odd-demonstration/releases/download/v$VERSION"

# Download binary and checksums
curl -LO "$BASE_URL/odd-dashboard-linux-x64"
curl -LO "$BASE_URL/SHA256SUMS"

# Verify checksum
sha256sum -c SHA256SUMS --ignore-missing
# Expected: "odd-dashboard-linux-x64: OK"

# Run
chmod +x odd-dashboard-linux-x64
./odd-dashboard-linux-x64 --version
```

### macOS (Apple Silicon)
```bash
# Use shasum and arm64 binary
curl -LO "$BASE_URL/odd-dashboard-macos-arm64"
shasum -a 256 -c SHA256SUMS --ignore-missing
chmod +x odd-dashboard-macos-arm64
./odd-dashboard-macos-arm64 --version
```

> [!NOTE]
> macOS will show "cannot be opened because the developer cannot be verified."
> Right-click → Open, or: `xattr -d com.apple.quarantine odd-dashboard-macos-*`

### Windows (PowerShell)
```powershell
$Version = "0.1.0"
$BaseUrl = "https://github.com/oddessentials/odd-demonstration/releases/download/v$Version"

# Download
Invoke-WebRequest "$BaseUrl/odd-dashboard-windows-x64.exe" -OutFile odd-dashboard.exe
Invoke-WebRequest "$BaseUrl/SHA256SUMS" -OutFile SHA256SUMS

# Verify checksum
$expected = (Get-Content SHA256SUMS | Select-String "odd-dashboard-windows-x64.exe").ToString().Split()[0]
$actual = (Get-FileHash -Algorithm SHA256 odd-dashboard.exe).Hash
if ($expected.ToLower() -eq $actual.ToLower()) {
    Write-Host "[OK] Checksum verified" -ForegroundColor Green
} else {
    Write-Host "[FAIL] CHECKSUM MISMATCH - DO NOT RUN" -ForegroundColor Red
    exit 1
}

# Run
.\odd-dashboard.exe --version
```

> [!NOTE]
> Windows SmartScreen may show "Windows protected your PC."
> Click "More info" → "Run anyway" (after verifying checksum).

---

## Install Script Verification

The install scripts (`install.sh`, `install.ps1`) perform automatic checksum verification:

| Failure Mode | Exit Code | Message |
|--------------|-----------|---------|
| Download failed | 1 | "Failed to download" |
| Checksum mismatch | 1 | "Checksum mismatch!" |
| Unsupported platform | 1 | "Unsupported platform: {os}-{arch}" |
| Success | 0 | "Successfully installed" |

The npm shim (`@oddessentials/odd-dashboard`) creates a `.install-failed` sentinel file on failure, which the CLI wrapper reads to provide specific error messages.

---

## Future: GPG Signing

GPG signing of checksums will be added in a future release. When available:

```bash
# Import key
gpg --import keys/release-signing.pub

# Verify signature
gpg --verify SHA256SUMS.sig SHA256SUMS

# Then verify checksum
sha256sum -c SHA256SUMS --ignore-missing
```

---

## Security Contact

Report security issues via [GitHub Security Advisories](https://github.com/oddessentials/odd-demonstration/security/advisories).
