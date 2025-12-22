const express = require('express');
const amqp = require('amqplib');
const { v4: uuidv4 } = require('uuid');
const prom = require('prom-client');

const Ajv = require('ajv');
const addFormats = require('ajv-formats');
const fs = require('fs');
const path = require('path');

const app = express();
app.use(express.json());

const ajv = new Ajv();
addFormats(ajv);

// Load schemas (paths relative to Bazel runfiles or project root)
const CONTRACTS_PATH = process.env.CONTRACTS_PATH || path.join(__dirname, '../../../contracts');
const eventSchema = JSON.parse(fs.readFileSync(path.join(CONTRACTS_PATH, 'schemas/event-envelope.json'), 'utf8'));
const jobSchema = JSON.parse(fs.readFileSync(path.join(CONTRACTS_PATH, 'schemas/job.json'), 'utf8'));


const validateJob = ajv.compile(jobSchema);
const validateEvent = ajv.compile(eventSchema);

// Metrics setup
const register = new prom.Registry();
prom.collectDefaultMetrics({ register });

const jobsSubmitted = new prom.Counter({
    name: 'gateway_jobs_submitted_total',
    help: 'Total number of jobs submitted via gateway',
    labelNames: ['type'],
    registers: [register]
});

const jobsAccepted = new prom.Counter({
    name: 'gateway_jobs_accepted_total',
    help: 'Total number of jobs accepted and published to RabbitMQ',
    registers: [register]
});


const RABBITMQ_URL = process.env.RABBITMQ_URL || 'amqp://guest:guest@rabbitmq:5672';
const QUEUE_NAME = 'jobs.created';

let channel;

async function connectRabbitMQ() {
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

app.post('/jobs', async (req, res) => {
    const jobData = req.body;
    jobsSubmitted.inc({ type: jobData.type || 'unknown' });

    if (!validateJob(jobData)) {

        return res.status(400).json({ error: 'Invalid job data', details: validateJob.errors });
    }

    const eventId = uuidv4();
    const correlationId = req.get('X-Correlation-Id') || uuidv4();

    const event = {
        contractVersion: '1.0.0',
        eventType: 'job.created',
        eventId: eventId,
        occurredAt: new Date().toISOString(),
        producer: {
            service: 'gateway',
            instanceId: process.env.HOSTNAME || 'unknown',
            version: '0.1.0'
        },
        correlationId: correlationId,
        idempotencyKey: jobData.id || eventId,
        payload: jobData
    };

    if (!validateEvent(event)) {
        console.error('Internal Error: Generated event violates contract', validateEvent.errors);
        return res.status(500).json({ error: 'Internal contract violation' });
    }

    try {
        channel.sendToQueue(QUEUE_NAME, Buffer.from(JSON.stringify(event)), {
            persistent: true
        });
        jobsAccepted.inc();
        res.status(202).json({ jobId: jobData.id, eventId: eventId });
    } catch (error) {

        console.error('Failed to publish event', error);
        res.status(500).json({ error: 'Failed to submit job' });
    }
});

app.get('/healthz', (req, res) => res.send('OK'));
app.get('/readyz', (req, res) => res.send('OK'));

const PORT = process.env.PORT || 3000;
// Metrics endpoint
app.get('/metrics', async (req, res) => {
    res.set('Content-Type', register.contentType);
    res.end(await register.metrics());
});

app.listen(PORT, () => {

    console.log(`Gateway service listening on port ${PORT}`);
});
