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
# Step 5: Template Fail-Hard Guarantee & Missing LLM Validation
# ------------------------------------------------------------------------------
echo ""
echo "--- [Step 5] Testing Missing LLM Validation and Template Fail-Hard Guarantee ---"

# 5a. Consolidate without LLM must fail cleanly
set +e
NO_LLM_OUTPUT="$("${BIN}" consolidate 2>&1)"
NO_LLM_EXIT=$?
set -e

if [ "${NO_LLM_EXIT}" -eq 0 ]; then
    echo "[-] FAILED: consolidate should fail when no LLM command is provided!"
    exit 1
fi
if echo "${NO_LLM_OUTPUT}" | grep -qi "No external LLM CLI command configured"; then
    echo "[✔] Step 5a PASSED: Engine cleanly required --llm command:"
    echo "    ${NO_LLM_OUTPUT}"
else
    echo "[-] FAILED: Unexpected error message when no LLM provided:"
    echo "    ${NO_LLM_OUTPUT}"
    exit 1
fi

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

# 5b. Consolidate with LLM but missing template must fail hard
rm -f "${EXPERIENCED_LLMS_HOME}/template.md"

set +e
FAIL_OUTPUT="$("${BIN}" consolidate --llm "bash ${MOCK_LLM_SCRIPT}" 2>&1)"
FAIL_EXIT=$?
set -e

if [ "${FAIL_EXIT}" -eq 0 ]; then
    echo "[-] FAILED: consolidate should have failed when template.md is missing!"
    exit 1
fi

if echo "${FAIL_OUTPUT}" | grep -qi "template"; then
    echo "[✔] Step 5b PASSED: Engine failed hard with expected template error:"
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
echo "[✔] Step 5c PASSED: 'init' restored missing template.md."

# ------------------------------------------------------------------------------
# Step 6: Consolidation via External LLM Pipe
# ------------------------------------------------------------------------------
echo ""
echo "--- [Step 6] Testing consolidation with external LLM CLI mock ---"

"${BIN}" consolidate --llm "bash ${MOCK_LLM_SCRIPT}"

SHOW_OUTPUT="$(cat "${EXPERIENCED_LLMS_HOME}/EXPERIENCE.md")"
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

RENDER_SHOW="$(cat "${EXPERIENCED_LLMS_HOME}/EXPERIENCE.md")"
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

# ------------------------------------------------------------------------------
# Step 9: Verification of Removed Commands, Streamlined CLI & Self-Healing
# ------------------------------------------------------------------------------
echo ""
echo "--- [Step 9] Verifying removal of show/setup/prompt, and consolidate self-healing ---"

set +e
"${BIN}" consolidate --provider gemini > /dev/null 2>&1
PROVIDER_EXIT=$?
set -e

if [ "${PROVIDER_EXIT}" -eq 0 ]; then
    echo "[-] FAILED: --provider should be rejected as an unknown argument on consolidate!"
    exit 1
fi

set +e
"${BIN}" consolidate --template "custom.md" > /dev/null 2>&1
TEMPLATE_FLAG_EXIT=$?
set -e

if [ "${TEMPLATE_FLAG_EXIT}" -eq 0 ]; then
    echo "[-] FAILED: --template should be rejected as an unknown argument on consolidate!"
    exit 1
fi

# Verify removed commands (show, setup, prompt) are rejected
set +e
"${BIN}" show > /dev/null 2>&1
SHOW_EXIT=$?
"${BIN}" setup --llm "test" > /dev/null 2>&1
SETUP_EXIT=$?
"${BIN}" prompt --target worker > /dev/null 2>&1
PROMPT_EXIT=$?
set -e

if [ "${SHOW_EXIT}" -eq 0 ] || [ "${SETUP_EXIT}" -eq 0 ] || [ "${PROMPT_EXIT}" -eq 0 ]; then
    echo "[-] FAILED: Obsolete commands (show, setup, prompt) must be rejected!"
    exit 1
fi

# Configure default external LLM command directly in config.json
python3 -c "
import json, os
cfg_file = '${EXPERIENCED_LLMS_HOME}/config.json'
cfg = {}
if os.path.exists(cfg_file):
    with open(cfg_file, 'r') as f:
        cfg = json.load(f)
cfg['external_llm_cmd'] = 'bash ${MOCK_LLM_SCRIPT}'
with open(cfg_file, 'w') as f:
    json.dump(cfg, f, indent=2)
"

# Verify consolidation runs cleanly using configured default command without explicit --llm flag
"${BIN}" consolidate

# Verify self-healing: remove EXPERIENCE.md and verify consolidate regenerates it
rm -f "${EXPERIENCED_LLMS_HOME}/EXPERIENCE.md"
"${BIN}" consolidate

if [ ! -f "${EXPERIENCED_LLMS_HOME}/EXPERIENCE.md" ]; then
    echo "[-] FAILED: consolidate must self-heal missing EXPERIENCE.md!"
    exit 1
fi

echo "[✔] Step 9 PASSED: Removed commands verified and consolidate self-healing confirmed."

echo ""
echo "================================================================="
echo "[SUCCESS] All Manual Acceptance checks passed with flying colors!"
echo "================================================================="
