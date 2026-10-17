#!/usr/bin/env bash
# Hermetic Containerized Test Pipeline for Experienced LLMs (0 Cloud Workers)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "================================================================="
echo "   Experienced LLMs: Local Docker Test Pipeline"
echo "   (Zero Cloud Workers - 100% Hermetic Container Execution)"
echo "================================================================="

cd "${REPO_ROOT}"

echo "[+] Building isolated test container (experienced-llms-test)..."
docker build -f Dockerfile.test -t experienced-llms-test .

echo "[+] Running automated test suite inside container..."
docker run --rm experienced-llms-test

echo "[✓] Local Docker test pipeline passed successfully!"
