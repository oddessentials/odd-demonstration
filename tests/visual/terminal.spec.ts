/**
 * Visual Regression Tests for ODTO Web Terminal
 * 
 * Tests xterm.js terminal rendering fidelity against golden snapshots.
 * Requires cluster to be running: ./scripts/start-all.ps1
 */

import { test, expect } from '@playwright/test';

/**
 * Bundle Smoke Tests - Deterministic tests that don't require live WebSocket
 * These tests verify the bundle is correctly served and the page loads without errors.
 */
test.describe('Bundle Smoke Tests', () => {
    // Close WebSocket connections to free session slots (prevents limit exhaustion)
    test.afterEach(async ({ page }) => {
        try {
            await page.evaluate(() => {
                // @ts-ignore - __odtoWs is exposed by terminal.js for test cleanup
                if (window.__odtoWs) window.__odtoWs.close(1000, 'test cleanup');
            });
        } catch {
            // Page may be closed or navigated away
        }
        await page.close();
    });

    test('no HTTP errors on page load', async ({ page }) => {
        const errors: string[] = [];
        page.on('response', (res) => {
            if (res.status() >= 400) {
                errors.push(`${res.status()} ${res.url()}`);
            }
        });
        await page.goto('/');
        await page.waitForTimeout(1000);
        expect(errors).toEqual([]);
    });

    test('connection indicator visible on load', async ({ page }) => {
        // Element exists in DOM on initial load (even in disconnected state)
        await page.goto('/');
        const status = page.locator('.connection-status');
        await expect(status).toBeVisible({ timeout: 5000 });
        // Text will be "Connecting..." initially
    });

    test('exactly one script tag referencing bundle', async ({ page }) => {
        await page.goto('/');
        const scripts = await page.locator('script[src*="bundle"]').count();
        expect(scripts).toBe(1);
    });

    test('no CDN references remain', async ({ page }) => {
        await page.goto('/');
        const html = await page.content();
        expect(html).not.toContain('cdn.jsdelivr');
        expect(html).not.toContain('terminal.js');
    });

    test('no stray terminal.js reference', async ({ page }) => {
        await page.goto('/');
        const terminalScripts = await page.locator('script[src*="terminal.js"]').count();
        expect(terminalScripts).toBe(0);
    });
});

// STAGE: Nightly/Manual - Screenshot-heavy visual regression tests
// These are isolated from CI to avoid flakiness from timing/rendering variations.
// Run manually with: npx playwright test --grep "Web Terminal Visual Tests"
test.describe.skip('Web Terminal Visual Tests', () => {
    test.beforeEach(async ({ page }) => {
        // Navigate to terminal
        await page.goto('/');

        // Wait for WebSocket connection (connection status indicator)
        await page.waitForSelector('.connection-status.connected', {
            timeout: 15000,
            state: 'visible'
        });

        // Wait for terminal to render initial content
        await page.waitForTimeout(2000);
    });

    // Deterministic teardown: close WebSocket to free session slot
    test.afterEach(async ({ page }) => {
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

    test('terminal renders with correct theme', async ({ page }) => {
        // Verify terminal container is visible
        const terminal = page.locator('#terminal');
        await expect(terminal).toBeVisible();

        // Screenshot the terminal area
        await expect(terminal).toHaveScreenshot('terminal-initial.png', {
            animations: 'disabled',
        });
    });

    test('terminal shows TUI dashboard content', async ({ page }) => {
        // Wait for TUI to fully load and display dashboard
        // The TUI shows "Distributed Task Observatory" header
        await page.waitForTimeout(3000);

        // Take full-page screenshot for overall layout
        await expect(page).toHaveScreenshot('terminal-dashboard.png', {
            fullPage: true,
            animations: 'disabled',
        });
    });

    test('connection status indicator works', async ({ page }) => {
        // Verify connection status shows "Connected"
        const status = page.locator('.connection-status');
        await expect(status).toContainText('Connected');
        await expect(status).toHaveClass(/connected/);

        // Screenshot status indicator
        await expect(status).toHaveScreenshot('connection-status-connected.png');
    });

    test('terminal resizes correctly', async ({ page }) => {
        // Initial screenshot
        const terminal = page.locator('#terminal');
        await expect(terminal).toBeVisible();

        // Resize viewport
        await page.setViewportSize({ width: 1920, height: 1080 });
        await page.waitForTimeout(500); // Wait for resize debounce

        // Screenshot after resize (larger)
        await expect(terminal).toHaveScreenshot('terminal-resized-large.png', {
            animations: 'disabled',
        });

        // Resize to smaller
        await page.setViewportSize({ width: 800, height: 600 });
        await page.waitForTimeout(500);

        // Screenshot after resize (smaller)
        await expect(terminal).toHaveScreenshot('terminal-resized-small.png', {
            animations: 'disabled',
        });
    });
});

// DISABLED: Playwright cannot intercept WebSocket connections
// page.route() and page.addInitScript() do not reliably mock WebSocket before page scripts run.
// This is a known Playwright limitation. Alternative: use a separate test mode in the server.
// See: https://github.com/microsoft/playwright/issues/15684
test.describe.skip('Fallback Dashboard', () => {
    test('shows fallback when WebSocket unavailable', async ({ page }) => {
        // Override WebSocket to always fail connection
        // page.route() doesn't intercept WebSocket connections
        await page.addInitScript(() => {
            class FakeWebSocket {
                readyState = 3; // CLOSED
                onopen: (() => void) | null = null;
                onclose: ((event: CloseEvent) => void) | null = null;
                onerror: ((event: Event) => void) | null = null;
                onmessage: (() => void) | null = null;

                constructor() {
                    setTimeout(() => {
                        if (this.onerror) {
                            this.onerror(new Event('error'));
                        }
                        if (this.onclose) {
                            this.onclose(new CloseEvent('close', { code: 1006 }));
                        }
                    }, 100);
                }

                send() { }
                close() { }
            }
            (window as any).WebSocket = FakeWebSocket;
        });

        await page.goto('/');

        // Wait for fallback to appear after connection failure
        const fallback = page.locator('#fallback-container');
        await expect(fallback).toBeVisible({ timeout: 20000 });

        // Screenshot fallback UI
        await expect(fallback).toHaveScreenshot('fallback-dashboard.png', {
            animations: 'disabled',
        });
    });

    test('retry button is visible in fallback mode', async ({ page }) => {
        // Override WebSocket to always fail connection
        await page.addInitScript(() => {
            class FakeWebSocket {
                readyState = 3;
                onopen: (() => void) | null = null;
                onclose: ((event: CloseEvent) => void) | null = null;
                onerror: ((event: Event) => void) | null = null;
                onmessage: (() => void) | null = null;

                constructor() {
                    setTimeout(() => {
                        if (this.onerror) this.onerror(new Event('error'));
                        if (this.onclose) this.onclose(new CloseEvent('close', { code: 1006 }));
                    }, 100);
                }

                send() { }
                close() { }
            }
            (window as any).WebSocket = FakeWebSocket;
        });

        await page.goto('/');

        // Wait for fallback with retry button
        const fallback = page.locator('#fallback-container');
        await expect(fallback).toBeVisible({ timeout: 20000 });

        // Verify retry button exists and is visible
        const retryButton = page.locator('#retry-button');
        await expect(retryButton).toBeVisible();
        await expect(retryButton).toHaveText('Retry Connection');
    });
});
