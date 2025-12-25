/**
 * Builders Unit Tests
 * 
 * Tests for pure builder functions with injected dependencies.
 */

import { describe, it, expect, vi } from 'vitest';
import {
    buildEventEnvelope,
    buildHealthResponse,
    buildOpenApiSpec,
    buildValidationErrorResponse,
    buildInternalErrorResponse,
    buildJobAcceptedResponse,
} from '../lib/builders.js';

describe('Builders', () => {
    describe('buildEventEnvelope', () => {
        const mockUuid = vi.fn(() => 'mock-uuid-1234');
        const mockDate = vi.fn(() => new Date('2024-01-01T12:00:00.000Z'));

        it('should build a complete event envelope', () => {
            const payload = { id: 'job-1', type: 'test' };
            const options = { version: '1.0.0', instanceId: 'instance-1' };

            const envelope = buildEventEnvelope(
                'job.created',
                payload,
                options,
                mockUuid,
                mockDate
            );

            expect(envelope.contractVersion).toBe('1.0.0');
            expect(envelope.eventType).toBe('job.created');
            expect(envelope.eventId).toBe('mock-uuid-1234');
            expect(envelope.occurredAt).toBe('2024-01-01T12:00:00.000Z');
            expect(envelope.producer).toEqual({
                service: 'gateway',
                instanceId: 'instance-1',
                version: '1.0.0',
            });
            expect(envelope.payload).toEqual(payload);
        });

        it('should use provided eventId when given', () => {
            const envelope = buildEventEnvelope(
                'test.event',
                { id: 'j1', type: 't1' },
                { eventId: 'custom-event-id' },
                mockUuid,
                mockDate
            );

            expect(envelope.eventId).toBe('custom-event-id');
        });

        it('should generate eventId when not provided', () => {
            const envelope = buildEventEnvelope(
                'test.event',
                { id: 'j1', type: 't1' },
                {},
                mockUuid,
                mockDate
            );

            expect(envelope.eventId).toBe('mock-uuid-1234');
        });

        it('should use payload.id as idempotencyKey when provided', () => {
            const envelope = buildEventEnvelope(
                'test.event',
                { id: 'job-id-123', type: 't1' },
                {},
                mockUuid,
                mockDate
            );

            expect(envelope.idempotencyKey).toBe('job-id-123');
        });

        it('should use eventId as idempotencyKey when payload.id not provided', () => {
            const envelope = buildEventEnvelope(
                'test.event',
                { type: 't1' },
                { eventId: 'event-id-456' },
                mockUuid,
                mockDate
            );

            expect(envelope.idempotencyKey).toBe('event-id-456');
        });

        it('should use explicit idempotencyKey when provided', () => {
            const envelope = buildEventEnvelope(
                'test.event',
                { id: 'job-1', type: 't1' },
                { idempotencyKey: 'custom-idem-key' },
                mockUuid,
                mockDate
            );

            expect(envelope.idempotencyKey).toBe('custom-idem-key');
        });

        it('should use default version when not provided', () => {
            const envelope = buildEventEnvelope(
                'test.event',
                { id: 'j1', type: 't1' },
                {},
                mockUuid,
                mockDate
            );

            expect(envelope.producer.version).toBe('0.1.0');
        });

        it('should use default instanceId when not provided', () => {
            const envelope = buildEventEnvelope(
                'test.event',
                { id: 'j1', type: 't1' },
                {},
                mockUuid,
                mockDate
            );

            expect(envelope.producer.instanceId).toBe('unknown');
        });
    });

    describe('buildHealthResponse', () => {
        it('should build health response with version', () => {
            const response = buildHealthResponse('1.2.3');

            expect(response).toEqual({
                status: 'ok',
                version: '1.2.3',
            });
        });

        it('should always have status ok', () => {
            const response = buildHealthResponse('0.0.1');

            expect(response.status).toBe('ok');
        });
    });

    describe('buildOpenApiSpec', () => {
        it('should build a valid OpenAPI 3.0 spec', () => {
            const spec = buildOpenApiSpec('2.0.0');

            expect(spec.openapi).toBe('3.0.3');
            expect(spec.info.version).toBe('2.0.0');
            expect(spec.info.title).toBe('Gateway API');
        });

        it('should include all required endpoints', () => {
            const spec = buildOpenApiSpec('1.0.0');
            const requiredPaths = ['/jobs', '/healthz', '/readyz', '/metrics', '/proxy/alerts', '/proxy/targets'];

            for (const path of requiredPaths) {
                expect(spec.paths[path]).toBeDefined();
            }
        });

        it('should include contact information', () => {
            const spec = buildOpenApiSpec('1.0.0');

            expect(spec.info.contact).toEqual({
                name: 'Odd Essentials',
                url: 'https://oddessentials.com',
            });
        });

        it('should include server configuration', () => {
            const spec = buildOpenApiSpec('1.0.0');

            expect(spec.servers).toHaveLength(1);
            expect(spec.servers[0].url).toBe('http://localhost:3000');
        });

        it('should document POST /jobs endpoint', () => {
            const spec = buildOpenApiSpec('1.0.0');
            const jobsPath = spec.paths['/jobs'] as { post: { summary: string } };

            expect(jobsPath.post).toBeDefined();
            expect(jobsPath.post.summary).toBe('Submit a new job');
        });
    });

    describe('buildValidationErrorResponse', () => {
        it('should build error response with details', () => {
            const errors = [
                { path: '/id', message: 'is required' },
                { path: '/type', message: 'must be string' },
            ];

            const response = buildValidationErrorResponse(errors);

            expect(response.error).toBe('Invalid job data');
            expect(response.details).toEqual(errors);
        });

        it('should handle empty errors array', () => {
            const response = buildValidationErrorResponse([]);

            expect(response.error).toBe('Invalid job data');
            expect(response.details).toEqual([]);
        });
    });

    describe('buildInternalErrorResponse', () => {
        it('should build error response with custom message', () => {
            const response = buildInternalErrorResponse('Database connection failed');

            expect(response).toEqual({ error: 'Database connection failed' });
        });

        it('should build error response for contract violation', () => {
            const response = buildInternalErrorResponse('Internal contract violation');

            expect(response).toEqual({ error: 'Internal contract violation' });
        });
    });

    describe('buildJobAcceptedResponse', () => {
        it('should build success response with jobId and eventId', () => {
            const response = buildJobAcceptedResponse('job-123', 'event-456');

            expect(response).toEqual({
                jobId: 'job-123',
                eventId: 'event-456',
            });
        });

        it('should handle undefined jobId', () => {
            const response = buildJobAcceptedResponse(undefined, 'event-789');

            expect(response.jobId).toBeUndefined();
            expect(response.eventId).toBe('event-789');
        });
    });
});
