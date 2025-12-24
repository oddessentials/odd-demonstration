# Verifying Release Signatures

All `odd-dashboard` releases include cryptographically signed checksums.

## GPG Public Key

**Key ID**: `(to be generated)`  
**Fingerprint**: `(to be generated)`

### Obtaining the Key

**Option 1: From GitHub**
```bash
curl -fsSL https://github.com/oddessentials.gpg | gpg --import
```

**Option 2: From a keyserver**
```bash
gpg --keyserver keys.openpgp.org --recv-keys KEY_ID
```

**Option 3: From this repository**
```bash
gpg --import keys/release-signing.pub
```

## Verification Steps

### Linux

```bash
# 1. Download release files
VERSION="0.1.0"
BASE_URL="https://github.com/oddessentials/odd-demonstration/releases/download/v$VERSION"

curl -LO "$BASE_URL/SHA256SUMS"
curl -LO "$BASE_URL/SHA256SUMS.sig"
curl -LO "$BASE_URL/odd-dashboard-linux-x64"

# 2. Verify GPG signature
gpg --verify SHA256SUMS.sig SHA256SUMS
# Should show: "Good signature from..."

# 3. Verify checksum
sha256sum -c SHA256SUMS --ignore-missing
# Should show: "odd-dashboard-linux-x64: OK"

# 4. Make executable and run
chmod +x odd-dashboard-linux-x64
./odd-dashboard-linux-x64 --version
```

### macOS

```bash
# Use shasum instead of sha256sum
shasum -a 256 -c SHA256SUMS --ignore-missing
```

### Windows (PowerShell)

```powershell
# 1. Download files
$Version = "0.1.0"
$BaseUrl = "https://github.com/oddessentials/odd-demonstration/releases/download/v$Version"

Invoke-WebRequest "$BaseUrl/SHA256SUMS" -OutFile SHA256SUMS
Invoke-WebRequest "$BaseUrl/SHA256SUMS.sig" -OutFile SHA256SUMS.sig
Invoke-WebRequest "$BaseUrl/odd-dashboard-windows-x64.exe" -OutFile odd-dashboard.exe

# 2. Verify GPG signature (requires Gpg4win)
gpg --verify SHA256SUMS.sig SHA256SUMS

# 3. Verify checksum
$expected = (Get-Content SHA256SUMS | Select-String "odd-dashboard-windows-x64.exe").ToString().Split()[0]
$actual = (Get-FileHash -Algorithm SHA256 odd-dashboard.exe).Hash
if ($expected.ToLower() -eq $actual.ToLower()) {
    Write-Host "✓ Checksum OK" -ForegroundColor Green
} else {
    Write-Host "✗ CHECKSUM MISMATCH" -ForegroundColor Red
}

# 4. Run
.\odd-dashboard.exe --version
```

## Troubleshooting

### "No public key" error

Import the key first:
```bash
gpg --keyserver keys.openpgp.org --recv-keys KEY_ID
```

### "BAD signature" error

The file may have been tampered with. **Do NOT use it.**

1. Delete the downloaded files
2. Report the issue via GitHub Security Advisories
3. Wait for official response before re-downloading

### Windows: GPG not installed

Option 1: Install [Gpg4win](https://www.gpg4win.org/)

Option 2: Verify checksum only (skip signature):
```powershell
$expected = (Get-Content SHA256SUMS | Select-String "odd-dashboard-windows-x64.exe").ToString().Split()[0]
$actual = (Get-FileHash -Algorithm SHA256 odd-dashboard.exe).Hash
$expected.ToLower() -eq $actual.ToLower()
```

### Signature valid but "unknown trust"

This is normal if you haven't marked the key as trusted:
```bash
gpg --edit-key KEY_ID
gpg> trust
# Select trust level (e.g., 4 = fully)
gpg> quit
```

## Security Contact

Report security issues to: security@oddessentials.com (or via GitHub Security Advisories)
