#!/usr/bin/env bash
# Runs the full lifecycle test suite inside a disposable Docker container
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if ! command -v docker &> /dev/null; then
    echo "[-] Error: docker is not installed or not available in PATH."
    echo "[!] Use scripts/test_sandbox.sh for host-isolated acceptance testing without Docker."
    exit 1
fi

echo "=========================================================="
echo "  Running Isolated Acceptance Test in Docker Container"
echo "=========================================================="

IMAGE_TAG="experienced-llms-test:latest"

echo "[+] Building test image..."
docker build -f "${SCRIPT_DIR}/Dockerfile.test" -t "${IMAGE_TAG}" "${SCRIPT_DIR}"

echo "[+] Running container test suite..."
docker run --rm "${IMAGE_TAG}"

echo ""
echo "[✔] Isolated Docker tests completed successfully!"
