/**
 * ODTO Web Terminal - xterm.js client with WebSocket connection
 * 
 * Features:
 * - WebSocket connection to PTY server
 * - Session management with reconnect tokens (R2)
 * - Auto-reconnect on disconnect (R6)
 * - Terminal resize handling (R8)
 * - Fallback dashboard when WS unavailable (R10)
 */

// Configuration
const CONFIG = {
    wsUrl: `${location.protocol === 'https:' ? 'wss:' : 'ws:'}//${location.host}/ws`,
    reconnectDelay: 1000,
    reconnectMaxDelay: 30000,
    reconnectBackoffFactor: 1.5,
    resizeDebounceMs: 100,
    statsUrl: '/api/stats',
    authToken: null, // Set if auth required
};

// State
let terminal = null;
let fitAddon = null;
let ws = null;
let sessionId = null;
let reconnectToken = null;
let reconnectAttempts = 0;
let intentionalClose = false;
let resizeTimeout = null;

/**
 * Initialize the terminal
 */
function initTerminal() {
    // Create terminal with TUI-matching theme (R7)
    terminal = new Terminal({
        cursorBlink: true,
        fontFamily: '"Cascadia Code", "Fira Code", "JetBrains Mono", monospace',
        fontSize: 14,
        lineHeight: 1.2,
        theme: {
            background: 'rgba(15, 12, 41, 0.95)',
            foreground: '#ffffff',
            cursor: '#00ff88',
            cursorAccent: '#0f0c29',
            black: '#0f0c29',
            red: '#ff4757',
            green: '#00ff88',
            yellow: '#ffd700',
            blue: '#00d4ff',
            magenta: '#ff6b9d',
            cyan: '#00d4ff',
            white: '#ffffff',
            brightBlack: '#666699',
            brightRed: '#ff6b81',
            brightGreen: '#7bed9f',
            brightYellow: '#ffda79',
            brightBlue: '#70a1ff',
            brightMagenta: '#ff85a2',
            brightCyan: '#70d9ff',
            brightWhite: '#ffffff',
        },
    });

    // Load addons
    fitAddon = new FitAddon.FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.loadAddon(new WebLinksAddon.WebLinksAddon());

    // Open terminal in container
    const container = document.getElementById('terminal');
    terminal.open(container);
    
    // Fit to container
    setTimeout(() => fitAddon.fit(), 0);

    // Handle resize (R8: debounced)
    window.addEventListener('resize', handleResize);

    // Handle input
    terminal.onData((data) => {
        if (ws && ws.readyState === WebSocket.OPEN) {
            ws.send(JSON.stringify({ type: 'input', data }));
        }
    });

    // Connect to WebSocket
    connect();
}

/**
 * Handle window resize (R8)
 */
function handleResize() {
    clearTimeout(resizeTimeout);
    resizeTimeout = setTimeout(() => {
        if (fitAddon) {
            fitAddon.fit();
            if (ws && ws.readyState === WebSocket.OPEN) {
                ws.send(JSON.stringify({
                    type: 'resize',
                    cols: terminal.cols,
                    rows: terminal.rows,
                }));
            }
        }
    }, CONFIG.resizeDebounceMs);
}

/**
 * Connect to WebSocket server
 */
function connect() {
    // Build URL with reconnect params if available
    let url = CONFIG.wsUrl;
    if (sessionId && reconnectToken) {
        url += `?session=${sessionId}&token=${encodeURIComponent(reconnectToken)}`;
    }

    updateConnectionStatus('connecting');

    try {
        // Create WebSocket with auth header if configured (R5)
        if (CONFIG.authToken) {
            ws = new WebSocket(url, [], {
                headers: { 'Authorization': `Bearer ${CONFIG.authToken}` }
            });
        } else {
            ws = new WebSocket(url);
        }
    } catch (e) {
        console.error('WebSocket creation failed:', e);
        showFallback();
        return;
    }

    ws.onopen = () => {
        console.log('WebSocket connected');
        reconnectAttempts = 0;
        updateConnectionStatus('connected');
        showTerminal();
        
        // Send initial resize
        setTimeout(() => {
            if (fitAddon) {
                fitAddon.fit();
                ws.send(JSON.stringify({
                    type: 'resize',
                    cols: terminal.cols,
                    rows: terminal.rows,
                }));
            }
        }, 100);
    };

    ws.onmessage = (event) => {
        try {
            const msg = JSON.parse(event.data);
            handleMessage(msg);
        } catch (e) {
            console.error('Failed to parse message:', e);
        }
    };

    ws.onclose = (event) => {
        console.log('WebSocket closed:', event.code, event.reason);
        updateConnectionStatus('disconnected');
        
        if (!intentionalClose) {
            scheduleReconnect();
        }
    };

    ws.onerror = (error) => {
        console.error('WebSocket error:', error);
    };
}

/**
 * Handle server messages
 */
