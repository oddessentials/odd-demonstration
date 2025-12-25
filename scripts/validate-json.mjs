#!/usr/bin/env node
// JSON Schema Validator Helper
// Usage: node scripts/validate-json.mjs <schema-path> <payload-path>
//
// Returns exit code 0 if valid, 1 if invalid or error.
// Errors are written to stderr in JSON format.

import { readFileSync } from 'fs';
import Ajv from 'ajv';
import addFormats from 'ajv-formats';

const [, , schemaPath, payloadPath] = process.argv;

if (!schemaPath || !payloadPath) {
    console.error('Usage: node validate-json.mjs <schema-path> <payload-path>');
    process.exit(1);
}

try {
    const schema = JSON.parse(readFileSync(schemaPath, 'utf8'));
    const payload = JSON.parse(readFileSync(payloadPath, 'utf8'));

    const ajv = new Ajv({
        allErrors: true,
        strict: false  // Allow unknown keywords like $version
    });
    addFormats(ajv);

    const validate = ajv.compile(schema);
    const valid = validate(payload);

    if (!valid) {
        console.error(JSON.stringify({
            valid: false,
            schemaPath,
            payloadPath,
            errors: validate.errors
        }, null, 2));
        process.exit(1);
    }

    console.log(JSON.stringify({ valid: true, schemaPath, payloadPath }));
    process.exit(0);

} catch (err) {
    console.error(JSON.stringify({
        valid: false,
        error: err.message,
        schemaPath,
        payloadPath
    }, null, 2));
    process.exit(1);
}
