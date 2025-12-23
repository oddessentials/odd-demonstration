# Contributing to Distributed Task Observatory

Thank you for your interest in contributing! This document outlines the guidelines and expectations for contributors.

## Commit Convention

We use [Conventional Commits](https://www.conventionalcommits.org/) for all commit messages. This enables automated changelog generation and semantic versioning.

### Format

```
<type>(<scope>): <subject>

[optional body]

[optional footer(s)]
```

### Types

| Type | Description |
|------|-------------|
| `feat` | New feature |
| `fix` | Bug fix |
| `docs` | Documentation only |
| `style` | Formatting, no code change |
| `refactor` | Code change that neither fixes a bug nor adds a feature |
| `perf` | Performance improvement |
| `test` | Adding or updating tests |
| `build` | Build system or external dependencies |
| `ci` | CI configuration |
| `chore` | Other changes that don't modify src or test files |
| `revert` | Reverts a previous commit |

### Examples

```
feat(gateway): add health check endpoint
fix(processor): handle null task payloads
docs: update README with setup instructions
test(metrics-engine): add unit tests for aggregation
```

## Development Workflow

1. **Fork and clone** the repository
2. **Install dependencies**: `npm install` (installs Husky hooks)
3. **Create a branch**: `git checkout -b feat/my-feature`
4. **Make changes** following the coding standards
5. **Run tests**: `./scripts/run-all-tests.ps1`
6. **Commit** using Conventional Commits format
7. **Push and create a PR**

## Testing Requirements

- All new code must have accompanying tests
- Tests must pass before merging
- Coverage should not decrease on new code

## Service-Specific Guidelines

### Node.js (gateway)
- Run `npm run lint` before committing
- Run `npm run format` to fix formatting

### Python (processor)
- Run `ruff check .` for linting
- Run `black .` for formatting

### Go (metrics-engine, read-model)
- Run `golangci-lint run` for linting
- Run `go fmt ./...` for formatting

### Rust (tui)
- Run `cargo clippy` for linting
- Run `cargo fmt` for formatting

## Code Review

All PRs require:
- Passing CI checks
- At least one approving review
- No unresolved conversations

## Questions?

Open an issue if you have questions about contributing.
