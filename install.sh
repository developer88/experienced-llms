#!/usr/bin/env bash
# One-Click Installer for Experienced LLMs (Linux / macOS / WSL)
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="${HOME}/.local/bin"
SKILL_DIR="${HOME}/.gemini/config/skills/experienced-llms"
EXPERIENCED_HOME="${HOME}/.experienced-llms"

echo "============================================="
echo "  Experienced LLMs: One-Click Installer"
echo "============================================="

# 1. Check Python 3
if ! command -v python3 &> /dev/null; then
    echo "[-] Error: python3 is required but was not found."
    exit 1
fi
echo "[+] Python 3 detected: $(python3 --version)"

# 2. Initialize directories and database
echo "[+] Initializing Experienced LLMs storage at ${EXPERIENCED_HOME}..."
mkdir -p "${EXPERIENCED_HOME}"
python3 -c "import sys; sys.path.insert(0, '${SCRIPT_DIR}'); from src.config import ensure_directories; ensure_directories(); from src.db import init_db; init_db()"

# 3. Install CLI wrapper to ~/.local/bin
mkdir -p "${BIN_DIR}"
WRAPPER_PATH="${BIN_DIR}/experienced-llms"

cat <<EOF > "${WRAPPER_PATH}"
#!/usr/bin/env bash
export PYTHONPATH="${SCRIPT_DIR}:\${PYTHONPATH}"
exec python3 -m src.cli "\$@"
EOF
chmod +x "${WRAPPER_PATH}"
echo "[+] Installed CLI executable: ${WRAPPER_PATH}"

# Check PATH
if [[ ":$PATH:" != *":${BIN_DIR}:"* ]]; then
    echo "[!] Tip: Add ${BIN_DIR} to your PATH in ~/.bashrc or ~/.zshrc:"
    echo "    export PATH=\"${BIN_DIR}:\$PATH\""
fi

# 4. Integrate with AI Agents (Antigravity, Claude, Copilot, Cursor, Pi)
echo "[+] Configuring AI Agent integrations..."
export PYTHONPATH="${SCRIPT_DIR}:${PYTHONPATH}"
python3 -m src.cli integrate --target all

# 5. Setup Nightly Consolidation Cron (Default: 02:00 AM)
echo "[+] Installing nightly consolidation schedule..."
export PYTHONPATH="${SCRIPT_DIR}:${PYTHONPATH}"
python3 -m src.cli schedule --install --time "02:00"

echo ""
echo "============================================="
echo "[✔] Experienced LLMs successfully installed!"
echo "============================================="
echo ""
echo "Quick Commands:"
echo "  experienced-llms log \"Always run npm in WSL\" --category user_preference"
echo "  experienced-llms show"
echo "  experienced-llms status"
echo "  experienced-llms setup --provider [gemini|claude|openai|ollama]"
echo "  experienced-llms consolidate"
echo ""
