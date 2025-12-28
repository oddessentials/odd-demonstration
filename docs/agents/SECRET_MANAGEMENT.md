# Release Secret Management

## Overview

This document defines the secret management policies for `odd-dashboard` releases.
All signing secrets are scoped exclusively to the GitHub `release` environment.

## Secrets Inventory

| Secret | Purpose | Scope |
|--------|---------|-------|
| `WINDOWS_SIGNING_CERT` | Authenticode certificate (base64 .pfx) | release env only |
| `WINDOWS_SIGNING_PASSWORD` | Certificate password | release env only |
| `APPLE_ID` | Apple Developer account email | release env only |
| `APPLE_APP_PASSWORD` | App-specific password for notarization | release env only |
| `APPLE_TEAM_ID` | Apple Developer Team ID | release env only |
| `APPLE_SIGNING_CERT` | Developer ID Application cert (base64 .p12) | release env only |
| `APPLE_SIGNING_PASSWORD` | Certificate password | release env only |
| `GPG_PRIVATE_KEY` | Checksum signing key (ASCII-armored) | release env only |
| `GPG_PASSPHRASE` | GPG key passphrase | release env only |

## Scope Enforcement

Secrets are defined at the environment level, NOT repository level.

### GitHub Environment Configuration

**Environment Name**: `release`

**Deployment Branches**: 
- Select "Selected branches and tags"
- Add rule: `refs/tags/v*`
- Do NOT add `main` or any branch pattern

**Required Reviewers**: 2+ members from security team

## Rotation Schedule

| Secret Type | Rotation Frequency | Notes |
|-------------|-------------------|-------|
| Windows cert | Before expiry or annually | Check cert expiration date |
| Apple creds | Annually | App-specific passwords may expire |
| Apple cert | Before expiry (typically ~1 year) | Check cert in Keychain |
| GPG key | Every 2 years | Consider key size if upgrading |

## Rotation Procedure

### Windows Certificate

1. Obtain new certificate from CA
2. Export as .pfx with password
3. Base64 encode: `[Convert]::ToBase64String([IO.File]::ReadAllBytes('cert.pfx'))`
4. Update `WINDOWS_SIGNING_CERT` in GitHub `release` environment
5. Update `WINDOWS_SIGNING_PASSWORD` if changed
6. Test with a prerelease: `git tag v0.0.0-test.1`

### Apple Credentials

1. Generate new app-specific password at appleid.apple.com
2. Update `APPLE_APP_PASSWORD` in GitHub `release` environment
3. Test with a prerelease

### GPG Key

1. Generate new key: `gpg --full-generate-key`
2. Export private key: `gpg --export-secret-keys --armor KEY_ID`
3. Update `GPG_PRIVATE_KEY` and `GPG_PASSPHRASE`
4. Export and publish public key to `keys/release-signing.pub`
5. Update key fingerprint in `VERIFYING_RELEASES.md`
6. Update GitHub user GPG keys

## Emergency Revocation

### Windows Certificate Compromise

1. **Immediately** contact CA to revoke certificate
2. Remove `WINDOWS_SIGNING_CERT` from GitHub Secrets
3. Issue new certificate
4. Re-sign affected releases if possible
5. Post security advisory with details
6. Update rotation log below

### Apple Credentials Compromise

1. Revoke app-specific password at appleid.apple.com
2. Generate new app-specific password
3. Update `APPLE_APP_PASSWORD` in GitHub Secrets
4. No re-signing needed (credential doesn't affect binary signatures)

### GPG Key Compromise

1. Generate revocation certificate:
   ```bash
   gpg --gen-revoke KEY_ID > revoke.asc
   gpg --import revoke.asc
   gpg --keyserver keys.openpgp.org --send-keys KEY_ID
   ```
2. Post revocation notice in security advisory
3. Generate new GPG key
4. Update secrets and public key documentation
5. Re-sign `SHA256SUMS` for affected releases

## Revocation Log

| Date | Secret | Action | Reason | New Key ID |
|------|--------|--------|--------|------------|
| (none) | | | | |
