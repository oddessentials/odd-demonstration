/**
 * App Factory Unit Tests
 * 
 * Tests for Express app creation with supertest for route testing.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import request from 'supertest';
import {
    createApp,
    createMetrics,
    createHealthRouter,
    createDocsRouter,
    type AppDependencies,
} from '../lib/app.js';
import type { GatewayConfig } from '../lib/config.js';
import type { ValidateFn } from '../lib/types.js';

describe('App Factory', () => {
    // Mock dependencies
    const mockConfig: GatewayConfig = {
        rabbitmqUrl: 'amqp://localhost:5672',
        queueName: 'test-queue',
        port: 3000,
        alertmanagerUrl: 'http://localhost:9093',
        prometheusUrl: 'http://localhost:9090',
        proxyTimeoutMs: 1000,
        contractsPath: '/contracts',
        serviceVersion: '1.0.0',
    };

    const mockValidateJob: ValidateFn = vi.fn(() => true) as unknown as ValidateFn;
    const mockValidateEvent: ValidateFn = vi.fn(() => true) as unknown as ValidateFn;
    const mockGenerateUuid = vi.fn(() => 'test-uuid-1234');
    const mockGetDate = vi.fn(() => new Date('2024-01-01T00:00:00.000Z'));
    const mockChannel = {
        sendToQueue: vi.fn().mockReturnValue(true),
    } as unknown as import('amqplib').Channel;
    const mockGetChannel = vi.fn(() => mockChannel);

    const createMockDeps = (): AppDependencies => ({
        config: mockConfig,
        validateJob: mockValidateJob,
        validateEvent: mockValidateEvent,
        generateUuid: mockGenerateUuid,
        getDate: mockGetDate,
        getChannel: mockGetChannel,
    });

    beforeEach(() => {
        vi.clearAllMocks();
        (mockValidateJob as ReturnType<typeof vi.fn>).mockReturnValue(true);
        (mockValidateEvent as ReturnType<typeof vi.fn>).mockReturnValue(true);
    });

    describe('createMetrics', () => {
        it('should create Prometheus metrics registry', () => {
            const metrics = createMetrics();

            expect(metrics.register).toBeDefined();
            expect(metrics.jobsSubmitted).toBeDefined();
            expect(metrics.jobsAccepted).toBeDefined();
        });

        it('should have counter methods', () => {
            const metrics = createMetrics();

            expect(typeof metrics.jobsSubmitted.inc).toBe('function');
            expect(typeof metrics.jobsAccepted.inc).toBe('function');
        });
    });

    describe('createHealthRouter', () => {
        it('should return 200 on GET /healthz when channel connected', async () => {
            const express = await import('express');
            const app = express.default();
            const mockChannel = {} as import('amqplib').Channel;
            app.use(createHealthRouter('2.0.0', () => mockChannel));

            const response = await request(app).get('/healthz');

            expect(response.status).toBe(200);
            expect(response.body).toEqual({ status: 'ok', version: '2.0.0' });
        });

        it('should return 503 on GET /healthz when channel not connected', async () => {
            const express = await import('express');
            const app = express.default();
            app.use(createHealthRouter('2.0.0', () => null));

            const response = await request(app).get('/healthz');

            expect(response.status).toBe(503);
            expect(response.body.status).toBe('not_ready');
        });

        it('should handle GET /readyz regardless of channel state', async () => {
            const express = await import('express');
            const app = express.default();
            app.use(createHealthRouter('3.0.0', () => null));

            const response = await request(app).get('/readyz');

            expect(response.status).toBe(200);
            expect(response.body).toEqual({ status: 'ok', version: '3.0.0' });
        });
    });

    describe('createDocsRouter', () => {
        it('should handle GET /openapi.json', async () => {
            const express = await import('express');
            const app = express.default();
            app.use(createDocsRouter('1.5.0'));

            const response = await request(app).get('/openapi.json');

            expect(response.status).toBe(200);
            expect(response.body.openapi).toBe('3.0.3');
            expect(response.body.info.version).toBe('1.5.0');
        });

        it('should serve Swagger UI at /docs', async () => {
            const express = await import('express');
            const app = express.default();
            app.use(createDocsRouter('1.0.0'));

            const response = await request(app).get('/docs/');

            expect(response.status).toBe(200);
            expect(response.text).toContain('swagger');
        });
    });

    describe('createApp', () => {
        it('should create Express app with all routes', () => {
            const deps = createMockDeps();
            const { app, metrics } = createApp(deps);

            expect(app).toBeDefined();
            expect(metrics).toBeDefined();
        });

        it('should handle health check endpoints', async () => {
            const deps = createMockDeps();
            const { app } = createApp(deps);

            const healthzResponse = await request(app).get('/healthz');
            const readyzResponse = await request(app).get('/readyz');

            expect(healthzResponse.status).toBe(200);
            expect(readyzResponse.status).toBe(200);
        });

        it('should handle metrics endpoint', async () => {
            const deps = createMockDeps();
            const { app } = createApp(deps);

            const response = await request(app).get('/metrics');

            expect(response.status).toBe(200);
            expect(response.text).toContain('gateway_jobs');
        });

        it('should handle openapi.json endpoint', async () => {
            const deps = createMockDeps();
            const { app } = createApp(deps);

            const response = await request(app).get('/openapi.json');

            expect(response.status).toBe(200);
            expect(response.body.info.version).toBe('1.0.0');
        });
    });

    describe('POST /jobs', () => {
        it('should accept valid job and return 202', async () => {
            const deps = createMockDeps();
            const { app } = createApp(deps);

            const response = await request(app)
                .post('/jobs')
                .send({ id: 'job-1', type: 'test' })
                .set('Content-Type', 'application/json');

            expect(response.status).toBe(202);
            expect(response.body.jobId).toBe('job-1');
            expect(response.body.eventId).toBe('test-uuid-1234');
        });

        it('should reject invalid job with 400', async () => {
            const deps = createMockDeps();
            (deps.validateJob as ReturnType<typeof vi.fn>).mockReturnValue(false);
            (deps.validateJob as ValidateFn).errors = [
                { instancePath: '/id', message: 'is required' },
            ];
            const { app } = createApp(deps);

            const response = await request(app)
                .post('/jobs')
                .send({ type: 'test' })
                .set('Content-Type', 'application/json');

            expect(response.status).toBe(400);
            expect(response.body.error).toBe('Invalid job data');
            expect(response.body.details).toHaveLength(1);
        });

        it('should return 500 when RabbitMQ channel not available', async () => {
            const deps = createMockDeps();
            deps.getChannel = vi.fn(() => null);
            const { app } = createApp(deps);

            const response = await request(app)
                .post('/jobs')
                .send({ id: 'job-1', type: 'test' })
                .set('Content-Type', 'application/json');

            expect(response.status).toBe(500);
            expect(response.body.error).toBe('Failed to submit job');
        });

        it('should return 500 on event validation failure', async () => {
            const deps = createMockDeps();
            (deps.validateEvent as ReturnType<typeof vi.fn>).mockReturnValue(false);
            (deps.validateEvent as ValidateFn).errors = [{ message: 'invalid event' }];
            const { app } = createApp(deps);

            const response = await request(app)
                .post('/jobs')
                .send({ id: 'job-1', type: 'test' })
                .set('Content-Type', 'application/json');

            expect(response.status).toBe(500);
            expect(response.body.error).toBe('Internal contract violation');
        });

        it('should use X-Correlation-Id header when provided', async () => {
            // Create a fresh mock channel for this specific test
            const sendToQueueMock = vi.fn().mockReturnValue(true);
            const testChannel = { sendToQueue: sendToQueueMock } as unknown as import('amqplib').Channel;
            const deps: AppDependencies = {
                config: mockConfig,
                validateJob: mockValidateJob,
                validateEvent: mockValidateEvent,
                generateUuid: mockGenerateUuid,
                getDate: mockGetDate,
                getChannel: () => testChannel,
            };
            const { app } = createApp(deps);

            await request(app)
                .post('/jobs')
                .send({ id: 'job-1', type: 'test' })
                .set('Content-Type', 'application/json')
                .set('X-Correlation-Id', 'custom-correlation-id');

            expect(sendToQueueMock).toHaveBeenCalled();
            const sentMessage = JSON.parse(
                sendToQueueMock.mock.calls[0][1].toString()
            );
            expect(sentMessage.correlationId).toBe('custom-correlation-id');
        });

        it('should increment metrics on job submission', async () => {
            const deps = createMockDeps();
            const { app, metrics } = createApp(deps);

            // Spy on metrics
            const submitSpy = vi.spyOn(metrics.jobsSubmitted, 'inc');
            const acceptSpy = vi.spyOn(metrics.jobsAccepted, 'inc');

            await request(app)
                .post('/jobs')
                .send({ id: 'job-1', type: 'analytics' })
                .set('Content-Type', 'application/json');

            expect(submitSpy).toHaveBeenCalledWith({ type: 'analytics' });
            expect(acceptSpy).toHaveBeenCalled();
        });
    });
});
