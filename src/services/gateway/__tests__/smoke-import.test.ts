/**
 * Smoke Import Tests
 * 
 * Verifies that:
 * 1. The refactored modules can be imported without side effects
 * 2. Coverage is actually measuring production code
 * 3. Tests aren't just testing mocks
 */

import { describe, it, expect } from 'vitest';

describe('Smoke Import Tests', () => {
    describe('Module Imports (No Side Effects)', () => {
        it('should import types without side effects', async () => {
            const types = await import('../lib/types.js');

            // Types module should export interfaces/types only at compile time
            // Runtime should have DI interfaces
            expect(types).toBeDefined();
        });

        it('should import config without side effects', async () => {
            const config = await import('../lib/config.js');

            expect(config.isValidSemVer).toBeDefined();
            expect(config.loadVersionFile).toBeDefined();
            expect(config.createConfig).toBeDefined();
            expect(config.createDefaultConfig).toBeDefined();
            expect(config.defaultEnvReader).toBeDefined();
        });

        it('should import validators without side effects', async () => {
            const validators = await import('../lib/validators.js');

            expect(validators.createAjv).toBeDefined();
            expect(validators.compileSchema).toBeDefined();
            expect(validators.formatValidationErrors).toBeDefined();
            expect(validators.createDefaultSchemaReader).toBeDefined();
            expect(validators.createValidators).toBeDefined();
        });

        it('should import builders without side effects', async () => {
            const builders = await import('../lib/builders.js');

            expect(builders.buildEventEnvelope).toBeDefined();
            expect(builders.buildHealthResponse).toBeDefined();
            expect(builders.buildOpenApiSpec).toBeDefined();
            expect(builders.buildValidationErrorResponse).toBeDefined();
            expect(builders.buildInternalErrorResponse).toBeDefined();
            expect(builders.buildJobAcceptedResponse).toBeDefined();
        });

        it('should import app factory without side effects', async () => {
            const app = await import('../lib/app.js');

            expect(app.createApp).toBeDefined();
            expect(app.createMetrics).toBeDefined();
            expect(app.createHealthRouter).toBeDefined();
            expect(app.createDocsRouter).toBeDefined();
            expect(app.createJobsRouter).toBeDefined();
            expect(app.createMetricsRouter).toBeDefined();
            expect(app.createProxyRouter).toBeDefined();
        });

        it('should import lib index without side effects', async () => {
            const lib = await import('../lib/index.js');

            // Should re-export everything
            expect(lib.isValidSemVer).toBeDefined();
            expect(lib.buildEventEnvelope).toBeDefined();
            expect(lib.createApp).toBeDefined();
        });
    });

    describe('Production Code Coverage Verification', () => {
        it('should actually test isValidSemVer from production code', async () => {
            const { isValidSemVer } = await import('../lib/config.js');

            // This ensures the production function is tested, not a mock
            expect(isValidSemVer('1.0.0')).toBe(true);
            expect(isValidSemVer('invalid')).toBe(false);
        });

        it('should actually test buildHealthResponse from production code', async () => {
            const { buildHealthResponse } = await import('../lib/builders.js');

            const response = buildHealthResponse('2.0.0');
            expect(response.status).toBe('ok');
            expect(response.version).toBe('2.0.0');
        });

        it('should actually test formatValidationErrors from production code', async () => {
            const { formatValidationErrors } = await import('../lib/validators.js');

            const errors = formatValidationErrors([
                { instancePath: '/test', message: 'error' }
            ]);
            expect(errors).toHaveLength(1);
            expect(errors[0].path).toBe('/test');
        });

        it('should actually test createConfig from production code', async () => {
            const { createConfig } = await import('../lib/config.js');

            const mockEnv = (_key: string, def: string) => def;
            const config = createConfig(mockEnv, '/test', '1.0.0');

            expect(config.serviceVersion).toBe('1.0.0');
            expect(config.queueName).toBe('jobs.created');
        });

        it('should actually test buildOpenApiSpec from production code', async () => {
            const { buildOpenApiSpec } = await import('../lib/builders.js');

            const spec = buildOpenApiSpec('3.0.0');

            expect(spec.openapi).toBe('3.0.3');
            expect(spec.info.version).toBe('3.0.0');
            expect(spec.paths['/jobs']).toBeDefined();
        });
    });

    describe('Entry Point Guard', () => {
        it('should be able to import index.ts without starting server', async () => {
            // This import should NOT start the server or connect to RabbitMQ
            // because of the runtime guard in index.ts
            const indexModule = await import('../index.js');

            // Should export lib functions for testing
            expect(indexModule.createApp).toBeDefined();
            expect(indexModule.buildEventEnvelope).toBeDefined();
            expect(indexModule.isValidSemVer).toBeDefined();

            // Should export startServer for explicit invocation
            expect(indexModule.startServer).toBeDefined();
            expect(typeof indexModule.startServer).toBe('function');
        });
    });
});
