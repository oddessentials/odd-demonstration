// @ts-check
const { defineConfig, devices } = require('@playwright/test');

/**
 * Playwright configuration for Docs Viewer E2E tests.
 * Uses a local static server to serve the docs directory.
 */
module.exports = defineConfig({
    testDir: './tests',
    fullyParallel: true,
    forbidOnly: !!process.env.CI,
    retries: process.env.CI ? 2 : 0,
    workers: process.env.CI ? 1 : undefined,
    reporter: [
        ['html', { outputFolder: 'playwright-report' }],
        ['list']
    ],

    use: {
        baseURL: 'http://localhost:3000',
        trace: 'on-first-retry',
        screenshot: 'only-on-failure',
    },

    projects: [
        {
            name: 'chromium',
            use: { ...devices['Desktop Chrome'] },
        },
        {
            name: 'mobile-safari',
            use: { ...devices['iPhone 12'] },
        },
    ],

    /* Run local static server before starting tests */
    webServer: {
        command: 'npx serve -l 3000 .',
        url: 'http://localhost:3000',
        reuseExistingServer: !process.env.CI,
        timeout: 30000,
    },
});
