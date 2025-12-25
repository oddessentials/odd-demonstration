/**
 * Config Unit Tests
 * 
 * Tests for configuration module with dependency injection.
 */

import { describe, it, expect, vi } from 'vitest';
import {
    isValidSemVer,
    loadVersionFile,
    createConfig,
    createDefaultConfig,
    defaultEnvReader,
} from '../lib/config.js';

describe('Config', () => {
    describe('isValidSemVer', () => {
        it('should accept valid semver formats', () => {
            expect(isValidSemVer('1.0.0')).toBe(true);
            expect(isValidSemVer('0.1.0')).toBe(true);
            expect(isValidSemVer('10.20.30')).toBe(true);
            expect(isValidSemVer('0.0.1')).toBe(true);
        });

        it('should reject invalid semver formats', () => {
            expect(isValidSemVer('1.0')).toBe(false);
            expect(isValidSemVer('v1.0.0')).toBe(false);
            expect(isValidSemVer('1.0.0-alpha')).toBe(false);
            expect(isValidSemVer('1.0.0.0')).toBe(false);
            expect(isValidSemVer('')).toBe(false);
            expect(isValidSemVer('abc')).toBe(false);
        });
    });

    describe('loadVersionFile', () => {
        it('should load and validate version from file', () => {
            const mockReader = vi.fn().mockReturnValue('1.2.3\n');

            const version = loadVersionFile('/path/to/VERSION', mockReader);

            expect(mockReader).toHaveBeenCalledWith('/path/to/VERSION');
            expect(version).toBe('1.2.3');
        });

        it('should trim whitespace from version', () => {
            const mockReader = vi.fn().mockReturnValue('  2.0.0  \n');

            const version = loadVersionFile('/path/to/VERSION', mockReader);

            expect(version).toBe('2.0.0');
        });

        it('should throw on invalid semver', () => {
            const mockReader = vi.fn().mockReturnValue('v1.0.0');

            expect(() => loadVersionFile('/path/to/VERSION', mockReader))
                .toThrow('Invalid SemVer format: v1.0.0');
        });

        it('should throw on empty version', () => {
            const mockReader = vi.fn().mockReturnValue('');

            expect(() => loadVersionFile('/path/to/VERSION', mockReader))
                .toThrow('Invalid SemVer format: ');
        });
    });

    describe('createConfig', () => {
        it('should create config with default values', () => {
            const mockEnv = vi.fn().mockImplementation((_key, defaultValue) => defaultValue);

            const config = createConfig(mockEnv, '/app', '1.0.0');

            expect(config.rabbitmqUrl).toBe('amqp://guest:guest@rabbitmq:5672');
            expect(config.queueName).toBe('jobs.created');
            expect(config.port).toBe(3000);
            expect(config.alertmanagerUrl).toBe('http://alertmanager:9093');
            expect(config.prometheusUrl).toBe('http://prometheus:9090');
            expect(config.proxyTimeoutMs).toBe(5000);
            expect(config.contractsPath).toBe('/app/../../../contracts');
            expect(config.serviceVersion).toBe('1.0.0');
        });

        it('should use environment values when provided', () => {
            const mockEnv = vi.fn().mockImplementation((key) => {
                const envMap: Record<string, string> = {
                    'RABBITMQ_URL': 'amqp://custom:5672',
                    'PORT': '8080',
                    'ALERTMANAGER_URL': 'http://custom-alertmanager:9093',
                    'PROMETHEUS_URL': 'http://custom-prometheus:9090',
                    'CONTRACTS_PATH': '/custom/contracts',
                };
                return envMap[key] || '';
            });

            const config = createConfig(mockEnv, '/app', '2.0.0');

            expect(config.rabbitmqUrl).toBe('amqp://custom:5672');
            expect(config.port).toBe(8080);
            expect(config.alertmanagerUrl).toBe('http://custom-alertmanager:9093');
            expect(config.prometheusUrl).toBe('http://custom-prometheus:9090');
            expect(config.contractsPath).toBe('/custom/contracts');
        });

        it('should parse port as integer', () => {
            const mockEnv = vi.fn().mockImplementation((key, defaultValue) => {
                return key === 'PORT' ? '9000' : defaultValue;
            });

            const config = createConfig(mockEnv, '/app', '1.0.0');

            expect(config.port).toBe(9000);
            expect(typeof config.port).toBe('number');
        });
    });

    describe('defaultEnvReader', () => {
        it('should read from process.env', () => {
            const originalValue = process.env.TEST_VAR_CONFIG;
            process.env.TEST_VAR_CONFIG = 'test-value';

            try {
                const value = defaultEnvReader('TEST_VAR_CONFIG', 'default');
                expect(value).toBe('test-value');
            } finally {
                if (originalValue === undefined) {
                    delete process.env.TEST_VAR_CONFIG;
                } else {
                    process.env.TEST_VAR_CONFIG = originalValue;
                }
            }
        });

        it('should return default when env var not set', () => {
            const value = defaultEnvReader('NONEXISTENT_VAR_12345', 'default-value');
            expect(value).toBe('default-value');
        });
    });

    describe('createDefaultConfig', () => {
        it('should create config using real environment', () => {
            const config = createDefaultConfig('/test/dir', '3.0.0');

            expect(config.serviceVersion).toBe('3.0.0');
            expect(config.queueName).toBe('jobs.created');
            expect(typeof config.port).toBe('number');
        });
    });
});
