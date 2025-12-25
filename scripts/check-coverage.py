#!/usr/bin/env python3
"""
Unified coverage enforcement script.

Single source of truth for coverage validation across all services.
Reads thresholds from coverage-config.json and validates actual coverage.

Usage:
    python check-coverage.py <service> <coverage_percent>
    python check-coverage.py --self-test  # Run internal validation tests

Exit codes:
    0 - Coverage meets minimum threshold
    1 - Coverage below minimum threshold
    2 - Configuration error or invalid arguments
"""
import json
import os
import sys
from pathlib import Path


def find_project_root() -> Path:
    """Find project root by looking for MODULE.bazel."""
    current = Path(__file__).resolve().parent
    while current != current.parent:
        if (current / "MODULE.bazel").exists():
            return current
        current = current.parent
    raise FileNotFoundError("Could not find project root (MODULE.bazel)")


def load_config(project_root: Path) -> dict:
    """Load coverage configuration."""
    config_path = project_root / "coverage-config.json"
    if not config_path.exists():
        raise FileNotFoundError(f"Coverage config not found: {config_path}")
    
    with open(config_path, 'r', encoding='utf-8') as f:
        return json.load(f)


def parse_coverage(raw_value: str) -> float:
    """Parse coverage value from various formats.
    
    Handles:
    - "85.5%" -> 85.5
    - "85.5"  -> 85.5
    - "85"    -> 85.0
    """
    value = raw_value.strip().rstrip('%')
    return float(value)


def check_coverage(service: str, coverage_pct: float, config: dict) -> tuple[bool, str]:
    """
    Check if coverage meets threshold.
    
    Returns:
        (passed: bool, message: str)
    """
    thresholds = config.get("thresholds", {})
    
    if service not in thresholds:
        return False, f"Unknown service: {service}. Valid: {list(thresholds.keys())}"
    
    service_config = thresholds[service]
    min_threshold = service_config["min"]
    warn_threshold = service_config.get("warn", min_threshold)
    
    if coverage_pct < min_threshold:
        return False, f"FAIL: {service} coverage {coverage_pct:.1f}% below minimum {min_threshold}%"
    elif coverage_pct < warn_threshold:
        return True, f"WARN: {service} coverage {coverage_pct:.1f}% below warning threshold {warn_threshold}%"
    else:
        return True, f"OK: {service} coverage {coverage_pct:.1f}% meets threshold (min: {min_threshold}%)"


def run_self_test() -> bool:
    """Run internal validation tests with sample coverage outputs."""
    print("=== check-coverage.py Self-Test ===\n")
    
    # Sample config for testing
    test_config = {
        "thresholds": {
            "processor": {"min": 80, "warn": 85},
            "metrics-engine": {"min": 10, "warn": 15},
        }
    }
    
    test_cases = [
        # (service, coverage, expected_pass, expected_substring)
        ("processor", 90.0, True, "OK"),
        ("processor", 82.0, True, "WARN"),
        ("processor", 75.0, False, "FAIL"),
        ("metrics-engine", 12.0, True, "WARN"),
        ("metrics-engine", 8.0, False, "FAIL"),
        ("unknown", 50.0, False, "Unknown service"),
    ]
    
    # Test parse_coverage with various formats
    parse_tests = [
        ("85.5%", 85.5),
        ("85.5", 85.5),
        ("85", 85.0),
        ("  90%  ", 90.0),
    ]
    
    all_passed = True
    
    print(">> Testing parse_coverage():")
    for raw, expected in parse_tests:
        result = parse_coverage(raw)
        passed = abs(result - expected) < 0.01
        status = "[PASS]" if passed else "[FAIL]"
        print(f"  {status} parse_coverage({raw!r}) = {result} (expected {expected})")
        if not passed:
            all_passed = False
    
    print("\n>> Testing check_coverage():")
    for service, coverage, expected_pass, expected_substr in test_cases:
        passed, message = check_coverage(service, coverage, test_config)
        test_passed = (passed == expected_pass) and (expected_substr in message)
        status = "[PASS]" if test_passed else "[FAIL]"
        print(f"  {status} {service} @ {coverage}% -> {message}")
        if not test_passed:
            all_passed = False
    
    print()
    if all_passed:
        print("=== All self-tests passed ===")
        return True
    else:
        print("=== Some self-tests FAILED ===")
        return False


def main() -> int:
    """Main entry point."""
    if len(sys.argv) == 2 and sys.argv[1] == "--self-test":
        return 0 if run_self_test() else 1
    
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <service> <coverage_percent>")
        print(f"       {sys.argv[0]} --self-test")
        print()
        print("Services: processor, metrics-engine, read-model, tui")
        return 2
    
    service = sys.argv[1]
    
    try:
        coverage_pct = parse_coverage(sys.argv[2])
    except ValueError:
        print(f"Invalid coverage value: {sys.argv[2]}")
        return 2
    
    try:
        project_root = find_project_root()
        config = load_config(project_root)
    except FileNotFoundError as e:
        print(f"Configuration error: {e}")
        return 2
    
    passed, message = check_coverage(service, coverage_pct, config)
    print(message)
    
    return 0 if passed else 1


if __name__ == "__main__":
    sys.exit(main())
