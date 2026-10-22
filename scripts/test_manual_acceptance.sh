#!/usr/bin/env bash
# ==============================================================================
# Comprehensive Manual Acceptance Test Suite for Experienced LLMs
# Verifies end-to-end CLI workflows, template fail-hard behavior, idempotency,
# and zero-token-waste outputs in an isolated environment.
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "================================================================="
echo "   Experienced LLMs: End-to-End Manual Acceptance Test"
echo "================================================================="

cd "${REPO_ROOT}"

# Build target binary
cargo build --quiet

BIN="${REPO_ROOT}/target/debug/experienced-llms"
if [ ! -x "${BIN}" ]; then
    echo "[-] Error: Binary not found at ${BIN}"
    exit 1
fi

ACCEPT_DIR="$(mktemp -d /tmp/exp_acceptance_XXXXXX)"
trap 'rm -rf "${ACCEPT_DIR}"' EXIT

export EXPERIENCED_LLMS_HOME="${ACCEPT_DIR}/.experienced-llms"
export HOME="${ACCEPT_DIR}"

echo "[+] Using isolated test directory: ${ACCEPT_DIR}"
echo "[+] EXPERIENCED_LLMS_HOME: ${EXPERIENCED_LLMS_HOME}"

# ------------------------------------------------------------------------------
# Step 1: Initialize Storage & Baseline State
# ------------------------------------------------------------------------------
echo ""
echo "--- [Step 1] Verifying 'init' on clean directory ---"
"${BIN}" init

if [ ! -f "${EXPERIENCED_LLMS_HOME}/memory.db" ]; then
    echo "[-] FAILED: memory.db not created"
    exit 1
fi

if [ ! -f "${EXPERIENCED_LLMS_HOME}/template.md" ]; then
    echo "[-] FAILED: template.md not created"
    exit 1
fi

if [ ! -f "${EXPERIENCED_LLMS_HOME}/EXPERIENCE.md" ]; then
    echo "[-] FAILED: EXPERIENCE.md not rendered"
    exit 1
fi

echo "[✔] Step 1 PASSED: init created memory.db, template.md, and EXPERIENCE.md."

# ------------------------------------------------------------------------------
# Step 2: Log Daytime Operational Rules
# ------------------------------------------------------------------------------
echo ""
echo "--- [Step 2] Testing 'log' command ---"
"${BIN}" log "Always run cargo test in WSL" \
    --category user_preference \
    --scope rust \
    --reason "Windows environment lacks cargo toolchain"

"${BIN}" log "Always verify .env is in .gitignore before commit" \
    --category defensive_heuristic \
    --scope security \
    --reason "Prevent API secret leaks"

echo "[✔] Step 2 PASSED: Logged 2 heuristics."

# ------------------------------------------------------------------------------
# Step 3: Inspect Status & Verify Pending State
# ------------------------------------------------------------------------------
echo ""
echo "--- [Step 3] Testing 'status' command ---"
STATUS_OUTPUT="$("${BIN}" status)"
echo "${STATUS_OUTPUT}"

if ! echo "${STATUS_OUTPUT}" | grep -q "Always run cargo test in WSL"; then
    echo "[-] FAILED: Fact 1 not visible in status output"
    exit 1
fi

if ! echo "${STATUS_OUTPUT}" | grep -q "Always verify .env is in .gitignore"; then
    echo "[-] FAILED: Fact 2 not visible in status output"
    exit 1
fi

echo "[✔] Step 3 PASSED: Status correctly reflects active/pending facts."

# ------------------------------------------------------------------------------
# Step 4: Idempotency Verification (Re-running 'init' on Existing DB)
# ------------------------------------------------------------------------------
echo ""
echo "--- [Step 4] Verifying 'init' idempotency (existing data must NOT be lost) ---"
"${BIN}" init

STATUS_OUTPUT_AFTER="$("${BIN}" status)"
if ! echo "${STATUS_OUTPUT_AFTER}" | grep -q "Always run cargo test in WSL"; then
    echo "[-] FAILED: Existing facts were lost after re-running 'init'!"
    exit 1
fi

echo "[✔] Step 4 PASSED: Re-running 'init' is safe and preserved existing facts."

# ------------------------------------------------------------------------------
# Step 5: Template Fail-Hard Guarantee
# ------------------------------------------------------------------------------
echo ""
echo "--- [Step 5] Testing Template Fail-Hard Guarantee ---"
rm -f "${EXPERIENCED_LLMS_HOME}/template.md"

set +e
FAIL_OUTPUT="$("${BIN}" consolidate 2>&1)"
FAIL_EXIT=$?
set -e

