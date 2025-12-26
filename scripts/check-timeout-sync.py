#!/usr/bin/env python3
"""
Timeout Sync Invariant Test

Ensures the CI job timeout for integration-phase is always >= harness budget + buffer.
This prevents the CI job from killing a passing test run.

Invariant: CI timeout >= (harness budget + startup/teardown buffer)
"""

import re
import sys
from pathlib import Path

# Configuration
HARNESS_SCRIPT = Path("scripts/integration-harness.ps1")
CI_WORKFLOW = Path(".github/workflows/ci.yml")
BUFFER_SECONDS = 60  # Buffer for startup/teardown

def extract_harness_budget():
    """Extract RuntimeBudgetSec from integration-harness.ps1"""
    content = HARNESS_SCRIPT.read_text(encoding='utf-8')
    match = re.search(r'\$RuntimeBudgetSec\s*=\s*(\d+)', content)
    if not match:
        print(f"ERROR: Could not find $RuntimeBudgetSec in {HARNESS_SCRIPT}")
        sys.exit(1)
    return int(match.group(1))

def extract_ci_timeout():
    """Extract timeout-minutes for integration-phase job from ci.yml"""
    content = CI_WORKFLOW.read_text(encoding='utf-8')
    
    # Find integration-phase section and its timeout
    # Pattern: Look for "Run integration harness" step and its timeout-minutes
    lines = content.split('\n')
    in_integration_step = False
    
    for i, line in enumerate(lines):
        if 'Run integration harness' in line:
            in_integration_step = True
        if in_integration_step and 'timeout-minutes:' in line:
            match = re.search(r'timeout-minutes:\s*(\d+)', line)
            if match:
                return int(match.group(1)) * 60  # Convert to seconds
    
    print(f"ERROR: Could not find integration harness timeout in {CI_WORKFLOW}")
    sys.exit(1)

def main():
    harness_budget = extract_harness_budget()
    ci_timeout = extract_ci_timeout()
    required_timeout = harness_budget + BUFFER_SECONDS
    
    print(f"=== Timeout Sync Invariant Check ===")
    print(f"Harness budget:     {harness_budget}s ({harness_budget // 60}m {harness_budget % 60}s)")
    print(f"Buffer:             {BUFFER_SECONDS}s")
    print(f"Required CI timeout: {required_timeout}s ({required_timeout // 60}m {required_timeout % 60}s)")
    print(f"Actual CI timeout:  {ci_timeout}s ({ci_timeout // 60}m)")
    print()
    
    if ci_timeout < required_timeout:
        print(f"[FAIL] CI timeout ({ci_timeout}s) < required ({required_timeout}s)")
        print(f"       Increase timeout-minutes in ci.yml integration-phase job")
        sys.exit(1)
    else:
        print(f"[PASS] CI timeout ({ci_timeout}s) >= required ({required_timeout}s)")
        sys.exit(0)

if __name__ == "__main__":
    main()
