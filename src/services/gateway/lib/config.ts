/**
 * Gateway Configuration
 * 
 * Configuration with dependency injection for testability.
 * No side effects on import - all env reads go through injected reader.
 */

import type { EnvReader, FileReader } from './types.js';

export interface GatewayConfig {
    rabbitmqUrl: string;
    queueName: string;
    port: number;
    alertmanagerUrl: string;
    prometheusUrl: string;
    proxyTimeoutMs: number;
    contractsPath: string;
    serviceVersion: string;
}

/**
 * Default environment reader using process.env
 */
export const defaultEnvReader: EnvReader = (key: string, defaultValue: string): string => {
    return process.env[key] ?? defaultValue;
};

/**
 * Validate SemVer format
 */
export function isValidSemVer(version: string): boolean {
    return /^\d+\.\d+\.\d+$/.test(version);
}

/**
 * Load and validate version from file with injected file reader.
 * Throws on missing file or invalid format.
 */
export function loadVersionFile(
    versionPath: string,
    readFile: FileReader
): string {
    const version = readFile(versionPath).trim();
    if (!isValidSemVer(version)) {
        throw new Error(`Invalid SemVer format: ${version}`);
    }
    return version;
}

/**
 * Create configuration with injected dependencies.
 * Pure function - no side effects.
 */
export function createConfig(
    getEnv: EnvReader,
    dirname: string,
    serviceVersion: string
): GatewayConfig {
    return {
        rabbitmqUrl: getEnv('RABBITMQ_URL', 'amqp://guest:guest@rabbitmq:5672'),
        queueName: 'jobs.created',
        port: parseInt(getEnv('PORT', '3000'), 10),
        alertmanagerUrl: getEnv('ALERTMANAGER_URL', 'http://alertmanager:9093'),
        prometheusUrl: getEnv('PROMETHEUS_URL', 'http://prometheus:9090'),
        proxyTimeoutMs: 5000,
        contractsPath: getEnv('CONTRACTS_PATH', `${dirname}/../../../contracts`),
        serviceVersion,
    };
}

/**
 * Default config factory using real environment.
 * Used by runtime only - tests inject mock readers.
 */
export function createDefaultConfig(dirname: string, serviceVersion: string): GatewayConfig {
    return createConfig(defaultEnvReader, dirname, serviceVersion);
}
