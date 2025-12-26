#!/usr/bin/env python3
"""
Check that service VERSION files match K8s manifest image tags.

Validates:
- Every VERSION file in src/services/*/VERSION exists and is valid SemVer
- Every K8s manifest image tag matches the corresponding VERSION file
- Every workload has app.kubernetes.io/version label matching VERSION
"""
import os
import re
import sys
from pathlib import Path
import yaml


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


def get_service_versions(project_root: Path) -> dict[str, str]:
    """Read VERSION files from all services and interfaces."""
    versions = {}
    
    # Scan both src/services/ and src/interfaces/ for VERSION files
    scan_dirs = [
        project_root / "src" / "services",
        project_root / "src" / "interfaces",
    ]
    
    for scan_dir in scan_dirs:
        if not scan_dir.exists():
            continue
        for service_dir in scan_dir.iterdir():
            if service_dir.is_dir():
                version_file = service_dir / "VERSION"
                if version_file.exists():
                    version = version_file.read_text().strip()
                    # Use directory name as key (e.g., "web" -> "web-ui" mapping handled by manifest dict)
                    service_name = service_dir.name
                    if validate_semver(version):
                        versions[service_name] = version
                    else:
                        print(f"  [ERROR] {service_name}/VERSION has invalid SemVer: {version}")
    
    return versions


def check_k8s_manifests(project_root: Path, service_versions: dict[str, str]) -> list[str]:
    """Check K8s manifests for version consistency."""
    errors = []
    k8s_dir = project_root / "infra" / "k8s"
    
    if not k8s_dir.exists():
        print(f"  [WARN] K8s directory not found: {k8s_dir}")
        return errors
    
    # Map of K8s file names to service names
    # All services with VERSION files must be included here
    service_manifests = {
        "gateway.yaml": "gateway",
        "processor.yaml": "processor",
        "metrics-engine.yaml": "metrics-engine",
        "read-model.yaml": "read-model",
        "web-pty-ws.yaml": "web-pty-server",
        "web-ui-http.yaml": "web",  # Uses src/interfaces/web/VERSION
    }
    
    for manifest_name, service_name in service_manifests.items():
        manifest_path = k8s_dir / manifest_name
        if not manifest_path.exists():
            continue
        
        expected_version = service_versions.get(service_name)
        if not expected_version:
            print(f"  [WARN] No VERSION file for service: {service_name}")
            continue
        
        try:
            content = manifest_path.read_text()
            docs = list(yaml.safe_load_all(content))
            
            for doc in docs:
                if doc is None:
                    continue
                
                kind = doc.get("kind", "")
                
                # Check Deployment/StatefulSet containers
                if kind in ("Deployment", "StatefulSet"):
                    spec = doc.get("spec", {}).get("template", {}).get("spec", {})
                    containers = spec.get("containers", [])
                    
                    for container in containers:
                        image = container.get("image", "")
                        if ":" in image:
                            _, tag = image.rsplit(":", 1)
                            if tag == "latest":
                                errors.append(
                                    f"{manifest_name}: {service_name} uses :latest, "
                                    f"should be :{expected_version}"
                                )
                            elif tag != expected_version:
                                errors.append(
                                    f"{manifest_name}: {service_name} image tag '{tag}' "
                                    f"does not match VERSION '{expected_version}'"
                                )
                            else:
                                print(f"  [OK] {manifest_name}: image tag matches VERSION ({expected_version})")
                    
                    # Check for version label
                    labels = doc.get("spec", {}).get("template", {}).get("metadata", {}).get("labels", {})
                    version_label = labels.get("app.kubernetes.io/version")
                    if version_label is None:
                        print(f"  [WARN] {manifest_name}: missing app.kubernetes.io/version label")
                    elif version_label != expected_version:
                        errors.append(
                            f"{manifest_name}: version label '{version_label}' "
                            f"does not match VERSION '{expected_version}'"
                        )
        
        except Exception as e:
            errors.append(f"{manifest_name}: failed to parse - {e}")
    
    return errors


def main():
    """Run all version consistency checks."""
    print("=" * 60)
    print("SERVICE VERSION CONSISTENCY CHECK")
    print("=" * 60)
    
    try:
        project_root = find_project_root()
    except FileNotFoundError as e:
        print(f"ERROR: {e}")
        return 1
    
    # Get service versions
    print("\n>> Reading VERSION files...")
    service_versions = get_service_versions(project_root)
    for service, version in service_versions.items():
        print(f"  [OK] {service} = {version}")
    
    # Check K8s manifests
    print("\n>> Checking K8s manifest consistency...")
    errors = check_k8s_manifests(project_root, service_versions)
    
    # Report results
    print("\n" + "=" * 60)
    if errors:
        print(f"RESULTS: {len(errors)} error(s)")
        for error in errors:
            print(f"  [ERROR] {error}")
        print("=" * 60)
        return 1
    else:
        print("RESULTS: All checks passed (or warnings only)")
        print("=" * 60)
        return 0


if __name__ == "__main__":
    sys.exit(main())
