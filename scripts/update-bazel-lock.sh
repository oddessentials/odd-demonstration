#!/bin/bash
# update-bazel-lock.sh - Standard recovery path for stale MODULE.bazel.lock
#
# Usage: ./scripts/update-bazel-lock.sh
#
# This script regenerates the MODULE.bazel.lock file to match the current
# MODULE.bazel configuration. Run this locally when CI fails with:
#   "lockfile is out of date"
#
# After running, commit the updated lockfile.

set -euo pipefail

echo "🔄 Updating MODULE.bazel.lock..."
bazel mod deps --lockfile_mode=update

echo "✅ Lockfile updated successfully."
echo ""
echo "Next steps:"
echo "  1. Review the changes: git diff MODULE.bazel.lock"
echo "  2. Commit: git add MODULE.bazel.lock && git commit -m 'chore: update MODULE.bazel.lock'"
echo "  3. Push and re-run CI"
