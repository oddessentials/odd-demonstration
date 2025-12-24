#!/usr/bin/env node
/**
 * odd-dashboard postinstall script
 * 
 * Downloads the native binary from GitHub Releases with checksum verification.
 * On failure, creates a sentinel file so the CLI wrapper can provide helpful
 * error messages instead of silently failing.
 */

const fs = require('fs');
const path = require('path');
const https = require('https');
const crypto = require('crypto');

const PLATFORM_MAP = {
    'darwin-x64': 'odd-dashboard-macos-x64',
    'darwin-arm64': 'odd-dashboard-macos-arm64',
    'linux-x64': 'odd-dashboard-linux-x64',
    'linux-arm64': 'odd-dashboard-linux-arm64',
    'win32-x64': 'odd-dashboard-windows-x64.exe',
};

const VERSION = require('../package.json').version;
const REPO = 'oddessentials/odd-demonstration';
const BASE_URL = `https://github.com/${REPO}/releases/download/v${VERSION}`;

const binDir = path.join(__dirname, '..', 'bin');
const sentinelPath = path.join(__dirname, '..', '.install-failed');

/**
 * Download a file from a URL, following redirects
 */
function download(url) {
    return new Promise((resolve, reject) => {
        const request = (url) => {
            https.get(url, (res) => {
                if (res.statusCode === 301 || res.statusCode === 302) {
                    // Follow redirect
                    request(res.headers.location);
                } else if (res.statusCode !== 200) {
                    reject(new Error(`HTTP ${res.statusCode}: ${url}`));
                } else {
                    const chunks = [];
                    res.on('data', (chunk) => chunks.push(chunk));
                    res.on('end', () => resolve(Buffer.concat(chunks)));
                    res.on('error', reject);
                }
            }).on('error', reject);
        };
        request(url);
    });
}

/**
 * Parse checksum file and find the expected hash for an artifact
 */
function parseChecksum(content, artifact) {
    const lines = content.toString().split('\n');
    for (const line of lines) {
        const [hash, filename] = line.trim().split(/\s+/);
        if (filename === artifact) {
            return hash;
        }
    }
    throw new Error(`Artifact ${artifact} not found in SHA256SUMS`);
}

/**
 * Write failure sentinel file
 */
function writeSentinel(reason) {
    fs.writeFileSync(sentinelPath, reason);
}

/**
 * Remove failure sentinel if it exists
 */
function clearSentinel() {
    if (fs.existsSync(sentinelPath)) {
        fs.unlinkSync(sentinelPath);
    }
}

async function install() {
    const platform = `${process.platform}-${process.arch}`;
    const artifact = PLATFORM_MAP[platform];

    if (!artifact) {
        const msg = `Unsupported platform: ${platform}`;
        console.log(`[odd-dashboard] ${msg}`);
        console.log('[odd-dashboard] Binary not installed. Run `odd-dashboard` for manual installation options.');
        writeSentinel(msg);
        return; // Exit 0 to not break npm install
    }

    const binName = process.platform === 'win32' ? 'odd-dashboard.exe' : 'odd-dashboard';
    const binPath = path.join(binDir, binName);

    try {
        // Ensure bin directory exists
        fs.mkdirSync(binDir, { recursive: true });

        console.log(`[odd-dashboard] Downloading ${artifact}...`);

        // Download checksum file
        const checksums = await download(`${BASE_URL}/SHA256SUMS`);
        const expectedHash = parseChecksum(checksums, artifact);

        // Download binary
        const binary = await download(`${BASE_URL}/${artifact}`);

        // Verify checksum
        const actualHash = crypto.createHash('sha256').update(binary).digest('hex');
        if (actualHash.toLowerCase() !== expectedHash.toLowerCase()) {
            throw new Error(`Checksum mismatch! Expected: ${expectedHash}, Actual: ${actualHash}`);
        }
        console.log('[odd-dashboard] Checksum verified');

        // Write binary
        fs.writeFileSync(binPath, binary);

        // Set executable permission on POSIX
        if (process.platform !== 'win32') {
            fs.chmodSync(binPath, 0o755);
        }

        console.log(`[odd-dashboard] Installed to ${binPath}`);
        clearSentinel();

    } catch (err) {
        const msg = err.message || String(err);
        console.error(`[odd-dashboard] Installation failed: ${msg}`);
        console.error('[odd-dashboard] Run `odd-dashboard` for manual installation options.');
        writeSentinel(msg);
        // Exit 0 to not break npm install - the CLI wrapper will handle the error
    }
}

install();
