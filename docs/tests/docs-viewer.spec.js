// @ts-check
const { test, expect } = require('@playwright/test');

/**
 * Docs Viewer E2E Tests
 * 
 * Tests key functionality of the AI Assessment Experiment Viewer:
 * A. Open article - markdown file loads and renders
 * B. Comparison mode - toggle compare and dual panes work
 * C. Comparison mobile - vertical stacking on small viewports
 * D. URL hash navigation - deep linking works
 */

test.describe('Docs Viewer', () => {

    test.beforeEach(async ({ page }) => {
        await page.goto('/');
    });

    // ============================================================
    // Test A: Open Article
    // ============================================================
    test('A: should load and render a markdown article', async ({ page }) => {
        // Navigate directly to a markdown file (more reliable than UI navigation)
        const testFile = 'experiment/control-groups/dapr/dapr-claude-opus-assessment-2025-12-27.md';
        await page.goto(`/#${testFile}`);

        // Wait for content to load
        const contentBody = page.locator('#content-primary .content-body');
        await expect(contentBody.locator('.markdown-body')).toBeVisible({ timeout: 10000 });

        // Verify file path is shown in header
        const filePath = page.locator('#content-primary .file-path-slot');
        await expect(filePath).toContainText('dapr-claude-opus-assessment');

        // Verify markdown was actually parsed (has headings or paragraphs)
        await expect(contentBody.locator('.markdown-body h1, .markdown-body h2, .markdown-body p').first()).toBeVisible();
    });

    // ============================================================
    // Test B: Comparison Mode
    // ============================================================
    test('B: should toggle comparison mode and show dual panes', async ({ page }) => {
        // Initially, secondary pane should not be visible
        const secondaryPane = page.locator('.pane.secondary');
        await expect(secondaryPane).not.toBeVisible();

        // Click Compare button
        const compareBtn = page.locator('#compare-btn');
        await compareBtn.click();

        // Secondary pane should now be visible
        await expect(secondaryPane).toBeVisible();

        // Compare button should be active
        await expect(compareBtn).toHaveClass(/active/);

        // Secondary tree should be populated
        await expect(page.locator('#tree-secondary')).not.toBeEmpty();

        // Click Compare button again to exit
        await compareBtn.click();

        // Secondary pane should be hidden again
        await expect(secondaryPane).not.toBeVisible();
    });

    // ============================================================
    // Test C: Comparison Mobile
    // ============================================================
    test('C: should stack panes vertically on mobile in compare mode', async ({ page }) => {
        // Set mobile viewport
        await page.setViewportSize({ width: 375, height: 667 });

        // Enter compare mode
        const compareBtn = page.locator('#compare-btn');
        await compareBtn.click();

        // Both panes should be visible
        const primaryPane = page.locator('.pane.primary');
        const secondaryPane = page.locator('.pane.secondary');
        await expect(primaryPane).toBeVisible();
        await expect(secondaryPane).toBeVisible();

        // Viewer should have compare-mode class
        const viewer = page.locator('#viewer');
        await expect(viewer).toHaveClass(/compare-mode/);

        // Per-pane nav buttons should be visible on mobile compare mode
        const primaryNavBtn = page.locator('#nav-btn-primary');
        const secondaryNavBtn = page.locator('#nav-btn-secondary');
        await expect(primaryNavBtn).toBeVisible();
        await expect(secondaryNavBtn).toBeVisible();

        // Check that panes are stacked (secondary pane should be below primary)
        const primaryBox = await primaryPane.boundingBox();
        const secondaryBox = await secondaryPane.boundingBox();

        expect(primaryBox).not.toBeNull();
        expect(secondaryBox).not.toBeNull();

        if (primaryBox && secondaryBox) {
            // Secondary pane top should be at or after primary pane bottom (stacked)
            expect(secondaryBox.y).toBeGreaterThanOrEqual(primaryBox.y + primaryBox.height - 5);
        }
    });

    // ============================================================
    // Test D: URL Hash Navigation
    // ============================================================
    test('D: should load file from URL hash (deep linking)', async ({ page }) => {
        // Navigate directly to a specific file via hash
        const testFilePath = 'experiment/experiment.md';
        await page.goto(`/#${testFilePath}`);

        // Wait for content to load
        const contentBody = page.locator('#content-primary .content-body');
        await expect(contentBody.locator('.markdown-body')).toBeVisible({ timeout: 10000 });

        // Verify file path is shown correctly
        const filePath = page.locator('#content-primary .file-path-slot');
        await expect(filePath).toContainText('experiment.md');

        // Verify URL hash is preserved
        const url = page.url();
        expect(url).toContain(testFilePath);
    });

    // ============================================================
    // Test D2: URL Hash with Compare Mode
    // ============================================================
    test('D2: should load compare mode from URL hash', async ({ page }) => {
        // First, navigate to primary file and enable compare mode via UI
        const primaryFile = 'experiment/experiment.md';
        await page.goto(`/#${primaryFile}`);

        // Wait for primary to load
        await expect(page.locator('#content-primary .markdown-body')).toBeVisible({ timeout: 10000 });

        // Enable compare mode via button click
        const compareBtn = page.locator('#compare-btn');
        await compareBtn.click();

        // Verify compare mode is active
        const viewer = page.locator('#viewer');
        await expect(viewer).toHaveClass(/compare-mode/);

        // Secondary tree should be populated
        const secondaryTree = page.locator('#tree-secondary');
        await expect(secondaryTree).not.toBeEmpty();

        // Click on 'experiment.pdf' which is a top-level file (always visible)
        const secondaryFile = secondaryTree.locator('.file-link', { hasText: 'experiment.pdf' });
        await expect(secondaryFile).toBeVisible({ timeout: 5000 });
        await secondaryFile.click();

        // Wait for secondary pane to have PDF content
        await expect(page.locator('#content-secondary .pdf-viewer')).toBeVisible({ timeout: 10000 });

        // Both panes should have content
        await expect(page.locator('#content-primary .markdown-body')).toBeVisible();
    });

});
