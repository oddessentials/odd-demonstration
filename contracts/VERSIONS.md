# Contract Version Compatibility Matrix

This document tracks schema versions and service compatibility.

## Schema Versions

| Schema | Current Version | Introduced | Status |
|--------|-----------------|------------|--------|
| [event-envelope](./schemas/event-envelope.json) | 1.0.0 | Phase 11 | Stable |
| [job](./schemas/job.json) | 1.0.0 | Phase 11 | Stable |

## Service Compatibility

| Service | Min event-envelope | Min job | Notes |
|---------|-------------------|---------|-------|
| gateway | 1.0.0 | 1.0.0 | Produces events, validates jobs |
| processor | 1.0.0 | 1.0.0 | Consumes events, updates jobs |
| metrics-engine | 1.0.0 | - | Consumes events only |
| read-model | 1.0.0 | 1.0.0 | Reads from both |

## Compatibility Policy

### Backward-Compatible Changes (minor version bump)
- Adding optional fields
- Widening enum values
- Relaxing validation (e.g., making required → optional)

### Breaking Changes (major version bump required)
- Removing fields
- Narrowing enum values
- Adding required fields
- Changing field types
- Renaming fields

## Version History

### event-envelope
- **1.0.0** — Initial stable release (Phase 11)

### job
- **1.0.0** — Initial stable release (Phase 11)
