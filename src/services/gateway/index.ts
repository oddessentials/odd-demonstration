/**
 * Gateway Service Entry Point
 * 
 * This file handles runtime initialization only:
 * - Loads VERSION file
 * - Connects to RabbitMQ
 * - Starts Express server
 * 
 * All business logic is in ./lib/ modules for testability.
 * Side effects are behind the runtime guard at the bottom.
 */

import amqp, { Channel } from 'amqplib';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

import { createDefaultConfig, loadVersionFile } from './lib/config.js';
import { createValidators, createDefaultSchemaReader } from './lib/validators.js';
import { createApp } from './lib/app.js';
import { v4 as uuidv4 } from 'uuid';

// ESM __dirname equivalent
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Re-export lib modules for testing convenience
export * from './lib/index.js';

/**
 * RabbitMQ connection state
 */
let channel: Channel | null = null;

/**
 * Connect to RabbitMQ with retry logic
 */
async function connectRabbitMQ(url: string, queueName: string): Promise<void> {
    try {
        const connection = await amqp.connect(url);
        channel = await connection.createChannel();
        await channel.assertQueue(queueName, { durable: true });
        console.log('Connected to RabbitMQ');
    } catch (error) {
        console.error('Failed to connect to RabbitMQ', error);
        setTimeout(() => connectRabbitMQ(url, queueName), 5000);
    }
}

/**
 * File reader for VERSION and schemas
 */
function readFileSync(filePath: string): string {
    return fs.readFileSync(filePath, 'utf8');
}

/**
 * Start the gateway service.
 * Exported for testing, but only called at runtime via the guard below.
 */
export async function startServer(): Promise<void> {
    // Load version with fail-fast
    const versionPath = path.join(__dirname, 'VERSION');
    let serviceVersion: string;
    try {
        serviceVersion = loadVersionFile(versionPath, readFileSync);
        console.log(`Gateway version: ${serviceVersion}`);
    } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        console.error(`FATAL: Failed to load VERSION file: ${errorMessage}`);
        process.exit(1);
    }

    // Create config
    const config = createDefaultConfig(__dirname, serviceVersion);

    // Load validators
    const schemaReader = createDefaultSchemaReader(readFileSync);
    const { validateJob, validateEvent } = createValidators(config.contractsPath, schemaReader);

    // Create app with all dependencies
    const { app } = createApp({
        config,
        validateJob,
        validateEvent,
        generateUuid: uuidv4,
        getDate: () => new Date(),
        getChannel: () => channel,
    });

    // Connect to RabbitMQ
    connectRabbitMQ(config.rabbitmqUrl, config.queueName);

    // Start server
    app.listen(config.port, () => {
        console.log(`Gateway service listening on port ${config.port}`);
    });
}

/**
 * Runtime guard - only execute when run directly, not when imported
 * This enables testing without triggering side effects
 */
// Check if this module is the entry point
const isMainModule = import.meta.url === `file://${process.argv[1].replace(/\\/g, '/')}`;

if (isMainModule) {
    startServer();
}
