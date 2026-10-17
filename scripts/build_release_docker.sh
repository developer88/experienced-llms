#!/usr/bin/env bash
# Local Multi-Target Release Builder (Zero Cloud GitHub Workers)
# Compiles standalone Linux (musl & gnu) and Windows binaries locally and packages to ./dist/
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
DIST_DIR="${REPO_ROOT}/dist"

echo "================================================================="
echo "   Experienced LLMs: Local Multi-Target Release Builder"
echo "   (Zero Cloud Workers - 100% Local Docker Cross-Compilation)"
echo "================================================================="

cd "${REPO_ROOT}"

mkdir -p "${DIST_DIR}"
rm -rf "${DIST_DIR:?}"/*

echo "[+] Building multi-target release container (experienced-llms-builder)..."
docker build -f Dockerfile.release -t experienced-llms-builder .

echo "[+] Extracting compiled binaries and release archives to ./dist/..."
docker run --rm -v "${DIST_DIR}:/output" experienced-llms-builder

echo ""
echo "================================================================="
echo "   COMPILED RELEASE ARTIFACTS (./dist/)"
echo "================================================================="
ls -lh "${DIST_DIR}"

echo ""
echo "=== SHA-256 Checksums ==="
cat "${DIST_DIR}/SHA256SUMS.txt"
echo "================================================================="
echo "[✓] Local release compilation completed with 0 GitHub Actions minutes used!"
