/**
 * Validators Unit Tests
 * 
 * Tests for pure validation functions with dependency injection.
 */

import { describe, it, expect, vi } from 'vitest';
import {
    createAjv,
    compileSchema,
    formatValidationErrors,
    createDefaultSchemaReader,
    createValidators,
} from '../lib/validators.js';

describe('Validators', () => {
    describe('createAjv', () => {
        it('should create an AJV instance', () => {
            const ajv = createAjv();
            expect(ajv).toBeDefined();
            expect(typeof ajv.compile).toBe('function');
        });

        it('should support string formats (via ajv-formats)', () => {
            const ajv = createAjv();
            const schema = {
                type: 'object',
                properties: {
                    email: { type: 'string', format: 'email' },
                },
            };
            const validate = ajv.compile(schema);

            expect(validate({ email: 'test@example.com' })).toBe(true);
            expect(validate({ email: 'not-an-email' })).toBe(false);
        });
    });

    describe('compileSchema', () => {
        it('should compile a valid schema', () => {
            const ajv = createAjv();
            const schema = {
                type: 'object',
                properties: {
                    name: { type: 'string' },
                },
                required: ['name'],
            };

            const validate = compileSchema(ajv, schema);
            expect(typeof validate).toBe('function');
        });

        it('should validate data correctly', () => {
            const ajv = createAjv();
            const schema = {
                type: 'object',
                properties: {
                    id: { type: 'string' },
                    count: { type: 'number' },
                },
                required: ['id'],
            };

            const validate = compileSchema(ajv, schema);

            expect(validate({ id: 'test-123' })).toBe(true);
            expect(validate({ id: 'test', count: 42 })).toBe(true);
            expect(validate({ count: 42 })).toBe(false); // missing required id
            expect(validate({ id: 123 })).toBe(false); // wrong type
        });

        it('should attach errors on validation failure', () => {
            const ajv = createAjv();
            const schema = {
                type: 'object',
                properties: {
                    name: { type: 'string' },
                },
                required: ['name'],
            };

            const validate = compileSchema(ajv, schema);
            validate({});

            expect(validate.errors).toBeDefined();
            expect(validate.errors!.length).toBeGreaterThan(0);
        });
    });

    describe('formatValidationErrors', () => {
        it('should format null errors as empty array', () => {
            expect(formatValidationErrors(null)).toEqual([]);
        });

        it('should format undefined errors as empty array', () => {
            expect(formatValidationErrors(undefined)).toEqual([]);
        });

        it('should format empty array as empty array', () => {
            expect(formatValidationErrors([])).toEqual([]);
        });

        it('should format errors with path and message', () => {
            const errors = [
                { instancePath: '/name', message: 'must be string' },
                { instancePath: '/age', message: 'must be number' },
            ];

            const formatted = formatValidationErrors(errors);

            expect(formatted).toEqual([
                { path: '/name', message: 'must be string' },
                { path: '/age', message: 'must be number' },
            ]);
        });

        it('should use "/" as default path when instancePath missing', () => {
            const errors = [{ message: 'error at root' }];

            const formatted = formatValidationErrors(errors);

            expect(formatted).toEqual([{ path: '/', message: 'error at root' }]);
        });

        it('should use default message when message missing', () => {
            const errors = [{ instancePath: '/field' }];

            const formatted = formatValidationErrors(errors);

            expect(formatted).toEqual([{ path: '/field', message: 'Unknown validation error' }]);
        });
    });

    describe('createDefaultSchemaReader', () => {
        it('should create a schema reader from file reader', () => {
            const mockFileReader = vi.fn().mockReturnValue('{"type": "object"}');
            const schemaReader = createDefaultSchemaReader(mockFileReader);

            const schema = schemaReader('/path/to/schema.json');

            expect(mockFileReader).toHaveBeenCalledWith('/path/to/schema.json');
            expect(schema).toEqual({ type: 'object' });
        });

        it('should parse JSON content correctly', () => {
            const complexSchema = {
                type: 'object',
                properties: {
                    id: { type: 'string' },
                    nested: {
                        type: 'object',
                        properties: {
                            value: { type: 'number' },
                        },
                    },
                },
            };
            const mockFileReader = vi.fn().mockReturnValue(JSON.stringify(complexSchema));
            const schemaReader = createDefaultSchemaReader(mockFileReader);

            expect(schemaReader('/any/path')).toEqual(complexSchema);
        });
    });

    describe('createValidators', () => {
        it('should create job and event validators', () => {
            const jobSchema = {
                type: 'object',
                properties: {
                    id: { type: 'string' },
                    type: { type: 'string' },
                },
                required: ['id', 'type'],
            };
            const eventSchema = {
                type: 'object',
                properties: {
                    eventType: { type: 'string' },
                },
                required: ['eventType'],
            };

            const mockSchemaReader = vi.fn()
                .mockReturnValueOnce(jobSchema)
                .mockReturnValueOnce(eventSchema);

            const { validateJob, validateEvent } = createValidators('/contracts', mockSchemaReader);

            expect(mockSchemaReader).toHaveBeenCalledWith('/contracts/schemas/job.json');
            expect(mockSchemaReader).toHaveBeenCalledWith('/contracts/schemas/event-envelope.json');
            expect(typeof validateJob).toBe('function');
            expect(typeof validateEvent).toBe('function');
        });

        it('should validate job correctly', () => {
            const jobSchema = {
                type: 'object',
                properties: {
                    id: { type: 'string' },
                    type: { type: 'string' },
                },
                required: ['id', 'type'],
            };
            const eventSchema = { type: 'object' };

            const mockSchemaReader = vi.fn()
                .mockReturnValueOnce(jobSchema)
                .mockReturnValueOnce(eventSchema);

            const { validateJob } = createValidators('/contracts', mockSchemaReader);

            expect(validateJob({ id: 'job-1', type: 'test' })).toBe(true);
            expect(validateJob({ id: 'job-1' })).toBe(false);
        });
    });
});