if [ "${FAIL_EXIT}" -eq 0 ]; then
    echo "[-] FAILED: consolidate should have failed when template.md is missing!"
    exit 1
fi

if echo "${FAIL_OUTPUT}" | grep -qi "template"; then
    echo "[✔] Step 5 PASSED: Engine failed hard with expected template error:"
    echo "    ${FAIL_OUTPUT}"
else
    echo "[-] FAILED: Error message did not mention template:"
    echo "    ${FAIL_OUTPUT}"
    exit 1
fi

# Restore template via init
"${BIN}" init
if [ ! -f "${EXPERIENCED_LLMS_HOME}/template.md" ]; then
    echo "[-] FAILED: 'init' did not restore missing template.md"
    exit 1
fi
echo "[✔] Step 5b PASSED: 'init' restored missing template.md."

# ------------------------------------------------------------------------------
# Step 6: Consolidation via External LLM Pipe
# ------------------------------------------------------------------------------
echo ""
echo "--- [Step 6] Testing consolidation with external LLM CLI mock ---"

# Create a mock LLM that produces compliant JSON
MOCK_LLM_SCRIPT="${ACCEPT_DIR}/mock_llm.sh"
cat << 'EOF' > "${MOCK_LLM_SCRIPT}"
#!/usr/bin/env bash
cat << 'JSON'
{
  "updated_experience_markdown": "# Master AI Experience\n\n## 1. User Directives & Preferences\n- Always run cargo test in WSL\n\n## 2. Defensive Safeguards & Heuristics\n- Always verify .env is in .gitignore before commit",
  "processed_pending_ids": [1, 2],
  "superseded_fact_ids": [],
  "summary_of_changes": "Consolidated WSL and security rules into master experience."
}
JSON
EOF
chmod +x "${MOCK_LLM_SCRIPT}"

"${BIN}" consolidate --llm "bash ${MOCK_LLM_SCRIPT}"

SHOW_OUTPUT="$("${BIN}" show)"
echo "${SHOW_OUTPUT}"

if ! echo "${SHOW_OUTPUT}" | grep -q "Always run cargo test in WSL"; then
    echo "[-] FAILED: Consolidated rule 1 not present in EXPERIENCE.md"
    exit 1
fi

if ! echo "${SHOW_OUTPUT}" | grep -q "Always verify .env is in .gitignore"; then
    echo "[-] FAILED: Consolidated rule 2 not present in EXPERIENCE.md"
    exit 1
fi

echo "[✔] Step 6 PASSED: Consolidation completed and updated EXPERIENCE.md."

# ------------------------------------------------------------------------------
# Step 7: Render Command Verification
# ------------------------------------------------------------------------------
echo ""
echo "--- [Step 7] Testing 'render' command ---"
# Remove EXPERIENCE.md and rebuild directly from SQLite
rm -f "${EXPERIENCED_LLMS_HOME}/EXPERIENCE.md"

"${BIN}" render

if [ ! -f "${EXPERIENCED_LLMS_HOME}/EXPERIENCE.md" ]; then
    echo "[-] FAILED: render did not create EXPERIENCE.md"
    exit 1
fi

RENDER_SHOW="$("${BIN}" show)"
if ! echo "${RENDER_SHOW}" | grep -q "Always run cargo test in WSL"; then
    echo "[-] FAILED: Rendered markdown missing facts from SQLite"
    exit 1
fi

echo "[✔] Step 7 PASSED: 'render' successfully rebuilt EXPERIENCE.md from SQLite."

# ------------------------------------------------------------------------------
# Step 8: Verification of Removed Pathway B Flags (--prepare, --apply)
# ------------------------------------------------------------------------------
echo ""
echo "--- [Step 8] Verifying removal of legacy Pathway B flags ---"

set +e
"${BIN}" consolidate --prepare > /dev/null 2>&1
PREP_EXIT=$?
set -e

if [ "${PREP_EXIT}" -eq 0 ]; then
    echo "[-] FAILED: --prepare should be rejected as an unknown argument!"
    exit 1
fi

set +e
"${BIN}" consolidate --apply "{}" > /dev/null 2>&1
APPLY_EXIT=$?
set -e

if [ "${APPLY_EXIT}" -eq 0 ]; then
    echo "[-] FAILED: --apply should be rejected as an unknown argument!"
    exit 1
fi

echo "[✔] Step 8 PASSED: --prepare and --apply flags are completely rejected."

echo ""
echo "================================================================="
echo "[SUCCESS] All Manual Acceptance checks passed with flying colors!"
echo "================================================================="
