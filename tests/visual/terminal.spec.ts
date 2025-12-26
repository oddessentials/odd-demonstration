/**
 * Visual Regression Tests for ODTO Web Terminal
 * 
 * Tests xterm.js terminal rendering fidelity against golden snapshots.
 * Requires cluster to be running: ./scripts/start-all.ps1
 */

import { test, expect } from '@playwright/test';

test.describe('Web Terminal Visual Tests', () => {
    test.beforeEach(async ({ page }) => {
        // Navigate to terminal
        await page.goto('/');

        // Wait for WebSocket connection (connection status indicator)
        await page.waitForSelector('.connection-status.connected', {
            timeout: 30000,
            state: 'visible'
        });

        // Wait for terminal to render initial content
        await page.waitForTimeout(2000);
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

test.describe('Fallback Dashboard', () => {
    test('shows fallback when WebSocket unavailable', async ({ page }) => {
        // Block WebSocket connections
        await page.route('**/ws', route => route.abort());

        await page.goto('/');

        // Wait for fallback to appear after connection timeout
        const fallback = page.locator('#fallback-container');
        await expect(fallback).toBeVisible({ timeout: 15000 });

        // Screenshot fallback UI
        await expect(fallback).toHaveScreenshot('fallback-dashboard.png', {
            animations: 'disabled',
        });
    });

    test('retry button reconnects', async ({ page }) => {
        // Start with blocked WebSocket
        let blockWs = true;
        await page.route('**/ws', route => {
            if (blockWs) {
                route.abort();
            } else {
                route.continue();
            }
        });

        await page.goto('/');

        // Wait for fallback
        const fallback = page.locator('#fallback-container');
        await expect(fallback).toBeVisible({ timeout: 15000 });

        // Unblock WebSocket
        blockWs = false;

        // Click retry
        await page.click('#retry-button');

        // Wait for terminal to connect
        await page.waitForSelector('.connection-status.connected', {
            timeout: 30000
        });

        // Terminal should be visible now
        const terminal = page.locator('#terminal-container');
        await expect(terminal).toBeVisible();
    });
});
