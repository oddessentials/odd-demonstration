import express, { Request, Response } from 'express';
import amqp, { Channel } from 'amqplib';
import { v4 as uuidv4 } from 'uuid';
import prom from 'prom-client';
import Ajv from 'ajv';
import addFormats from 'ajv-formats';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

// ESM __dirname equivalent
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Type definitions
interface JobData {
    id?: string;
    type?: string;
    payload?: unknown;
}

interface EventEnvelope {
    contractVersion: string;
    eventType: string;
    eventId: string;
    occurredAt: string;
    producer: {
        service: string;
        instanceId: string;
        version: string;
    };
    correlationId: string;
    idempotencyKey: string;
    payload: JobData;
}

interface HealthResponse {
    status: string;
    version: string;
}

const app = express();
app.use(express.json());

// Read VERSION file with fail-fast on missing/invalid
const VERSION_PATH = path.join(__dirname, 'VERSION');
let SERVICE_VERSION: string;
try {
    SERVICE_VERSION = fs.readFileSync(VERSION_PATH, 'utf8').trim();
    if (!/^\d+\.\d+\.\d+$/.test(SERVICE_VERSION)) {
        throw new Error(`Invalid SemVer format: ${SERVICE_VERSION}`);
    }
    console.log(`Gateway version: ${SERVICE_VERSION}`);
} catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    console.error(`FATAL: Failed to load VERSION file: ${errorMessage}`);
    process.exit(1);
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const ajv = new Ajv.default({ strict: false }) as any;
addFormats.default(ajv);

// Load schemas (paths relative to Bazel runfiles or project root)
const CONTRACTS_PATH = process.env.CONTRACTS_PATH || path.join(__dirname, '../../../contracts');
const eventSchema = JSON.parse(fs.readFileSync(path.join(CONTRACTS_PATH, 'schemas/event-envelope.json'), 'utf8'));
const jobSchema = JSON.parse(fs.readFileSync(path.join(CONTRACTS_PATH, 'schemas/job.json'), 'utf8'));

// Ajv validate function with errors property
interface ValidateFn {
    (data: unknown): boolean;
    errors?: Array<{ message?: string; instancePath?: string }> | null;
}

const validateJob: ValidateFn = ajv.compile(jobSchema);
const validateEvent: ValidateFn = ajv.compile(eventSchema);

// Metrics setup
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

const RABBITMQ_URL = process.env.RABBITMQ_URL || 'amqp://guest:guest@rabbitmq:5672';
const QUEUE_NAME = 'jobs.created';

let channel: Channel | null = null;

async function connectRabbitMQ(): Promise<void> {
    try {
        const connection = await amqp.connect(RABBITMQ_URL);
        channel = await connection.createChannel();
        await channel.assertQueue(QUEUE_NAME, { durable: true });
        console.log('Connected to RabbitMQ');
    } catch (error) {
        console.error('Failed to connect to RabbitMQ', error);
        setTimeout(connectRabbitMQ, 5000);
    }
}

connectRabbitMQ();

app.post('/jobs', async (req: Request, res: Response): Promise<void> => {
    const jobData: JobData = req.body;
    jobsSubmitted.inc({ type: jobData.type || 'unknown' });

    if (!validateJob(jobData)) {
        res.status(400).json({ error: 'Invalid job data', details: validateJob.errors });
        return;
    }

    const eventId = uuidv4();
    const correlationId = req.get('X-Correlation-Id') || uuidv4();

    const event: EventEnvelope = {
        contractVersion: '1.0.0',
        eventType: 'job.created',
        eventId: eventId,
        occurredAt: new Date().toISOString(),
        producer: {
            service: 'gateway',
            instanceId: process.env.HOSTNAME || 'unknown',
            version: '0.1.0',
        },
        correlationId: correlationId,
        idempotencyKey: jobData.id || eventId,
        payload: jobData,
    };

    if (!validateEvent(event)) {
        console.error('Internal Error: Generated event violates contract', validateEvent.errors);
        res.status(500).json({ error: 'Internal contract violation' });
        return;
    }

    try {
        if (!channel) {
            throw new Error('RabbitMQ channel not initialized');
        }
        channel.sendToQueue(QUEUE_NAME, Buffer.from(JSON.stringify(event)), {
            persistent: true,
        });
        jobsAccepted.inc();
        res.status(202).json({ jobId: jobData.id, eventId: eventId });
    } catch (error) {
        console.error('Failed to publish event', error);
        res.status(500).json({ error: 'Failed to submit job' });
    }
});

// Health endpoints with version
const healthResponse = (): HealthResponse => ({ status: 'ok', version: SERVICE_VERSION });
app.get('/healthz', (_req: Request, res: Response) => res.json(healthResponse()));
app.get('/readyz', (_req: Request, res: Response) => res.json(healthResponse()));

const PORT = process.env.PORT || 3000;

// Metrics endpoint
app.get('/metrics', async (_req: Request, res: Response) => {
    res.set('Content-Type', register.contentType);
    res.end(await register.metrics());
});

// Proxy endpoints for observability APIs (avoid CORS issues in browser)
const ALERTMANAGER_URL = process.env.ALERTMANAGER_URL || 'http://alertmanager:9093';
const PROMETHEUS_URL = process.env.PROMETHEUS_URL || 'http://prometheus:9090';
const PROXY_TIMEOUT_MS = 5000;

app.get('/proxy/alerts', async (_req: Request, res: Response) => {
    try {
        const controller = new AbortController();
        const timeout = setTimeout(() => controller.abort(), PROXY_TIMEOUT_MS);

        const response = await fetch(`${ALERTMANAGER_URL}/api/v2/alerts`, {
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

app.get('/proxy/targets', async (_req: Request, res: Response) => {
    try {
        const controller = new AbortController();
        const timeout = setTimeout(() => controller.abort(), PROXY_TIMEOUT_MS);

        const response = await fetch(`${PROMETHEUS_URL}/api/v1/targets`, {
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

app.listen(PORT, () => {
    console.log(`Gateway service listening on port ${PORT}`);
});
