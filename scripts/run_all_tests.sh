#!/usr/bin/env bash
# Pre-Release Master Test Runner for Experienced LLMs (100% Native Rust)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "================================================================="
echo "   Experienced LLMs: Pre-Release Integration Test Runner"
echo "================================================================="

cd "${REPO_ROOT}"

echo "[1/4] Running Rust Cargo test suite..."
cargo test

echo "[2/4] Running POSIX installer automated test suite..."
bash tests/test_install_posix.sh

echo "[3/4] Running Isolated Acceptance Sandbox..."
bash scripts/test_sandbox.sh

echo "[4/4] Running End-to-End Manual Acceptance Suite..."
bash scripts/test_manual_acceptance.sh

echo "================================================================="
echo "[SUCCESS] All native Rust and installer test suites passed!"
echo "================================================================="
