/**
 * Golden Response Tests
 * 
 * Verifies API response shapes don't drift during refactoring.
 * These are contract tests against expected JSON structures.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import request from 'supertest';
import { createApp, type AppDependencies } from '../lib/app.js';
import type { GatewayConfig } from '../lib/config.js';
import type { ValidateFn } from '../lib/types.js';

describe('Golden Response Tests', () => {
    const mockConfig: GatewayConfig = {
        rabbitmqUrl: 'amqp://localhost:5672',
        queueName: 'test-queue',
        port: 3000,
        alertmanagerUrl: 'http://localhost:9093',
        prometheusUrl: 'http://localhost:9090',
        proxyTimeoutMs: 1000,
        contractsPath: '/contracts',
        serviceVersion: '1.2.3',
    };

    const mockValidateJob: ValidateFn = vi.fn(() => true) as unknown as ValidateFn;
    const mockValidateEvent: ValidateFn = vi.fn(() => true) as unknown as ValidateFn;
    const mockChannel = { sendToQueue: vi.fn().mockReturnValue(true) } as unknown as import('amqplib').Channel;

    const createTestApp = () => {
        const deps: AppDependencies = {
            config: mockConfig,
            validateJob: mockValidateJob,
            validateEvent: mockValidateEvent,
            generateUuid: () => 'golden-uuid-12345',
            getDate: () => new Date('2024-06-15T10:30:00.000Z'),
            getChannel: () => mockChannel,
        };
        return createApp(deps);
    };

    beforeEach(() => {
        vi.clearAllMocks();
        (mockValidateJob as ReturnType<typeof vi.fn>).mockReturnValue(true);
        (mockValidateEvent as ReturnType<typeof vi.fn>).mockReturnValue(true);
    });

    describe('Health Endpoint Golden Response', () => {
        it('GET /healthz should match golden response shape', async () => {
            const { app } = createTestApp();

            const response = await request(app).get('/healthz');

            // Golden response shape
            expect(response.body).toMatchObject({
                status: 'ok',
                version: expect.stringMatching(/^\d+\.\d+\.\d+$/),
            });
            expect(Object.keys(response.body).sort()).toEqual(['status', 'version']);
        });

        it('GET /readyz should have identical shape to /healthz', async () => {
            const { app } = createTestApp();

            const healthzResponse = await request(app).get('/healthz');
            const readyzResponse = await request(app).get('/readyz');

            expect(Object.keys(healthzResponse.body).sort())
                .toEqual(Object.keys(readyzResponse.body).sort());
        });
    });

    describe('Job Submission Golden Response', () => {
        it('POST /jobs success should match golden response shape', async () => {
            const { app } = createTestApp();

            const response = await request(app)
                .post('/jobs')
                .send({ id: 'test-job-id', type: 'TEST_TYPE' })
                .set('Content-Type', 'application/json');

            // Golden response shape for 202 Accepted
            expect(response.status).toBe(202);
            expect(response.body).toMatchObject({
                jobId: 'test-job-id',
                eventId: expect.any(String),
            });
            expect(Object.keys(response.body).sort()).toEqual(['eventId', 'jobId']);
        });

        it('POST /jobs with missing required field should match error shape', async () => {
            const { app } = createTestApp();
            (mockValidateJob as ReturnType<typeof vi.fn>).mockReturnValue(false);
            (mockValidateJob as ValidateFn).errors = [
                { instancePath: '/type', message: 'is required' },
            ];

            const response = await request(app)
                .post('/jobs')
                .send({ id: 'job-1' })
                .set('Content-Type', 'application/json');

            // Golden error response shape
            expect(response.status).toBe(400);
            expect(response.body).toMatchObject({
                error: 'Invalid job data',
                details: expect.arrayContaining([
                    expect.objectContaining({
                        path: expect.any(String),
                        message: expect.any(String),
                    }),
                ]),
            });
            expect(Object.keys(response.body).sort()).toEqual(['details', 'error']);
        });

        it('POST /jobs with invalid type should include path in error', async () => {
            const { app } = createTestApp();
            (mockValidateJob as ReturnType<typeof vi.fn>).mockReturnValue(false);
            (mockValidateJob as ValidateFn).errors = [
                { instancePath: '/payload/count', message: 'must be number' },
            ];

            const response = await request(app)
                .post('/jobs')
                .send({ id: 'job-1', type: 'test', payload: { count: 'not-a-number' } })
                .set('Content-Type', 'application/json');

            expect(response.status).toBe(400);
            expect(response.body.details[0].path).toBe('/payload/count');
        });
    });

    describe('OpenAPI Spec Golden Response', () => {
        it('GET /openapi.json should have required OpenAPI 3.0 structure', async () => {
            const { app } = createTestApp();

            const response = await request(app).get('/openapi.json');

            expect(response.status).toBe(200);
            expect(response.body).toMatchObject({
                openapi: expect.stringMatching(/^3\.\d+\.\d+$/),
                info: {
                    title: expect.any(String),
                    version: expect.stringMatching(/^\d+\.\d+\.\d+$/),
                    contact: {
                        name: expect.any(String),
                        url: expect.stringMatching(/^https?:\/\//),
                    },
                },
                servers: expect.arrayContaining([
                    expect.objectContaining({ url: expect.any(String) }),
                ]),
                paths: expect.any(Object),
            });
        });

        it('GET /openapi.json should document all public endpoints', async () => {
            const { app } = createTestApp();

            const response = await request(app).get('/openapi.json');
            const paths = Object.keys(response.body.paths);

            expect(paths).toContain('/jobs');
            expect(paths).toContain('/healthz');
            expect(paths).toContain('/readyz');
            expect(paths).toContain('/metrics');
        });
    });

    describe('Internal Error Golden Response', () => {
        it('should return consistent error shape on RabbitMQ failure', async () => {
            const deps: AppDependencies = {
                config: mockConfig,
                validateJob: mockValidateJob,
                validateEvent: mockValidateEvent,
                generateUuid: () => 'uuid',
                getDate: () => new Date(),
                getChannel: () => null, // No channel = RabbitMQ unavailable
            };
            const { app } = createApp(deps);

            const response = await request(app)
                .post('/jobs')
                .send({ id: 'job-1', type: 'test' })
                .set('Content-Type', 'application/json');

            expect(response.status).toBe(500);
            expect(response.body).toMatchObject({
                error: expect.any(String),
            });
            expect(Object.keys(response.body)).toEqual(['error']);
        });

        it('should return consistent error shape on event validation failure', async () => {
            const deps: AppDependencies = {
                config: mockConfig,
                validateJob: mockValidateJob,
                validateEvent: vi.fn(() => false) as unknown as ValidateFn,
                generateUuid: () => 'uuid',
                getDate: () => new Date(),
                getChannel: () => mockChannel,
            };
            const { app } = createApp(deps);

            const response = await request(app)
                .post('/jobs')
                .send({ id: 'job-1', type: 'test' })
                .set('Content-Type', 'application/json');

            expect(response.status).toBe(500);
            expect(response.body.error).toBe('Internal contract violation');
        });
    });
});
