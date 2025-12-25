/**
 * Gateway Type Definitions
 * 
 * Shared types extracted for testability and reuse.
 * Pure type definitions - no runtime behavior.
 */

export interface JobData {
    id?: string;
    type?: string;
    payload?: unknown;
}

export interface EventProducer {
    service: string;
    instanceId: string;
    version: string;
}

export interface EventEnvelope {
    contractVersion: string;
    eventType: string;
    eventId: string;
    occurredAt: string;
    producer: EventProducer;
    correlationId: string;
    idempotencyKey: string;
    payload: JobData;
}

export interface HealthResponse {
    status: string;
    version: string;
}

export interface OpenApiSpec {
    openapi: string;
    info: {
        title: string;
        description: string;
        version: string;
        contact: { name: string; url: string };
    };
    servers: Array<{ url: string; description: string }>;
    paths: Record<string, Record<string, unknown>>;
}

export interface EventEnvelopeOptions {
    eventId?: string;
    version?: string;
    correlationId?: string;
    idempotencyKey?: string;
    instanceId?: string;
}

export interface ValidationError {
    message?: string;
    instancePath?: string;
}

export interface ValidateFn {
    (data: unknown): boolean;
    errors?: ValidationError[] | null;
}

/**
 * Dependency injection interfaces for testability
 */
export interface FileReader {
    (path: string): string;
}

export interface EnvReader {
    (key: string, defaultValue: string): string;
}

export interface UuidGenerator {
    (): string;
}

export interface DateProvider {
    (): Date;
}
