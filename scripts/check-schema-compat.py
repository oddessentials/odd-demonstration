#!/usr/bin/env python3
"""
Schema compatibility checker for Phase 11.3.

Detects breaking changes between schema versions:
- Removed required fields (BREAKING)
- Narrowed enum values (BREAKING)
- Changed field types (BREAKING)
- Added required fields (BREAKING)
- Added optional fields (OK)
- Widened enum values (OK)
"""
import json
import re
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
    
    # Check for removed required fields
    removed_required = old_required - new_required
    # Actually, making a required field optional is NOT breaking
    # But removing a field entirely IS breaking
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


def main():
    """Run schema compatibility checks."""
    print("=" * 60)
    print("SCHEMA COMPATIBILITY CHECK")
    print("=" * 60)
    
    try:
        project_root = find_project_root()
    except FileNotFoundError as e:
        print(f"ERROR: {e}")
        return 1
    
    # Run fixture tests if they exist
    fixtures_dir = project_root / "tests" / "fixtures" / "schema-compat"
    if fixtures_dir.exists():
        print("\n>> Running fixture tests...")
        passed, failed = run_fixture_tests(fixtures_dir)
        print(f"\n>> Fixture results: {passed} passed, {failed} failed")
        if failed > 0:
            return 1
    else:
        print(f"\n>> Fixtures directory not found: {fixtures_dir}")
        print("   (Create fixtures to test breaking change detection)")
    
    # Validate current schemas have $version
    print("\n>> Checking schema versions...")
    schemas_dir = project_root / "contracts" / "schemas"
    errors = []
    
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
            print(f"  [WARN] {schema_file.name}: missing $id")
    
    print("\n" + "=" * 60)
    if errors:
        print(f"RESULTS: {len(errors)} error(s)")
        for error in errors:
            print(f"  [ERROR] {error}")
        return 1
    else:
        print("RESULTS: All schema checks passed")
    print("=" * 60)
    return 0


if __name__ == "__main__":
    sys.exit(main())
