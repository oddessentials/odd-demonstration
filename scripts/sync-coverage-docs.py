#!/usr/bin/env python3
"""
Sync coverage documentation from coverage-config.json.

This script reads coverage thresholds from coverage-config.json (single source of truth)
and updates:
1. README.md - Coverage badges showing minimum thresholds
2. docs/agents/INVARIANTS.md - Coverage Invariants table

Usage:
    python sync-coverage-docs.py           # Update README.md and INVARIANTS.md
    python sync-coverage-docs.py --check   # Check if docs are in sync (exit 1 if not)
    python sync-coverage-docs.py --test    # Run self-tests

Exit codes:
    0 - Success (or docs already in sync for --check)
    1 - Failure (docs out of sync for --check, or test failures)
    2 - Configuration error
"""
import json
import re
import sys
from pathlib import Path
from typing import Optional


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


def get_badge_color(threshold: int) -> str:
    """Return shields.io color based on threshold value."""
    if threshold >= 80:
        return "brightgreen"
    elif threshold >= 50:
        return "yellowgreen"
    elif threshold >= 30:
        return "yellow"
    else:
        return "orange"


def generate_badge_markdown(service_name: str, threshold: int) -> str:
    """Generate a shields.io badge markdown for a service.
    
    Args:
        service_name: Display name for the badge (e.g., "Gateway", "TUI Lib")
        threshold: Minimum coverage threshold percentage
    
    Returns:
        Markdown string for the badge
    """
    color = get_badge_color(threshold)
    # URL-encode spaces and special characters
    encoded_name = service_name.replace(" ", "%20")
    return f"![{service_name}](https://img.shields.io/badge/{encoded_name}-{threshold}%25-{color})"


# Service display names mapping (coverage-config key -> display name)
SERVICE_DISPLAY_NAMES = {
    "processor": "Processor",
    "metrics-engine": "Metrics%20Engine",
    "read-model": "Read%20Model",
    "tui": "TUI%20Lib",
    "gateway": "Gateway",
    "web-pty-server": "PTY%20Server",
}

# Order for badges in README
BADGE_ORDER = ["gateway", "processor", "metrics-engine", "read-model", "tui", "web-pty-server"]


def generate_readme_badges(config: dict) -> str:
    """Generate the complete badges section for README.md."""
    thresholds = config.get("thresholds", {})
    badges = []
    
    for service_key in BADGE_ORDER:
        if service_key in thresholds:
            threshold = thresholds[service_key]["min"]
            display_name = SERVICE_DISPLAY_NAMES.get(service_key, service_key.title())
            color = get_badge_color(threshold)
            badges.append(f"![{display_name}](https://img.shields.io/badge/{display_name}-{threshold}%25-{color})")
    
    return "\n".join(badges)


def generate_invariants_table(config: dict) -> str:
    """Generate the Coverage Invariants table for INVARIANTS.md."""
    thresholds = config.get("thresholds", {})
    
    # Table header
    lines = [
        "| Service | Min Threshold | Warn Threshold | Notes |",
        "|---------|---------------|----------------|-------|",
    ]
    
    # Service order and display names for INVARIANTS
    invariants_order = [
        ("processor", "Processor (Python)"),
        ("metrics-engine", "Metrics Engine (Go)"),
        ("read-model", "Read Model (Go)"),
        ("tui", "TUI (Rust lib)"),
        ("gateway", "Gateway (TypeScript)"),
        ("web-pty-server", "web-pty-server (Rust)"),
    ]
    
    for service_key, display_name in invariants_order:
        if service_key in thresholds:
            svc = thresholds[service_key]
            min_val = svc["min"]
            warn_val = svc.get("warn", min_val)
            note = svc.get("note", "")
            lines.append(f"| {display_name} | {min_val}% | {warn_val}% | {note} |")
    
    # Add visual tests row (not in coverage-config)
    lines.append("| Visual Tests (Playwright) | — | — | Screenshot comparison; requires running cluster |")
    
    return "\n".join(lines)


def update_readme(project_root: Path, config: dict, check_only: bool = False) -> tuple[bool, str]:
    """Update README.md with new coverage badges.
    
    Returns:
        (changed: bool, message: str)
    """
    readme_path = project_root / "README.md"
    content = readme_path.read_text(encoding='utf-8')
    
    # Pattern to match the test coverage badges section
    # Looks for "**Test Coverage:**" followed by badge lines
    badge_pattern = re.compile(
        r'(\*\*Test Coverage:\*\*\s*\n\n)((?:!\[.*?\]\(https://img\.shields\.io/badge/.*?\)\s*\n?)+)',
        re.MULTILINE
    )
    
    new_badges = generate_readme_badges(config)
    
    match = badge_pattern.search(content)
    if not match:
        return False, "Could not find Test Coverage badges section in README.md"
    
    current_badges = match.group(2).strip()
    
    if current_badges == new_badges:
        return False, "README.md badges already in sync"
    
    if check_only:
        return True, f"README.md badges out of sync:\nExpected:\n{new_badges}\nActual:\n{current_badges}"
    
    # Replace badges
    new_content = badge_pattern.sub(f"\\1{new_badges}\n\n", content)
    readme_path.write_text(new_content, encoding='utf-8')
    
    return True, "Updated README.md badges"


