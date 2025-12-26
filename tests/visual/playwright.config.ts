import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
    testDir: './',
    fullyParallel: false, // Terminal state tests need to be sequential
    forbidOnly: !!process.env.CI,
    retries: process.env.CI ? 2 : 0,
    workers: 1, // Single worker for deterministic screenshots
    reporter: 'html',

    use: {
        baseURL: 'http://localhost:8081',
        trace: 'on-first-retry',
        screenshot: 'only-on-failure',
        video: 'retain-on-failure',
    },

    projects: [
        {
            name: 'chromium',
            use: { ...devices['Desktop Chrome'] },
        },
    ],

    // Expect the cluster to already be running
    // Use: ./scripts/start-all.ps1 before running tests
    webServer: undefined,

    // Snapshot config for golden image comparison
    expect: {
        toHaveScreenshot: {
            // Allow small differences due to font rendering
            maxDiffPixelRatio: 0.01,
            // Animation stabilization
            animations: 'disabled',
        },
    },
});
