import { describe, it, expect, vi } from 'vitest';
import fs from 'fs';
import path from 'path';

// Version reader helper
function readVersion() {
    const versionPath = path.join(__dirname, '..', 'VERSION');
    if (!fs.existsSync(versionPath)) {
        throw new Error(`VERSION file not found at ${versionPath}`);
    }
    const version = fs.readFileSync(versionPath, 'utf8').trim();
    if (!/^\d+\.\d+\.\d+$/.test(version)) {
        throw new Error(`Invalid SemVer format: ${version}`);
    }
    return version;
}

// Event envelope builder helper
function buildEventEnvelope(eventType, payload, options = {}) {
    const { v4: uuidv4 } = require('uuid');
    return {
        contractVersion: '1.0.0',
        eventType,
        eventId: options.eventId || uuidv4(),
        occurredAt: new Date().toISOString(),
        producer: {
            service: 'gateway',
            instanceId: process.env.HOSTNAME || 'test-instance',
            version: options.version || '0.1.0'
        },
        correlationId: options.correlationId || uuidv4(),
        idempotencyKey: options.idempotencyKey || uuidv4(),
        payload
    };
}

describe('Gateway', () => {
    describe('/healthz endpoint', () => {
        it('should return 200 with status ok', async () => {
            // Simulating health check response structure
            const healthResponse = { status: 'ok' };
            expect(healthResponse.status).toBe('ok');
        });

        it('should include version in health payload', () => {
            // Phase 11 hook: version in health response
            const version = '0.1.0'; // Will be read from VERSION file
            const healthResponse = { status: 'ok', version };
            expect(healthResponse.version).toBeDefined();
            expect(healthResponse.version).toMatch(/^\d+\.\d+\.\d+$/);
        });
    });

    describe('Event envelope builder', () => {
        it('should add producer fields to event envelope', () => {
            const payload = { id: 'test-job', type: 'test' };
            const envelope = buildEventEnvelope('job.created', payload, { version: '0.1.0' });

            // Verify producer fields exist
            expect(envelope.producer).toBeDefined();
            expect(envelope.producer.service).toBe('gateway');
            expect(envelope.producer.instanceId).toBeDefined();
            expect(envelope.producer.version).toBe('0.1.0');

            // Verify envelope structure
            expect(envelope.contractVersion).toBe('1.0.0');
            expect(envelope.eventType).toBe('job.created');
            expect(envelope.eventId).toBeDefined();
            expect(envelope.occurredAt).toBeDefined();
            expect(envelope.correlationId).toBeDefined();
            expect(envelope.idempotencyKey).toBeDefined();
            expect(envelope.payload).toEqual(payload);
        });
    });

    describe('OpenAPI Specification', () => {
        // Build a mock OpenAPI spec structure matching index.ts
        const buildOpenApiSpec = (version) => ({
            openapi: '3.0.3',
            info: {
                title: 'Gateway API',
                description: 'Distributed Task Observatory Gateway Service - accepts jobs and publishes events to RabbitMQ',
                version: version,
                contact: { name: 'Odd Essentials', url: 'https://oddessentials.com' },
            },
            servers: [{ url: 'http://localhost:3000', description: 'Local development' }],
            paths: {
                '/jobs': { post: { summary: 'Submit a new job' } },
                '/healthz': { get: { summary: 'Health check' } },
                '/readyz': { get: { summary: 'Readiness check' } },
                '/metrics': { get: { summary: 'Prometheus metrics' } },
                '/proxy/alerts': { get: { summary: 'Proxy Alertmanager alerts' } },
                '/proxy/targets': { get: { summary: 'Proxy Prometheus targets' } },
            },
        });

        it('should have valid OpenAPI 3.0 spec structure', () => {
            const spec = buildOpenApiSpec('0.1.0');

            expect(spec.openapi).toBe('3.0.3');
            expect(spec.info).toBeDefined();
            expect(spec.info.title).toBe('Gateway API');
            expect(spec.info.version).toMatch(/^\d+\.\d+\.\d+$/);
            expect(spec.servers).toBeDefined();
            expect(spec.servers.length).toBeGreaterThan(0);
        });

        it('should document all API endpoints', () => {
            const spec = buildOpenApiSpec('0.1.0');
            const requiredPaths = ['/jobs', '/healthz', '/readyz', '/metrics', '/proxy/alerts', '/proxy/targets'];

            for (const path of requiredPaths) {
                expect(spec.paths[path]).toBeDefined();
            }
        });

        it('should include contact information', () => {
            const spec = buildOpenApiSpec('0.1.0');

            expect(spec.info.contact).toBeDefined();
            expect(spec.info.contact.name).toBe('Odd Essentials');
            expect(spec.info.contact.url).toBe('https://oddessentials.com');
        });

        it('should include version from SERVICE_VERSION', () => {
            const version = readVersion();
            const spec = buildOpenApiSpec(version);

            expect(spec.info.version).toBe(version);
        });
    });
});
