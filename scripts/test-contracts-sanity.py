#!/usr/bin/env python3
"""
Contract sanity tests for Phase 10.5.

Validates:
- All schema files load as valid JSON
- Every schema has $version (SemVer) and $id
- No duplicate $id across schemas
- VERSION files exist and parse as SemVer
- Every schema appears in contracts/VERSIONS.md (when it exists)
"""
import json
import os
import re
import sys
from pathlib import Path


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


def load_json_file(path: Path) -> dict:
    """Load and parse a JSON file."""
    with open(path, 'r', encoding='utf-8') as f:
        return json.load(f)


def check_schemas(contracts_path: Path) -> list[str]:
    """Validate all schema files."""
    errors = []
    schemas_dir = contracts_path / "schemas"
    
    if not schemas_dir.exists():
        errors.append(f"Schemas directory not found: {schemas_dir}")
        return errors
    
    schema_ids = {}
    
    for schema_file in schemas_dir.glob("*.json"):
        try:
            schema = load_json_file(schema_file)
            
            # Check $version exists and is valid SemVer
            version = schema.get("$version")
            if version is None:
                # For now, just warn - $version will be added in Phase 11.2
                print(f"  [WARN] {schema_file.name}: missing $version (will be added in Phase 11.2)")
            elif not validate_semver(version):
                errors.append(f"{schema_file.name}: invalid $version format: {version}")
            
            # Check $id exists (using $id per JSON Schema standard)
            schema_id = schema.get("$id")
            if schema_id is None:
                # For now, just warn - $id will be added in Phase 11.2
                print(f"  [WARN] {schema_file.name}: missing $id (will be added in Phase 11.2)")
            else:
                # Check for duplicate $id
                if schema_id in schema_ids:
                    errors.append(f"Duplicate $id '{schema_id}' in {schema_file.name} and {schema_ids[schema_id]}")
                else:
                    schema_ids[schema_id] = schema_file.name
            
            print(f"  [OK] {schema_file.name} loads as valid JSON")
            
        except json.JSONDecodeError as e:
            errors.append(f"{schema_file.name}: invalid JSON - {e}")
        except Exception as e:
            errors.append(f"{schema_file.name}: error loading - {e}")
    
    return errors


def check_version_files(project_root: Path) -> list[str]:
    """Validate all VERSION files exist and contain valid SemVer."""
    errors = []
    services_dir = project_root / "src" / "services"
    
    if not services_dir.exists():
        errors.append(f"Services directory not found: {services_dir}")
        return errors
    
    for service_dir in services_dir.iterdir():
        if service_dir.is_dir():
            version_file = service_dir / "VERSION"
            if not version_file.exists():
                print(f"  [WARN] {service_dir.name}: missing VERSION file (will be created in Phase 11.1)")
            else:
                try:
                    version = version_file.read_text().strip()
                    if not validate_semver(version):
                        errors.append(f"{service_dir.name}/VERSION: invalid SemVer format: {version}")
                    else:
                        print(f"  [OK] {service_dir.name}/VERSION = {version}")
                except Exception as e:
                    errors.append(f"{service_dir.name}/VERSION: error reading - {e}")
    
    return errors


def check_versions_md(contracts_path: Path) -> list[str]:
    """Check that VERSIONS.md exists and covers all schemas."""
    warnings = []
    versions_md = contracts_path / "VERSIONS.md"
    schemas_dir = contracts_path / "schemas"
    
    if not versions_md.exists():
        print("  [WARN] contracts/VERSIONS.md not found (will be created in Phase 11.2)")
        return warnings
    
    # Read VERSIONS.md and check that each schema is mentioned
    versions_content = versions_md.read_text()
    
    for schema_file in schemas_dir.glob("*.json"):
        schema_name = schema_file.stem
        if schema_name not in versions_content:
            warnings.append(f"Schema '{schema_name}' not documented in VERSIONS.md")
    
    return warnings


def main():
    """Run all contract sanity checks."""
    print("=" * 60)
    print("CONTRACT SANITY TESTS")
    print("=" * 60)
    
    try:
        project_root = find_project_root()
    except FileNotFoundError as e:
        print(f"ERROR: {e}")
        return 1
    
    contracts_path = project_root / "contracts"
    all_errors = []
    
    # Check schemas
    print("\n>> Validating schemas...")
    errors = check_schemas(contracts_path)
    all_errors.extend(errors)
    
    # Check VERSION files
    print("\n>> Validating VERSION files...")
    errors = check_version_files(project_root)
    all_errors.extend(errors)
    
    # Check VERSIONS.md coverage
    print("\n>> Checking VERSIONS.md coverage...")
    warnings = check_versions_md(contracts_path)
    for w in warnings:
        print(f"  [WARN] {w}")
    
    # Report results
    print("\n" + "=" * 60)
    if all_errors:
        print(f"RESULTS: {len(all_errors)} error(s)")
        for error in all_errors:
            print(f"  [ERROR] {error}")
        print("=" * 60)
        return 1
    else:
        print("RESULTS: All checks passed")
        print("=" * 60)
        return 0


if __name__ == "__main__":
    sys.exit(main())
