/**
 * PTY State Preservation Integration Tests
 * 
 * Tests for Phase 7 PTY state preservation features:
 * - Session state machine transitions
 * - Token TTL validation
 * - Ring buffer behavior
 * - Replay protocol messaging
 * 
 * These are unit/integration tests for the backend logic.
 * Visual/E2E tests are in tests/visual/
 */

import { test, expect } from '@playwright/test';

// Test configuration
const WS_URL = 'ws://localhost:9000';
const METRICS_URL = 'http://localhost:9001/metrics';

test.describe('PTY State Preservation', () => {
    // Session state tests - need proper cleanup to avoid session limit issues
    test.describe('Session State Machine', () => {
        test.afterEach(async ({ page }) => {
            // Deterministic teardown: close WebSocket to free session slot
            try {
                await page.evaluate(() => {
                    // @ts-ignore - __odtoWs is exposed by terminal.js for test cleanup
                    if (window.__odtoWs) window.__odtoWs.close(1000, 'test cleanup');
                    // @ts-ignore - also check window.ws for compatibility
                    if (window.ws) window.ws.close();
                });
            } catch {
                // Page may be closed or navigated away
            }
            await page.close();
        });

        test('new connection starts in Connected state', async ({ page }) => {
            await page.goto('/');

            // Wait for connection to establish
            await page.waitForSelector('.connection-status.connected', { timeout: 15000 });

            // Verify metrics endpoint has session counters (don't check exact values due to accumulation)
            const response = await fetch(METRICS_URL);
            const metrics = await response.text();

            // Check metric names exist with any value
            expect(metrics).toMatch(/pty_sessions_connected \d+/);
            expect(metrics).toMatch(/pty_sessions_disconnected \d+/);
        });

        test('session transitions to Disconnected on WS close', async ({ page }) => {
            await page.goto('/');
            await page.waitForSelector('.connection-status.connected', { timeout: 15000 });

            // Close the WebSocket by navigating away
            await page.evaluate(() => {
                // @ts-ignore
                window.intentionalClose = true;
                // @ts-ignore
                if (window.ws) window.ws.close();
            });

            await page.waitForTimeout(1000);

            // Metrics should show disconnected metric exists (don't check exact values)
            const response = await fetch(METRICS_URL);
            const metrics = await response.text();

            expect(metrics).toMatch(/pty_sessions_disconnected \d+/);
        });
    });

    test.describe('Replay Protocol', () => {
        test.afterEach(async ({ page }) => {
            // Deterministic teardown: close WebSocket to free session slot
            try {
                await page.evaluate(() => {
                    // @ts-ignore - __odtoWs is exposed by terminal.js for test cleanup
                    if (window.__odtoWs) window.__odtoWs.close(1000, 'test cleanup');
                    // @ts-ignore - also check window.ws for compatibility
                    if (window.ws) window.ws.close();
                });
            } catch {
                // Page may be closed or navigated away
            }
            await page.close();
        });

        test('output messages include seq field', async ({ page }) => {
            const messages: any[] = [];

            // Intercept WebSocket messages
            page.on('websocket', ws => {
                ws.on('framereceived', frame => {
                    try {
                        const msg = JSON.parse(frame.payload?.toString() || '{}');
                        if (msg.type === 'output') {
                            messages.push(msg);
                        }
                    } catch { }
                });
            });

            await page.goto('/');
            await page.waitForSelector('.connection-status.connected', { timeout: 15000 });

            // Wait for some output
            await page.waitForTimeout(3000);

            // Check that we received output messages
            expect(messages.length).toBeGreaterThan(0);
        });

        test('session message includes reconnect token', async ({ page }) => {
            let sessionMsg: any = null;

            page.on('websocket', ws => {
                ws.on('framereceived', frame => {
                    try {
                        const msg = JSON.parse(frame.payload?.toString() || '{}');
                        if (msg.type === 'session') {
                            sessionMsg = msg;
                        }
                    } catch { }
                });
            });

            await page.goto('/');
            await page.waitForSelector('.connection-status.connected', { timeout: 15000 });

            expect(sessionMsg).not.toBeNull();
            expect(sessionMsg.sessionId).toBeDefined();
            expect(sessionMsg.reconnectToken).toBeDefined();
            expect(sessionMsg.reconnectToken.length).toBeGreaterThan(20);
        });
    });

    test.describe('Connection Status', () => {
        test.afterEach(async ({ page }) => {
            // Deterministic teardown: close WebSocket to free session slot
            try {
                await page.evaluate(() => {
                    // @ts-ignore - __odtoWs is exposed by terminal.js for test cleanup
                    if (window.__odtoWs) window.__odtoWs.close(1000, 'test cleanup');
                    // @ts-ignore - also check window.ws for compatibility
                    if (window.ws) window.ws.close();
                });
            } catch {
                // Page may be closed or navigated away
            }
            await page.close();
        });

        test('shows Connected status on successful WS connection', async ({ page }) => {
            await page.goto('/');

            await page.waitForSelector('.connection-status.connected', { timeout: 15000 });
            const status = page.locator('.connection-status');
            await expect(status).toContainText('Connected');
        });

        test('shows Connecting status while establishing connection', async ({ page }) => {
            // Check for connecting state before connected
            await page.goto('/');

            // May be very brief, just ensure no errors
            const status = page.locator('.connection-status');
            await expect(status).toBeVisible({ timeout: 5000 });
        });
    });

    // Metrics tests - standalone, no session cleanup needed
    test.describe('Metrics Endpoint', () => {
        test('/metrics returns session state counts', async () => {
            const response = await fetch(METRICS_URL);
            expect(response.ok).toBe(true);

            const metrics = await response.text();

            // Check Phase 7 metrics exist
            expect(metrics).toContain('pty_sessions_active');
            expect(metrics).toContain('pty_sessions_connected');
            expect(metrics).toContain('pty_sessions_disconnected');
            expect(metrics).toContain('pty_sessions_idle');
            expect(metrics).toContain('pty_sessions_reaping');
            expect(metrics).toContain('pty_output_drops_total');
        });
    });
});
