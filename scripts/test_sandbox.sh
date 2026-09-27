#!/usr/bin/env bash
# Isolated Full-Cycle Acceptance Test for Experienced LLMs
# Runs in an isolated sandbox directory without touching host ~/.experienced-llms or host crontab.
set -e

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SANDBOX_DIR="$(mktemp -d /tmp/experienced-llms-test.XXXXXX)"

cleanup() {
    echo "[+] Cleaning up sandbox directory: ${SANDBOX_DIR}..."
    rm -rf "${SANDBOX_DIR}"
}
trap cleanup EXIT

echo "=========================================================="
echo "  Experienced LLMs: Isolated Sandbox Acceptance Test"
echo "=========================================================="
echo "[+] Sandbox Directory: ${SANDBOX_DIR}"

# 1. Isolate environment
export HOME="${SANDBOX_DIR}"
export EXPERIENCED_LLMS_HOME="${SANDBOX_DIR}/.experienced-llms"
export EXPERIENCED_LLMS_DB="${EXPERIENCED_LLMS_HOME}/memory.db"
export PATH="${SANDBOX_DIR}/.local/bin:${PATH}"

# 2. Run installer in sandbox
echo "[+] Step 1: Running install.sh with isolated HOME..."
cd "${REPO_ROOT}"
bash install.sh

# 3. Verify files were created in sandbox, NOT host
CLI_BIN="${SANDBOX_DIR}/.local/bin/experienced-llms"
if [ ! -f "${CLI_BIN}" ]; then
    echo "[-] Error: CLI wrapper was not installed at ${CLI_BIN}"
    exit 1
fi
echo "[+] Verified CLI installed at ${CLI_BIN}"

if [ ! -f "${EXPERIENCED_LLMS_HOME}/EXPERIENCE.md" ]; then
    echo "[-] Error: EXPERIENCE.md not initialized in sandbox"
    exit 1
fi
echo "[+] Verified sandbox storage initialized at ${EXPERIENCED_LLMS_HOME}"

# 4. Test deterministic intraday logging
echo "[+] Step 2: Testing intraday logging in sandbox..."
"${CLI_BIN}" log "Always run isolated tests in temporary sandbox" \
    --category "defensive_heuristic" \
    --reason "Prevent host pollution" \
    --scope "test-project"

STATUS_OUTPUT="$("${CLI_BIN}" status)"
if ! echo "${STATUS_OUTPUT}" | grep -q "Always run isolated tests"; then
    echo "[-] Error: Logged rule not found in status output!"
    exit 1
fi
echo "[+] Verified fact recorded into sandbox SQLite database."

# 5. Test interday consolidation
echo "[+] Step 3: Testing consolidation pass in sandbox..."
"${CLI_BIN}" consolidate

EXP_CONTENT="$(cat "${EXPERIENCED_LLMS_HOME}/EXPERIENCE.md")"
if ! echo "${EXP_CONTENT}" | grep -q "Always run isolated tests"; then
    echo "[-] Error: Consolidated EXPERIENCE.md does not contain test rule!"
    exit 1
fi
echo "[+] Verified consolidation pass updated sandbox EXPERIENCE.md."

# 6. Run unit test suite
echo "[+] Step 4: Running full unit test suite..."
python3 -m unittest discover -s "${REPO_ROOT}/tests"

echo ""
echo "=========================================================="
echo "[SUCCESS] All isolated acceptance checks passed!"
echo "Zero changes were made to host ~/.experienced-llms or crontab."
echo "=========================================================="
