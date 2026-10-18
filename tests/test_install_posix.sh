#!/usr/bin/env bash
# ==============================================================================
# Automated Test Suite for POSIX Installer (install.sh)
# Tests local install, remote download streaming, and fallback/error handling
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "================================================================="
echo "  Running POSIX Installer Automated Tests (install.sh)"
echo "================================================================="

# Create isolated root sandbox for tests
TEST_SANDBOX="$(mktemp -d /tmp/exp_test_posix_XXXXXX)"
trap 'rm -rf "${TEST_SANDBOX}"' EXIT

echo "[+] Using isolated test sandbox: ${TEST_SANDBOX}"

# ------------------------------------------------------------------------------
# Test 1: Local Installation Mode (from repo with dist/ or target/release)
# ------------------------------------------------------------------------------
echo ""
echo "--- [Test 1] Testing Local Repo Installation Mode ---"
SANDBOX1="${TEST_SANDBOX}/test1_local"
mkdir -p "${SANDBOX1}"
touch "${SANDBOX1}/.bashrc"

(
    export HOME="${SANDBOX1}"
    cd "${REPO_ROOT}"
    bash ./install.sh
)

if [ ! -x "${SANDBOX1}/.local/bin/experienced-llms" ]; then
    echo "[-] Test 1 FAILED: Binary was not installed to ~/.local/bin/experienced-llms"
    exit 1
fi

if [ ! -f "${SANDBOX1}/.experienced-llms/memory.db" ]; then
    echo "[-] Test 1 FAILED: SQLite memory.db was not initialized"
    exit 1
fi

if [ ! -f "${SANDBOX1}/.experienced-llms/EXPERIENCE.md" ]; then
    echo "[-] Test 1 FAILED: Master EXPERIENCE.md was not generated"
    exit 1
fi

if ! grep -q 'export PATH=.*\.local/bin' "${SANDBOX1}/.bashrc"; then
    echo "[-] Test 1 FAILED: PATH was not configured in ~/.bashrc"
    exit 1
fi

echo "[✔] Test 1 PASSED: Local repository installation succeeded."

# ------------------------------------------------------------------------------
# Test 2: Piped / Remote Download Mode (simulating curl ... | bash)
# ------------------------------------------------------------------------------
echo ""
echo "--- [Test 2] Testing Piped Remote Download Streaming Mode ---"
SANDBOX2="${TEST_SANDBOX}/test2_piped"
mkdir -p "${SANDBOX2}"
touch "${SANDBOX2}/.bashrc"

# We run inside an empty directory outside the repo so SCRIPT_DIR is empty
EMPTY_DIR="${TEST_SANDBOX}/empty_workspace"
mkdir -p "${EMPTY_DIR}"

(
    export HOME="${SANDBOX2}"
    cd "${EMPTY_DIR}"
    # Pipe the installer content directly via stdin to mimic: curl -fsSL ... | bash
    cat "${REPO_ROOT}/install.sh" | bash
)

if [ ! -x "${SANDBOX2}/.local/bin/experienced-llms" ]; then
    echo "[-] Test 2 FAILED: Binary was not downloaded and installed to ~/.local/bin"
    exit 1
fi

# Verify binary execution
VERSION_OUT="$("${SANDBOX2}/.local/bin/experienced-llms" --version)"
echo "[+] Downloaded binary version: ${VERSION_OUT}"

if [ ! -f "${SANDBOX2}/.experienced-llms/memory.db" ]; then
    echo "[-] Test 2 FAILED: memory.db was not initialized"
    exit 1
fi

echo "[✔] Test 2 PASSED: Piped remote download installation succeeded."

# ------------------------------------------------------------------------------
# Test 3: Download Failure & Error Handling without Cargo
# ------------------------------------------------------------------------------
echo ""
echo "--- [Test 3] Testing Download Failure & No-Cargo Instruction Message ---"
SANDBOX3="${TEST_SANDBOX}/test3_failure"
mkdir -p "${SANDBOX3}"
touch "${SANDBOX3}/.bashrc"

# Create a modified installer script pointing to non-existent repo to force download failure
# and simulate environment without cargo in PATH
MOCK_INSTALLER="${SANDBOX3}/failing_install.sh"
sed 's/REPO="developer88\/experienced-llms"/REPO="developer88\/nonexistent-repo-test-12345"/' "${REPO_ROOT}/install.sh" > "${MOCK_INSTALLER}"
chmod +x "${MOCK_INSTALLER}"

OUTPUT_LOG="${SANDBOX3}/install_failure.log"
set +e
(
    export HOME="${SANDBOX3}"
    # Strip cargo from PATH to test no-cargo fallback instructions
    export PATH="/usr/bin:/bin:/usr/sbin:/sbin"
    cd "${EMPTY_DIR}"
    bash "${MOCK_INSTALLER}" > "${OUTPUT_LOG}" 2>&1
)
EXIT_CODE=$?
set -e

if [ "${EXIT_CODE}" -eq 0 ]; then
    echo "[-] Test 3 FAILED: Installer should have exited with non-zero code on download failure."
    cat "${OUTPUT_LOG}"
    exit 1
fi

if grep -q "No prebuilt binary found and Cargo is not installed" "${OUTPUT_LOG}" && \
   grep -q "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh" "${OUTPUT_LOG}"; then
    echo "[✔] Test 3 PASSED: Expected error message and Rustup instructions cleanly displayed."
else
    echo "[-] Test 3 FAILED: Expected instructions not found in output:"
    cat "${OUTPUT_LOG}"
    exit 1
fi

echo ""
echo "================================================================="
echo "[SUCCESS] All POSIX installer test suites passed perfectly!"
echo "================================================================="
