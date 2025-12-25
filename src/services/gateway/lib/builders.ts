/**
 * Gateway Builders
 * 
 * Pure builder functions with no side effects.
 * All dependencies injected via parameters.
 */

import type {
    EventEnvelope,
    EventEnvelopeOptions,
    HealthResponse,
    OpenApiSpec,
    JobData,
    UuidGenerator,
    DateProvider,
} from './types.js';

/**
 * Build event envelope with injected UUID and date providers.
 * Pure function - no global state access.
 */
export function buildEventEnvelope(
    eventType: string,
    payload: JobData,
    options: EventEnvelopeOptions,
    generateUuid: UuidGenerator,
    getDate: DateProvider
): EventEnvelope {
    const eventId = options.eventId ?? generateUuid();
    return {
        contractVersion: '1.0.0',
        eventType,
        eventId,
        occurredAt: getDate().toISOString(),
        producer: {
            service: 'gateway',
            instanceId: options.instanceId ?? 'unknown',
            version: options.version ?? '0.1.0',
        },
        correlationId: options.correlationId ?? generateUuid(),
        idempotencyKey: options.idempotencyKey ?? payload.id ?? eventId,
        payload,
    };
}

/**
 * Build health response.
 * Pure function.
 */
export function buildHealthResponse(version: string): HealthResponse {
    return { status: 'ok', version };
}

/**
 * Build OpenAPI specification.
 * Pure function - no external dependencies.
 */
export function buildOpenApiSpec(version: string): OpenApiSpec {
    return {
        openapi: '3.0.3',
        info: {
            title: 'Gateway API',
            description: 'Distributed Task Observatory Gateway Service - accepts jobs and publishes events to RabbitMQ',
            version,
            contact: { name: 'Odd Essentials', url: 'https://oddessentials.com' },
        },
        servers: [{ url: 'http://localhost:3000', description: 'Local development' }],
        paths: {
            '/jobs': {
                post: {
                    summary: 'Submit a new job',
                    description: 'Validates job data against schema and publishes to RabbitMQ',
                    requestBody: {
                        required: true,
                        content: {
                            'application/json': {
                                schema: {
                                    type: 'object',
                                    properties: {
                                        id: { type: 'string', description: 'Unique job identifier' },
                                        type: { type: 'string', description: 'Job type' },
                                        payload: { type: 'object', description: 'Job payload data' },
                                    },
                                    required: ['id', 'type'],
                                },
                            },
                        },
                    },
                    responses: {
                        '202': { description: 'Job accepted', content: { 'application/json': { schema: { type: 'object', properties: { jobId: { type: 'string' }, eventId: { type: 'string' } } } } } },
                        '400': { description: 'Invalid job data' },
                        '500': { description: 'Server error' },
                    },
                },
            },
            '/healthz': {
                get: { summary: 'Health check', responses: { '200': { description: 'Service healthy' } } },
            },
            '/readyz': {
                get: { summary: 'Readiness check', responses: { '200': { description: 'Service ready' } } },
            },
            '/metrics': {
                get: { summary: 'Prometheus metrics', responses: { '200': { description: 'Prometheus metrics in text format' } } },
            },
            '/proxy/alerts': {
                get: { summary: 'Proxy Alertmanager alerts', responses: { '200': { description: 'Current alerts' }, '502': { description: 'Alertmanager unavailable' } } },
            },
            '/proxy/targets': {
                get: { summary: 'Proxy Prometheus targets', responses: { '200': { description: 'Prometheus targets' }, '502': { description: 'Prometheus unavailable' } } },
            },
        },
    };
}

/**
 * Build error response for validation failures.
 */
export function buildValidationErrorResponse(
    errors: Array<{ path: string; message: string }>
): { error: string; details: Array<{ path: string; message: string }> } {
    return {
        error: 'Invalid job data',
        details: errors,
    };
}

/**
 * Build error response for internal errors.
 */
export function buildInternalErrorResponse(message: string): { error: string } {
    return { error: message };
}

/**
 * Build success response for job submission.
 */
export function buildJobAcceptedResponse(
    jobId: string | undefined,
    eventId: string
): { jobId: string | undefined; eventId: string } {
    return { jobId, eventId };
}