def update_invariants(project_root: Path, config: dict, check_only: bool = False) -> tuple[bool, str]:
    """Update INVARIANTS.md with new coverage table.
    
    Returns:
        (changed: bool, message: str)
    """
    invariants_path = project_root / "docs" / "agents" / "INVARIANTS.md"
    content = invariants_path.read_text(encoding='utf-8')
    
    # Pattern to match the coverage table
    # Looks for "## Coverage Invariants" section and the table within it
    table_pattern = re.compile(
        r'(## Coverage Invariants\s*\n\n'
        r'Thresholds are externalized in `coverage-config\.json` and enforced by `scripts/check-coverage\.py`\.\s*\n\n)'
        r'(\| Service \| Min Threshold \| Warn Threshold \| Notes \|\s*\n'
        r'\|[-]+\|[-]+\|[-]+\|[-]+\|\s*\n'
        r'(?:\|[^\n]+\|\s*\n)+)',
        re.MULTILINE
    )
    
    new_table = generate_invariants_table(config)
    
    match = table_pattern.search(content)
    if not match:
        return False, "Could not find Coverage Invariants table in INVARIANTS.md"
    
    current_table = match.group(2).strip()
    
    if current_table == new_table:
        return False, "INVARIANTS.md table already in sync"
    
    if check_only:
        return True, f"INVARIANTS.md table out of sync"
    
    # Replace table
    new_content = table_pattern.sub(f"\\g<1>{new_table}\n\n", content)
    invariants_path.write_text(new_content, encoding='utf-8')
    
    return True, "Updated INVARIANTS.md coverage table"


def run_self_tests() -> bool:
    """Run internal validation tests."""
    print("=== sync-coverage-docs.py Self-Test ===\n")
    
    all_passed = True
    
    # Test get_badge_color
    print(">> Testing get_badge_color():")
    color_tests = [
        (80, "brightgreen"),
        (85, "brightgreen"),
        (50, "yellowgreen"),
        (70, "yellowgreen"),
        (30, "yellow"),
        (45, "yellow"),
        (10, "orange"),
        (29, "orange"),
    ]
    for threshold, expected in color_tests:
        result = get_badge_color(threshold)
        passed = result == expected
        status = "[PASS]" if passed else "[FAIL]"
        print(f"  {status} get_badge_color({threshold}) = {result} (expected {expected})")
        if not passed:
            all_passed = False
    
    # Test generate_badge_markdown
    print("\n>> Testing generate_badge_markdown():")
    badge = generate_badge_markdown("Gateway", 80)
    expected = "![Gateway](https://img.shields.io/badge/Gateway-80%25-brightgreen)"
    passed = badge == expected
    status = "[PASS]" if passed else "[FAIL]"
    print(f"  {status} generate_badge_markdown('Gateway', 80)")
    if not passed:
        print(f"    Expected: {expected}")
        print(f"    Got: {badge}")
        all_passed = False
    
    # Test generate_readme_badges
    print("\n>> Testing generate_readme_badges():")
    test_config = {
        "thresholds": {
            "gateway": {"min": 80, "warn": 85},
            "processor": {"min": 80, "warn": 85},
            "tui": {"min": 31, "warn": 32},
        }
    }
    badges = generate_readme_badges(test_config)
    # Check that all three services are in the output
    has_gateway = "Gateway" in badges
    has_processor = "Processor" in badges
    has_tui = "TUI" in badges
    passed = has_gateway and has_processor and has_tui
    status = "[PASS]" if passed else "[FAIL]"
    print(f"  {status} generate_readme_badges() contains expected services")
    if not passed:
        all_passed = False
    
    # Test generate_invariants_table
    print("\n>> Testing generate_invariants_table():")
    table = generate_invariants_table(test_config)
    has_header = "| Service | Min Threshold |" in table
    has_gateway = "Gateway (TypeScript)" in table
    passed = has_header and has_gateway
    status = "[PASS]" if passed else "[FAIL]"
    print(f"  {status} generate_invariants_table() has correct structure")
    if not passed:
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
    check_only = "--check" in sys.argv
    run_tests = "--test" in sys.argv
    
    if run_tests:
        return 0 if run_self_tests() else 1
    
    try:
        project_root = find_project_root()
        config = load_config(project_root)
    except FileNotFoundError as e:
        print(f"Configuration error: {e}")
        return 2
    
    any_changes = False
    any_errors = False
    
    # Update README
    changed, message = update_readme(project_root, config, check_only)
    print(f"README.md: {message}")
    if changed:
        any_changes = True
    if "Could not find" in message:
        any_errors = True
    
    # Update INVARIANTS
    changed, message = update_invariants(project_root, config, check_only)
    print(f"INVARIANTS.md: {message}")
    if changed:
        any_changes = True
    if "Could not find" in message:
        any_errors = True
    
    if any_errors:
        return 2
    
    if check_only and any_changes:
        print("\nDocs are out of sync. Run 'python scripts/sync-coverage-docs.py' to fix.")
        return 1
    
    if any_changes:
        print("\nDocs updated successfully.")
    else:
        print("\nAll docs already in sync.")
    
    return 0


if __name__ == "__main__":
    sys.exit(main())