function handleMessage(msg) {
    switch (msg.type) {
        case 'session':
        case 'reconnected':
            sessionId = msg.sessionId;
            reconnectToken = msg.reconnectToken;
            console.log(`Session: ${sessionId} (${msg.type})`);
            // Store in sessionStorage for refresh
            try {
                sessionStorage.setItem('pty_session', sessionId);
                sessionStorage.setItem('pty_token', reconnectToken);
            } catch (e) {
                // Ignore storage errors
            }
            break;
            
        case 'output':
            terminal.write(msg.data);
            break;
            
        case 'notice':
            // Show notice in terminal (R4: read-only mode)
            terminal.write(`\r\n\x1b[33m${msg.message}\x1b[0m\r\n`);
            break;
            
        case 'pong':
            // Keepalive response, ignore
            break;
            
        case 'error':
            console.error('Server error:', msg.code, msg.message);
            if (msg.code === 'GLOBAL_CAP' || msg.code === 'PER_IP_CAP') {
                terminal.write(`\r\n\x1b[31mError: ${msg.message}\x1b[0m\r\n`);
                terminal.write(`\x1b[33mToo many sessions. Please close other tabs or try again later.\x1b[0m\r\n`);
            } else if (msg.code === 'INVALID_TOKEN' || msg.code === 'SESSION_NOT_FOUND') {
                // Clear stale session, reconnect fresh
                sessionId = null;
                reconnectToken = null;
                sessionStorage.removeItem('pty_session');
                sessionStorage.removeItem('pty_token');
                scheduleReconnect();
            }
            break;
    }
}

/**
 * Schedule reconnection with exponential backoff (R6)
 */
function scheduleReconnect() {
    const delay = Math.min(
        CONFIG.reconnectDelay * Math.pow(CONFIG.reconnectBackoffFactor, reconnectAttempts),
        CONFIG.reconnectMaxDelay
    );
    reconnectAttempts++;
    
    console.log(`Reconnecting in ${delay}ms (attempt ${reconnectAttempts})`);
    updateConnectionStatus('connecting');
    
    setTimeout(() => {
        if (!intentionalClose) {
            connect();
        }
    }, delay);
}

/**
 * Update connection status indicator
 */
function updateConnectionStatus(status) {
    let indicator = document.querySelector('.connection-status');
    if (!indicator) {
        indicator = document.createElement('div');
        indicator.className = 'connection-status';
        document.body.appendChild(indicator);
    }
    
    indicator.className = `connection-status ${status}`;
    
    const labels = {
        connected: 'Connected',
        disconnected: 'Disconnected',
        connecting: 'Connecting...',
    };
    
    indicator.innerHTML = `
        <span class="status-dot ${status === 'connecting' ? 'pulse' : ''}"></span>
        <span>${labels[status]}</span>
    `;
}

/**
 * Show terminal, hide fallback
 */
function showTerminal() {
    document.getElementById('terminal-container').style.display = 'flex';
    document.getElementById('fallback-container').style.display = 'none';
}

/**
 * Show fallback dashboard (R10)
 */
function showFallback() {
    document.getElementById('terminal-container').style.display = 'none';
    document.getElementById('fallback-container').style.display = 'flex';
    
    // Load stats via HTTP
    loadFallbackStats();
}

/**
 * Load stats for fallback dashboard
 */
async function loadFallbackStats() {
    const statsContainer = document.getElementById('fallback-stats');
    
    try {
        const response = await fetch(CONFIG.statsUrl);
        if (!response.ok) throw new Error('Stats unavailable');
        
        const stats = await response.json();
        
        statsContainer.innerHTML = `
            <div class="stat-grid">
                <div class="stat-item">
                    <div class="stat-value">${stats.total_jobs || 0}</div>
                    <div class="stat-label">Total Jobs</div>
                </div>
                <div class="stat-item">
                    <div class="stat-value">${stats.pending_jobs || 0}</div>
                    <div class="stat-label">Pending</div>
                </div>
                <div class="stat-item">
                    <div class="stat-value">${stats.completed_jobs || 0}</div>
                    <div class="stat-label">Completed</div>
                </div>
            </div>
        `;
    } catch (e) {
        statsContainer.innerHTML = `
            <div class="stat-loading">Stats unavailable</div>
        `;
    }
}

/**
 * Retry connection from fallback
 */
function retryConnection() {
    reconnectAttempts = 0;
    sessionId = null;
    reconnectToken = null;
    connect();
    showTerminal();
}

// Initialize on load
document.addEventListener('DOMContentLoaded', () => {
    // Try to restore session from storage
    try {
        sessionId = sessionStorage.getItem('pty_session');
        reconnectToken = sessionStorage.getItem('pty_token');
    } catch (e) {
        // Ignore storage errors
    }
    
    initTerminal();
    
    // Setup retry button
    document.getElementById('retry-button')?.addEventListener('click', retryConnection);
});

// Cleanup on unload
window.addEventListener('beforeunload', () => {
    intentionalClose = true;
    if (ws) {
        ws.close();
    }
});
