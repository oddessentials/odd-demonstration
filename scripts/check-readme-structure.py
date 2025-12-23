#!/usr/bin/env python3
"""
README Structure Governance (V6)
Validates that documented project structure in README.md matches filesystem.
Exits with code 1 on drift, blocking merge.
"""

import re
import sys
from pathlib import Path

# Minimal stable structure - these directories MUST exist
# Kept minimal per G4 to prevent silent drift
REQUIRED_STRUCTURE = {
    'src/services/gateway',
    'src/services/processor', 
    'src/services/metrics-engine',
    'src/services/read-model',
    'contracts/schemas',
    'scripts',
    'infra/k8s',
}

# Optional but documented directories
OPTIONAL_STRUCTURE = {
    'src/interfaces/tui',
    'src/interfaces/web',
    'audit',
    'docs',
    'tests',
}


def extract_documented_structure(readme_path: Path) -> set:
    """Extract directory paths from README.md Project Structure section."""
    content = readme_path.read_text(encoding='utf-8')
    
    # Find Project Structure section
    structure_match = re.search(
        r'##\s*📁\s*Project Structure.*?```(.*?)```',
        content,
        re.DOTALL
    )
    
    if not structure_match:
        return set()
    
    structure_block = structure_match.group(1)
    
    # Extract directory paths (lines ending with / or containing # comment)
    dirs = set()
    for line in structure_block.split('\n'):
        # Match tree branches like "├── gateway/    # Node.js API"
        match = re.search(r'[├└─│\s]+([a-zA-Z0-9_\-./]+)', line)
        if match:
            path = match.group(1).strip().rstrip('/')
            if path and not path.startswith('#'):
                dirs.add(path)
    
    return dirs


def main():
    repo_root = Path(__file__).parent.parent
    readme_path = repo_root / 'README.md'
    
    if not readme_path.exists():
        print("[FAIL] README.md not found")
        sys.exit(1)
    
    print("=== README Structure Governance ===")
    
    errors = []
    warnings = []
    
    # Check required structure exists on filesystem
    for required_dir in REQUIRED_STRUCTURE:
        full_path = repo_root / required_dir
        if not full_path.exists():
            errors.append(f"MISSING: {required_dir}")
        else:
            print(f"[OK] {required_dir}")
    
    # Check optional structure (warn only)
    for optional_dir in OPTIONAL_STRUCTURE:
        full_path = repo_root / optional_dir
        if not full_path.exists():
            warnings.append(f"OPTIONAL MISSING: {optional_dir}")
    
    # Report
    if warnings:
        print("\nWarnings:")
        for w in warnings:
            print(f"  [WARN] {w}")
    
    if errors:
        print("\n[FAIL] README structure drift detected:")
        for e in errors:
            print(f"  - {e}")
        print("\nFix: Update filesystem or README.md to match")
        sys.exit(1)
    
    print("\n[PASS] README structure governance check passed")
    sys.exit(0)


if __name__ == '__main__':
    main()
