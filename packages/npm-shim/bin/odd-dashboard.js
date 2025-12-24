#!/usr/bin/env node
/**
 * odd-dashboard npm shim - CLI entry point
 * 
 * This wrapper script spawns the native binary with all arguments forwarded.
 * If the binary is not installed, it displays installation instructions
 * and exits with a non-zero status code.
 */

const fs = require('fs');
const path = require('path');
const { spawn } = require('child_process');

const binName = process.platform === 'win32' ? 'odd-dashboard.exe' : 'odd-dashboard';
const binPath = path.join(__dirname, binName);
const sentinelPath = path.join(__dirname, '..', '.install-failed');

// Check for installation failure sentinel
if (fs.existsSync(sentinelPath)) {
    const reason = fs.readFileSync(sentinelPath, 'utf-8').trim();

    console.error(`
╔════════════════════════════════════════════════════════════════════╗
║  odd-dashboard binary not installed                                ║
╠════════════════════════════════════════════════════════════════════╣
║  Reason: ${reason.substring(0, 54).padEnd(54)}║
║                                                                    ║
║  Manual installation options:                                      ║
║                                                                    ║
║  Linux/macOS:                                                      ║
║    curl -fsSL https://raw.githubusercontent.com/oddessentials/     ║
║      odd-demonstration/main/install.sh | sh                        ║
║                                                                    ║
║  Windows PowerShell:                                               ║
║    iwr -useb https://raw.githubusercontent.com/oddessentials/      ║
║      odd-demonstration/main/install.ps1 | iex                      ║
║                                                                    ║
║  Or download directly from GitHub Releases:                        ║
║    https://github.com/oddessentials/odd-demonstration/releases     ║
╚════════════════════════════════════════════════════════════════════╝
`);
    process.exit(1);
}

// Check if binary exists
if (!fs.existsSync(binPath)) {
    console.error(`
╔════════════════════════════════════════════════════════════════════╗
║  odd-dashboard binary not found                                    ║
╠════════════════════════════════════════════════════════════════════╣
║  Expected at: ${binPath.substring(0, 50).padEnd(50)}║
║                                                                    ║
║  Try reinstalling:                                                 ║
║    npm install -g @oddessentials/odd-dashboard                     ║
╚════════════════════════════════════════════════════════════════════╝
`);
    process.exit(1);
}

// Forward all arguments to the binary
const args = process.argv.slice(2);
const child = spawn(binPath, args, {
    stdio: 'inherit',
    windowsHide: true,
});

child.on('error', (err) => {
    console.error(`Failed to start odd-dashboard: ${err.message}`);
    process.exit(1);
});

child.on('close', (code) => {
    process.exit(code ?? 0);
});
