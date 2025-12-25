/**
 * Gateway Validators
 * 
 * Pure validation functions with no side effects.
 * Uses dependency injection for all external dependencies.
 */

import Ajv from 'ajv';
import addFormats from 'ajv-formats';
import type { ValidateFn, FileReader } from './types.js';

/**
 * JSON Schema reader interface for DI
 */
export interface SchemaReader {
    (schemaPath: string): unknown;
}

/**
 * Default schema reader using file system
 */
export function createDefaultSchemaReader(readFile: FileReader): SchemaReader {
    return (schemaPath: string): unknown => {
        const content = readFile(schemaPath);
        return JSON.parse(content);
    };
}

/**
 * Create AJV instance with formats
 */
export function createAjv(): Ajv.default {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const ajv = new Ajv.default({ strict: false }) as any;
    addFormats.default(ajv);
    return ajv;
}

/**
 * Compile a JSON schema into a validator function
 */
export function compileSchema(ajv: Ajv.default, schema: unknown): ValidateFn {
    return ajv.compile(schema) as ValidateFn;
}

/**
 * Load and compile validators for job and event schemas.
 * Pure function - uses injected schema reader.
 */
export function createValidators(
    contractsPath: string,
    readSchema: SchemaReader
): { validateJob: ValidateFn; validateEvent: ValidateFn } {
    const ajv = createAjv();

    const jobSchema = readSchema(`${contractsPath}/schemas/job.json`);
    const eventSchema = readSchema(`${contractsPath}/schemas/event-envelope.json`);

    return {
        validateJob: compileSchema(ajv, jobSchema),
        validateEvent: compileSchema(ajv, eventSchema),
    };
}

/**
 * Format validation errors for API response
 */
export function formatValidationErrors(
    errors: Array<{ message?: string; instancePath?: string }> | null | undefined
): Array<{ path: string; message: string }> {
    if (!errors) return [];
    return errors.map(err => ({
        path: err.instancePath || '/',
        message: err.message || 'Unknown validation error',
    }));
}
