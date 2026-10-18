#!/usr/bin/env bash
# Automated Verification for rc2 -> Latest (1.0.0rc4+) Migration
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

TEST_SANDBOX="$(mktemp -d /tmp/exp_rc2_test_XXXXXX)"
trap 'rm -rf "${TEST_SANDBOX}"' EXIT

mkdir -p "${TEST_SANDBOX}/raw_days"

cat << 'EOF' > "${TEST_SANDBOX}/EXPERIENCE.md"
# Master AI Experience

## 1. User Directives & Preferences
- (global): Always run npm inside WSL *(Reason: Windows path collisions)*

## 2. Defensive Safeguards & Heuristics
- Check git status before commit *(Reason: Prevent uncommitted changes)*

## 4. Anti-Patterns & Critical Gotchas
- (windows): Do not use non-ASCII characters in PowerShell scripts *(Reason: PS 5.1 quote corruption)*
EOF

cat << 'EOF' > "${TEST_SANDBOX}/raw_days/2026-10-18.md"
## 2. Defensive Safeguards & Heuristics
- (security): Never commit hardcoded API keys
EOF

echo "[+] Running migrate_rc2_to_latest.py in sandbox: ${TEST_SANDBOX}"
python3 "${REPO_ROOT}/scripts/migrate_rc2_to_latest.py" --home "${TEST_SANDBOX}"

# Verify SQLite database
python3 -c "
import sqlite3
conn = sqlite3.connect('${TEST_SANDBOX}/memory.db')
rows = conn.execute('SELECT id, category, scope, rule_statement, is_consolidated, status FROM memory_facts ORDER BY id').fetchall()
assert len(rows) == 4, f'Expected 4 facts, got {len(rows)}'
assert rows[0][3] == 'Always run npm inside WSL'
assert rows[0][4] == 1, 'Expected is_consolidated=1 for active rule'
assert rows[3][3] == 'Never commit hardcoded API keys'
assert rows[3][4] == 0, 'Expected is_consolidated=0 for pending intraday rule'

# Verify migration record
mig = conn.execute('SELECT version FROM schema_migrations').fetchone()
assert mig[0] == '001_1_0_0rc3_baseline', f'Unexpected migration {mig}'
conn.close()
print('[✔] SQLite database verified with all 4 facts and baseline migration!')
"

# Verify re-rendered EXPERIENCE.md
grep -q "Always run npm inside WSL" "${TEST_SANDBOX}/EXPERIENCE.md"
grep -q "## 1. User Directives & Preferences" "${TEST_SANDBOX}/EXPERIENCE.md"
echo "[✔] EXPERIENCE.md cleanly re-rendered!"

# Verify backup
ls -d "${TEST_SANDBOX}"/backup_rc2_* > /dev/null
echo "[✔] Backup archive directory verified!"

echo "=========================================================="
echo "[SUCCESS] rc2 migration test passed 100%!"
echo "=========================================================="
