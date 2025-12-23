#!/bin/bash
# Bazel wrapper for vitest - runs from gateway service directory
set -e
cd "$BUILD_WORKSPACE_DIRECTORY/src/services/gateway"
npm test
