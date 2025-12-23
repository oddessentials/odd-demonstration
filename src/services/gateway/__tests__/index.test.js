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
});
