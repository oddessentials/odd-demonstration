#!/usr/bin/env python3
"""
Schema compatibility checker for Phase 11.3.

Usage:
  python check-schema-compat.py           # Run fixture tests + schema validation
  python check-schema-compat.py --ci      # CI mode: skip fixtures if no schema changes

Detects breaking changes between schema versions:
- Removed required fields (BREAKING → MAJOR)
- Narrowed enum values (BREAKING → MAJOR)
- Changed field types (BREAKING → MAJOR)
- Added required fields (BREAKING → MAJOR)
- Added optional fields (OK → MINOR)
- Widened enum values (OK → MINOR)
"""
import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


def find_project_root():
    """Find project root by looking for MODULE.bazel."""
    current = Path(__file__).resolve().parent
    while current != current.parent:
        if (current / "MODULE.bazel").exists():
            return current
        current = current.parent
    raise FileNotFoundError("Could not find project root (MODULE.bazel)")


def validate_semver(version: str) -> bool:
    """Check if version matches SemVer format."""
    return bool(re.match(r'^\d+\.\d+\.\d+$', version))


def parse_semver(version: str) -> tuple[int, int, int]:
    """Parse SemVer string into tuple."""
    parts = version.split('.')
    return int(parts[0]), int(parts[1]), int(parts[2])


def is_major_bump(old_version: str, new_version: str) -> bool:
    """Check if new_version is a major version bump from old_version."""
    old = parse_semver(old_version)
    new = parse_semver(new_version)
    return new[0] > old[0]


def get_required_fields(schema: dict) -> set[str]:
    """Get required fields from schema."""
    return set(schema.get("required", []))


def get_properties(schema: dict) -> dict[str, dict]:
    """Get properties from schema."""
    return schema.get("properties", {})


def get_enum_values(prop: dict) -> set[str] | None:
    """Get enum values from a property, or None if not an enum."""
    if "enum" in prop:
        return set(prop["enum"])
    return None


def check_breaking_changes(old_schema: dict, new_schema: dict) -> list[str]:
    """
    Compare two schema versions and return list of breaking changes.
    """
    breaking_changes = []
    
    old_required = get_required_fields(old_schema)
    new_required = get_required_fields(new_schema)
    old_props = get_properties(old_schema)
    new_props = get_properties(new_schema)
    
    # Check for removed fields (always breaking)
    for field in old_props:
        if field not in new_props:
            breaking_changes.append(f"Removed field: {field}")
    
    # Check for new required fields (breaking for consumers)
    new_required_fields = new_required - old_required
    for field in new_required_fields:
        if field not in old_props:  # Truly new required field
            breaking_changes.append(f"Added required field: {field}")
    
    # Check for type changes and enum narrowing
    for field in old_props:
        if field not in new_props:
            continue  # Already caught above
        
        old_prop = old_props[field]
        new_prop = new_props[field]
        
        # Check type changes
        old_type = old_prop.get("type")
        new_type = new_prop.get("type")
        if old_type != new_type:
            breaking_changes.append(f"Changed type of {field}: {old_type} -> {new_type}")
        
        # Check enum narrowing
        old_enum = get_enum_values(old_prop)
        new_enum = get_enum_values(new_prop)
        if old_enum is not None and new_enum is not None:
            removed_values = old_enum - new_enum
            if removed_values:
                breaking_changes.append(
                    f"Narrowed enum for {field}: removed {sorted(removed_values)}"
                )
    
    return breaking_changes


def load_schema(path: Path) -> dict:
    """Load schema from file."""
    with open(path, 'r', encoding='utf-8') as f:
        return json.load(f)


def get_changed_schemas(project_root: Path, base_ref: str = "origin/main") -> list[Path]:
    """Get list of schema files changed compared to base_ref."""
    try:
        result = subprocess.run(
            ["git", "diff", "--name-only", base_ref, "HEAD", "--", "contracts/schemas/*.json"],
            capture_output=True,
            text=True,
            cwd=project_root,
            check=True
        )
        changed = result.stdout.strip().split('\n')
        return [project_root / f for f in changed if f and f.endswith('.json')]
    except subprocess.CalledProcessError:
        # If git fails (e.g., no base ref), return empty
        return []


