/**
 * Gateway App Factory
 * 
 * Creates Express app and router without starting server or connecting to RabbitMQ.
 * Follows "no side effects on import" pattern - all connections initiated explicitly.
 */

import express, { Request, Response, Router, Express } from 'express';
import swaggerUi from 'swagger-ui-express';
import prom from 'prom-client';
import type { Channel } from 'amqplib';
import type { GatewayConfig } from './config.js';
import type { ValidateFn, JobData, UuidGenerator, DateProvider } from './types.js';
import {
    buildEventEnvelope,
    buildHealthResponse,
    buildOpenApiSpec,
    buildValidationErrorResponse,
    buildInternalErrorResponse,
    buildJobAcceptedResponse,
} from './builders.js';
import { formatValidationErrors } from './validators.js';

/**
 * Dependencies for creating the app
 */
export interface AppDependencies {
    config: GatewayConfig;
    validateJob: ValidateFn;
    validateEvent: ValidateFn;
    generateUuid: UuidGenerator;
    getDate: DateProvider;
    getChannel: () => Channel | null;
}

/**
 * Metrics registry for Prometheus
 */
export interface AppMetrics {
    register: prom.Registry;
    jobsSubmitted: prom.Counter<'type'>;
    jobsAccepted: prom.Counter<string>;
}

/**
 * Create Prometheus metrics
 */
export function createMetrics(): AppMetrics {
    const register = new prom.Registry();
    prom.collectDefaultMetrics({ register });

    const jobsSubmitted = new prom.Counter({
        name: 'gateway_jobs_submitted_total',
        help: 'Total number of jobs submitted via gateway',
        labelNames: ['type'] as const,
        registers: [register],
    });

    const jobsAccepted = new prom.Counter({
        name: 'gateway_jobs_accepted_total',
        help: 'Total number of jobs accepted and published to RabbitMQ',
        registers: [register],
    });

    return { register, jobsSubmitted, jobsAccepted };
}

/**
 * Create the jobs router (POST /jobs)
 */
export function createJobsRouter(deps: AppDependencies, metrics: AppMetrics): Router {
    const router = Router();
    const { config, validateJob, validateEvent, generateUuid, getDate, getChannel } = deps;

    router.post('/', async (req: Request, res: Response): Promise<void> => {
        const jobData: JobData = req.body;
        metrics.jobsSubmitted.inc({ type: jobData.type || 'unknown' });

        if (!validateJob(jobData)) {
            const errors = formatValidationErrors(validateJob.errors);
            res.status(400).json(buildValidationErrorResponse(errors));
            return;
        }

        const eventId = generateUuid();
        const correlationId = req.get('X-Correlation-Id') || generateUuid();

        const event = buildEventEnvelope(
            'job.created',
            jobData,
            {
                eventId,
                correlationId,
                idempotencyKey: jobData.id || eventId,
                instanceId: config.rabbitmqUrl.includes('localhost') ? 'local' : process.env.HOSTNAME,
                version: '0.1.0',
            },
            generateUuid,
            getDate
        );

        if (!validateEvent(event)) {
            console.error('Internal Error: Generated event violates contract', validateEvent.errors);
            res.status(500).json(buildInternalErrorResponse('Internal contract violation'));
            return;
        }

        try {
            const channel = getChannel();
            if (!channel) {
                throw new Error('RabbitMQ channel not initialized');
            }
            channel.sendToQueue(config.queueName, Buffer.from(JSON.stringify(event)), {
                persistent: true,
            });
            metrics.jobsAccepted.inc();
            res.status(202).json(buildJobAcceptedResponse(jobData.id, eventId));
        } catch (error) {
            console.error('Failed to publish event', error);
            res.status(500).json(buildInternalErrorResponse('Failed to submit job'));
        }
    });

    return router;
}

/**
 * Create health check router
 * /healthz checks RabbitMQ connection - returns 503 if not ready
 * /readyz is a simple liveness check
 */
export function createHealthRouter(
    version: string,
    getChannel: () => Channel | null
): Router {
    const router = Router();

    // Readiness check - only healthy when RabbitMQ is connected
    router.get('/healthz', (_req: Request, res: Response) => {
        const channel = getChannel();
        if (channel) {
            res.json(buildHealthResponse(version));
        } else {
            res.status(503).json({ status: 'not_ready', reason: 'RabbitMQ not connected' });
        }
    });

    // Liveness check - always healthy if process is running
    router.get('/readyz', (_req: Request, res: Response) => res.json(buildHealthResponse(version)));

    return router;
}

/**
 * Create metrics router
 */
export function createMetricsRouter(metrics: AppMetrics): Router {
    const router = Router();

    router.get('/metrics', async (_req: Request, res: Response) => {
        res.set('Content-Type', metrics.register.contentType);
        res.end(await metrics.register.metrics());
    });

    return router;
}

/**
 * Create proxy router for observability APIs
 */
export function createProxyRouter(config: GatewayConfig): Router {
    const router = Router();

    router.get('/proxy/alerts', async (_req: Request, res: Response) => {
        try {
            const controller = new AbortController();
            const timeout = setTimeout(() => controller.abort(), config.proxyTimeoutMs);

            const response = await fetch(`${config.alertmanagerUrl}/api/v2/alerts`, {
                signal: controller.signal,
                headers: { Accept: 'application/json' },
            });
            clearTimeout(timeout);

            const data = await response.json();
            res.status(response.status).json(data);
        } catch (error) {
            if (error instanceof Error && error.name === 'AbortError') {
                res.status(504).json({ error: 'Alertmanager timeout' });
            } else {
                const errorMessage = error instanceof Error ? error.message : String(error);
                res.status(502).json({ error: 'Alertmanager unavailable', details: errorMessage });
            }
        }
    });

    router.get('/proxy/targets', async (_req: Request, res: Response) => {
        try {
            const controller = new AbortController();
            const timeout = setTimeout(() => controller.abort(), config.proxyTimeoutMs);

            const response = await fetch(`${config.prometheusUrl}/api/v1/targets`, {
                signal: controller.signal,
                headers: { Accept: 'application/json' },
            });
            clearTimeout(timeout);

            const data = await response.json();
            res.status(response.status).json(data);
        } catch (error) {
            if (error instanceof Error && error.name === 'AbortError') {
                res.status(504).json({ error: 'Prometheus timeout' });
            } else {
                const errorMessage = error instanceof Error ? error.message : String(error);
                res.status(502).json({ error: 'Prometheus unavailable', details: errorMessage });
            }
        }
    });

    return router;
}

/**
 * Create OpenAPI documentation router
 */
export function createDocsRouter(version: string): Router {
    const router = Router();
    const spec = buildOpenApiSpec(version);

    router.get('/openapi.json', (_req: Request, res: Response) => res.json(spec));
    router.use('/docs', swaggerUi.serve, swaggerUi.setup(spec));

    return router;
}

/**
 * Create the full Express app.
 * Does NOT start the server or connect to RabbitMQ.
 */
export function createApp(deps: AppDependencies): { app: Express; metrics: AppMetrics } {
    const app = express();
    app.use(express.json());

    const metrics = createMetrics();

    // Mount routers
    app.use(createHealthRouter(deps.config.serviceVersion, deps.getChannel));
    app.use(createMetricsRouter(metrics));
    app.use(createDocsRouter(deps.config.serviceVersion));
    app.use('/jobs', createJobsRouter(deps, metrics));
    app.use(createProxyRouter(deps.config));

    return { app, metrics };
}
