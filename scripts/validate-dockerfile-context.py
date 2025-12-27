#!/usr/bin/env python3
"""
Validate CI Docker build contexts match Dockerfile path assumptions.

ASSUMPTIONS (documented in docs/INVARIANTS.md B1):
1. Dockerfiles with COPY paths starting with 'src/' or 'contracts/' require
   repo root as build context (context: '.')
2. Dockerfiles with local-relative COPY paths (e.g., 'package.json', 'go.mod')
   can use service-local context
3. Only services in the CI build-images matrix are checked

This script is CONSERVATIVE:
- It only flags DEFINITE mismatches (repo-relative path + non-root context)
- Ambiguous cases produce warnings, not failures
- Run with --strict to fail on warnings too

Usage:
  python scripts/validate-dockerfile-context.py        # Warn on ambiguous
  python scripts/validate-dockerfile-context.py --strict  # Fail on any issue
"""

import re
import sys
from pathlib import Path

# Explicit list of services that require repo root context
# These are services whose Dockerfiles use COPY with repo-relative paths
REPO_ROOT_REQUIRED = {
    "gateway": "Uses COPY src/services/gateway/... and COPY contracts/",
    "processor": "Uses COPY src/services/processor/... and COPY contracts/",
    "web-pty-server": "Uses COPY src/services/web-pty-server/... and COPY src/interfaces/tui/...",
}

# Services that are self-contained (service-local context OK)
SERVICE_LOCAL_OK = {
    "metrics-engine": "Self-contained Go service, no shared assets",
    "read-model": "Self-contained Go service, no shared assets",
    "web-ui": "Self-contained, builds within src/interfaces/web",
}


def parse_ci_contexts(ci_path: Path) -> dict[str, str]:
    """Extract build contexts from CI workflow using regex (avoids yaml dep)."""
    content = ci_path.read_text(encoding="utf-8")
    
    # Find the build-images job matrix
    contexts = {}
    
    # Pattern matches matrix items like:
    #   - service: gateway
    #     context: .
    #     dockerfile: src/services/gateway/Dockerfile
    pattern = r'-\s+service:\s+(\S+)\s+context:\s+(\S+)\s+dockerfile:'
    
    for match in re.finditer(pattern, content):
        service, context = match.groups()
        contexts[service] = context
    
    return contexts


def main():
    strict = "--strict" in sys.argv
    repo_root = Path(__file__).parent.parent
    ci_path = repo_root / ".github/workflows/ci.yml"
    
    if not ci_path.exists():
        print("⚠️  CI workflow not found, skipping validation")
        sys.exit(0)
    
    ci_contexts = parse_ci_contexts(ci_path)
    
    if not ci_contexts:
        print("⚠️  No build contexts found in CI workflow")
        sys.exit(0)
    
    errors = []
    warnings = []
    
    for service, context in ci_contexts.items():
        if service in REPO_ROOT_REQUIRED:
            if context != ".":
                errors.append(
                    f"❌ {service}: requires repo root context (.)\n"
                    f"   Current: {context}\n"
                    f"   Reason: {REPO_ROOT_REQUIRED[service]}"
                )
            else:
                print(f"✅ {service}: context '.' (repo root) - correct")
        elif service in SERVICE_LOCAL_OK:
            print(f"✅ {service}: context '{context}' (service-local OK)")
        else:
            warnings.append(
                f"⚠️  {service}: not in explicit list, context '{context}'\n"
                f"   Please add to REPO_ROOT_REQUIRED or SERVICE_LOCAL_OK"
            )
    
    # Summary
    print()
    if errors:
        print("=" * 60)
        print("BUILD CONTEXT ERRORS (will break CI builds):")
        print("=" * 60)
        for e in errors:
            print(e)
            print()
    
    if warnings:
        print("=" * 60)
        print("WARNINGS (review manually):")
        print("=" * 60)
        for w in warnings:
            print(w)
            print()
    
    if errors:
        print("❌ FAILED: Fix the errors above")
        sys.exit(1)
    elif warnings and strict:
        print("❌ FAILED (--strict): Resolve warnings above")
        sys.exit(1)
    elif warnings:
        print("⚠️  PASSED with warnings (run --strict to fail on warnings)")
        sys.exit(0)
    else:
        print("✅ All build contexts are valid")
        sys.exit(0)


if __name__ == "__main__":
    main()