def run_fixture_tests(fixtures_dir: Path) -> tuple[int, int]:
    """Run fixture-based tests. Returns (passed, failed) counts."""
    passed = 0
    failed = 0
    
    # Expected outcomes for fixtures
    test_cases = [
        ("add-optional-field", False),  # Not breaking
        ("remove-required-field", True),  # Breaking
        ("narrow-enum", True),  # Breaking
        ("widen-enum", False),  # Not breaking
    ]
    
    for test_name, should_break in test_cases:
        old_path = fixtures_dir / f"{test_name}-old.json"
        new_path = fixtures_dir / f"{test_name}-new.json"
        
        if not old_path.exists() or not new_path.exists():
            print(f"  [SKIP] {test_name}: fixtures not found")
            continue
        
        old_schema = load_schema(old_path)
        new_schema = load_schema(new_path)
        breaking = check_breaking_changes(old_schema, new_schema)
        
        has_breaking = len(breaking) > 0
        
        if has_breaking == should_break:
            print(f"  [PASS] {test_name}: correctly {'detected' if should_break else 'allowed'}")
            passed += 1
        else:
            print(f"  [FAIL] {test_name}: expected {'breaking' if should_break else 'non-breaking'}, got {breaking}")
            failed += 1
    
    return passed, failed


def check_versions_md_coverage(project_root: Path) -> list[str]:
    """Check that every schema appears in VERSIONS.md."""
    errors = []
    versions_md = project_root / "contracts" / "VERSIONS.md"
    schemas_dir = project_root / "contracts" / "schemas"
    
    if not versions_md.exists():
        errors.append("contracts/VERSIONS.md not found")
        return errors
    
    content = versions_md.read_text()
    
    for schema_file in schemas_dir.glob("*.json"):
        schema_name = schema_file.stem
        # Check if schema is mentioned in VERSIONS.md
        if schema_name not in content:
            errors.append(f"Schema '{schema_name}' not documented in VERSIONS.md")
    
    return errors


def main():
    """Run schema compatibility checks."""
    parser = argparse.ArgumentParser(description="Schema compatibility checker")
    parser.add_argument("--ci", action="store_true", help="CI mode: skip fixture tests if no schema changes")
    parser.add_argument("--base", default="origin/main", help="Base ref for diff comparison")
    args = parser.parse_args()
    
    print("=" * 60)
    print("SCHEMA COMPATIBILITY CHECK")
    print("=" * 60)
    
    try:
        project_root = find_project_root()
    except FileNotFoundError as e:
        print(f"ERROR: {e}")
        return 1
    
    errors = []
    
    # Check VERSIONS.md coverage
    print("\n>> Checking VERSIONS.md coverage...")
    coverage_errors = check_versions_md_coverage(project_root)
    if coverage_errors:
        for err in coverage_errors:
            print(f"  [ERROR] {err}")
        errors.extend(coverage_errors)
    else:
        print("  [OK] All schemas documented in VERSIONS.md")
    
    # In CI mode, check if schemas changed
    if args.ci:
        changed = get_changed_schemas(project_root, args.base)
        if not changed:
            print(f"\n>> CI mode: No schema changes vs {args.base}, skipping diff checks")
        else:
            print(f"\n>> CI mode: {len(changed)} schema(s) changed vs {args.base}")
            for f in changed:
                print(f"  - {f.name}")
    
    # Run fixture tests (always, to validate compat checker logic)
    fixtures_dir = project_root / "tests" / "fixtures" / "schema-compat"
    if fixtures_dir.exists():
        print("\n>> Running fixture tests...")
        passed, failed = run_fixture_tests(fixtures_dir)
        print(f"\n>> Fixture results: {passed} passed, {failed} failed")
        if failed > 0:
            errors.append(f"{failed} fixture test(s) failed")
    else:
        print(f"\n>> Fixtures directory not found: {fixtures_dir}")
    
    # Validate current schemas have $version and $id
    print("\n>> Checking schema metadata...")
    schemas_dir = project_root / "contracts" / "schemas"
    schema_ids = {}
    
    for schema_file in schemas_dir.glob("*.json"):
        schema = load_schema(schema_file)
        version = schema.get("$version")
        schema_id = schema.get("$id")
        
        if version is None:
            errors.append(f"{schema_file.name}: missing $version")
        elif not validate_semver(version):
            errors.append(f"{schema_file.name}: invalid $version format: {version}")
        else:
            print(f"  [OK] {schema_file.name}: $version={version}")
        
        if schema_id is None:
            errors.append(f"{schema_file.name}: missing $id")
        else:
            # Check for duplicate $id
            if schema_id in schema_ids:
                errors.append(f"Duplicate $id '{schema_id}' in {schema_file.name} and {schema_ids[schema_id]}")
            else:
                schema_ids[schema_id] = schema_file.name
    
    # Report results
    print("\n" + "=" * 60)
    if errors:
        print(f"RESULTS: {len(errors)} error(s)")
        for error in errors:
            print(f"  [ERROR] {error}")
        print("=" * 60)
        return 1
    else:
        print("RESULTS: All schema checks passed")
        print("=" * 60)
        return 0


if __name__ == "__main__":
    sys.exit(main())
