# Release Checklist

This checklist must be completed before the first production release.

## Secret Configuration Audit

### 1. Repository-Level Secrets (Settings → Secrets → Actions)

These secrets MUST NOT exist at repository level:

- [ ] `WINDOWS_SIGNING_CERT` - NOT present
- [ ] `WINDOWS_SIGNING_PASSWORD` - NOT present  
- [ ] `APPLE_ID` - NOT present
- [ ] `APPLE_APP_PASSWORD` - NOT present
- [ ] `APPLE_TEAM_ID` - NOT present
- [ ] `APPLE_SIGNING_CERT` - NOT present
- [ ] `APPLE_SIGNING_PASSWORD` - NOT present
- [ ] `GPG_PRIVATE_KEY` - NOT present
- [ ] `GPG_PASSPHRASE` - NOT present

### 2. Environment-Level Secrets (Settings → Environments → release)

These secrets MUST exist ONLY under the `release` environment:

- [ ] `WINDOWS_SIGNING_CERT` - present (base64 .pfx)
- [ ] `WINDOWS_SIGNING_PASSWORD` - present
- [ ] `APPLE_ID` - present
- [ ] `APPLE_APP_PASSWORD` - present (app-specific password)
- [ ] `APPLE_TEAM_ID` - present
- [ ] `APPLE_SIGNING_CERT` - present (base64 .p12)
- [ ] `APPLE_SIGNING_PASSWORD` - present
- [ ] `GPG_PRIVATE_KEY` - present (ASCII-armored)
- [ ] `GPG_PASSPHRASE` - present

### 3. Environment Protection Rules

- [ ] `release` environment exists
- [ ] `release` environment requires 2+ reviewers
- [ ] `release` environment limited to `refs/tags/v*` only
- [ ] `release` environment does NOT include `main` or any branch

### 4. Workflow Audit

Run the audit script to verify:

```powershell
pwsh ./scripts/audit-workflows.ps1
```

- [ ] Only `release.yml` references `environment: release`
- [ ] No other workflow file contains `environment: release`
- [ ] `ci.yml` contains secret isolation verification job

## Pre-Release Validation

Before tagging a release:

1. [ ] Run all audit scripts:
   ```powershell
   pwsh ./scripts/audit-naming-consistency.ps1
   pwsh ./scripts/verify-version-sync.ps1
   pwsh ./scripts/verify-artifact-names.ps1
   pwsh ./scripts/audit-workflows.ps1
   ```

2. [ ] Verify VERSION file matches intended release version

3. [ ] Ensure CHANGELOG.md is updated

4. [ ] Create and push tag:
   ```bash
   git tag -a v0.1.0 -m "Release v0.1.0"
   git push origin v0.1.0
   ```

## Post-Release Validation

After the release workflow completes:

1. [ ] All platform binaries present in release assets
2. [ ] SHA256SUMS file present
3. [ ] SHA256SUMS.sig file present (GPG signature)
4. [ ] Download and verify each binary checksum
5. [ ] Test installation on each platform
