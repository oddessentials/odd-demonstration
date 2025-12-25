import { defineConfig } from 'vitest/config';

export default defineConfig({
    test: {
        globals: true,
        environment: 'node',
        include: ['__tests__/**/*.test.ts'],
        coverage: {
            provider: 'v8',
            include: ['lib/**/*.ts', 'index.ts'],
            exclude: ['**/*.test.ts', '**/node_modules/**'],
            reporter: ['text', 'json', 'html'],
            // Fail if coverage drops below threshold
            thresholds: {
                statements: 80,
                branches: 70,
                functions: 80,
                lines: 80,
            },
        },
    },
});
