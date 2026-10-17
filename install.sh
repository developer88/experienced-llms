#!/usr/bin/env bash
# ==============================================================================
# Experienced LLMs: One-Line Autonomous Installer (Linux / macOS / WSL)
# Usage:
#   curl -fsSL https://github.com/developer88/experienced-llms/releases/latest/download/install.sh | bash
# ==============================================================================
set -e

REPO="developer88/experienced-llms"
VERSION_TAG="latest"
BIN_DIR="${HOME}/.local/bin"
EXPERIENCED_HOME="${HOME}/.experienced-llms"
TARGET_BIN="${BIN_DIR}/experienced-llms"

echo "=========================================================="
echo "  Experienced LLMs: Autonomous Cognitive Memory Installer"
echo "=========================================================="

mkdir -p "${BIN_DIR}"
mkdir -p "${EXPERIENCED_HOME}"

# 1. Detect OS and Architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "${OS}" in
    Linux*)
        # Detect musl vs glibc
        if ldd /bin/ls 2>&1 | grep -qi "musl"; then
            LIBC_FLAVOR="musl"
        else
            LIBC_FLAVOR="gnu"
        fi

        case "${ARCH}" in
            x86_64|amd64)
                ASSET_NAME="experienced-llms-linux-x86_64-${LIBC_FLAVOR}"
                ;;
            aarch64|arm64)
                ASSET_NAME="experienced-llms-linux-aarch64-${LIBC_FLAVOR}"
                ;;
            *)
                echo "[-] Unsupported Linux architecture: ${ARCH}"
                exit 1
                ;;
        esac
        ;;
    Darwin*)
        case "${ARCH}" in
            x86_64)
                ASSET_NAME="experienced-llms-darwin-x86_64"
                ;;
            arm64|aarch64)
                ASSET_NAME="experienced-llms-darwin-arm64"
                ;;
            *)
                echo "[-] Unsupported macOS architecture: ${ARCH}"
                exit 1
                ;;
        esac
        ;;
    *)
        echo "[-] Unsupported operating system: ${OS} (For Windows 11, use install.ps1 in PowerShell)"
        exit 1
        ;;
esac

echo "[+] Detected platform: ${OS} (${ARCH})"

# 2. Check for local build or prebuilt artifact first (offline / source repo installation)
INSTALLED_LOCAL=false
SCRIPT_DIR=""
if [ -n "${BASH_SOURCE[0]}" ] && [ -f "${BASH_SOURCE[0]}" ]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fi

if [ -n "${SCRIPT_DIR}" ]; then
    if [ -f "${SCRIPT_DIR}/dist/${ASSET_NAME}" ]; then
        echo "[+] Installing from local dist directory: ${SCRIPT_DIR}/dist/${ASSET_NAME}"
        cp "${SCRIPT_DIR}/dist/${ASSET_NAME}" "${TARGET_BIN}"
        chmod +x "${TARGET_BIN}"
        INSTALLED_LOCAL=true
    elif [ -f "${SCRIPT_DIR}/target/release/experienced-llms" ]; then
        echo "[+] Installing from local Cargo release build: ${SCRIPT_DIR}/target/release/experienced-llms"
        cp "${SCRIPT_DIR}/target/release/experienced-llms" "${TARGET_BIN}"
        chmod +x "${TARGET_BIN}"
        INSTALLED_LOCAL=true
    elif [ -f "${SCRIPT_DIR}/Cargo.toml" ] && command -v cargo >/dev/null 2>&1; then
        echo "[+] Compiling standalone binary from local source via Cargo..."
        (cd "${SCRIPT_DIR}" && cargo build --release)
        cp "${SCRIPT_DIR}/target/release/experienced-llms" "${TARGET_BIN}"
        chmod +x "${TARGET_BIN}"
        INSTALLED_LOCAL=true
    fi
fi

# 3. If not installed from local artifacts, download prebuilt binary from latest GitHub release
if [ "${INSTALLED_LOCAL}" = false ]; then
    DOWNLOAD_URL="https://github.com/${REPO}/releases/${VERSION_TAG}/download/${ASSET_NAME}"
    echo "[+] Fetching binary from: ${DOWNLOAD_URL}"

    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "${DOWNLOAD_URL}" -o "${TARGET_BIN}" || {
            # Fallback to gnu if musl download fails
            FALLBACK_ASSET="experienced-llms-linux-x86_64-gnu"
            FALLBACK_URL="https://github.com/${REPO}/releases/${VERSION_TAG}/download/${FALLBACK_ASSET}"
            echo "[*] Trying fallback: ${FALLBACK_URL}"
            curl -fsSL "${FALLBACK_URL}" -o "${TARGET_BIN}"
        }
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "${TARGET_BIN}" "${DOWNLOAD_URL}"
    else
        echo "[-] Error: curl or wget is required to download the binary."
        exit 1
    fi

    chmod +x "${TARGET_BIN}"
fi

echo "[✔] Installed standalone native binary to: ${TARGET_BIN}"

# 4. PATH Configuration check
if [[ ":$PATH:" != *":${BIN_DIR}:"* ]]; then
    export PATH="${BIN_DIR}:${PATH}"
    SHELL_PROFILE=""
    if [ -n "${ZSH_VERSION}" ] || [ -f "${HOME}/.zshrc" ]; then
        SHELL_PROFILE="${HOME}/.zshrc"
    elif [ -f "${HOME}/.bashrc" ]; then
        SHELL_PROFILE="${HOME}/.bashrc"
    elif [ -f "${HOME}/.profile" ]; then
        SHELL_PROFILE="${HOME}/.profile"
    fi

    if [ -n "${SHELL_PROFILE}" ]; then
        if ! grep -qs 'export PATH=.*\.local/bin' "${SHELL_PROFILE}"; then
            echo "" >> "${SHELL_PROFILE}"
            echo '# Experienced LLMs PATH' >> "${SHELL_PROFILE}"
            echo 'export PATH="$HOME/.local/bin:$PATH"' >> "${SHELL_PROFILE}"
            echo "[+] Added ~/.local/bin to PATH in ${SHELL_PROFILE}"
        fi
    fi
fi

# 5. Initialize SQLite storage and directories
echo "[+] Initializing Experienced LLMs storage..."
"${TARGET_BIN}" init

# 6. Configure AI Agent integrations (Antigravity, Claude, Copilot, Cursor, Pi)
echo "[+] Configuring AI Agent integrations..."
"${TARGET_BIN}" integrate --target all

# 7. Configure Nightly Consolidation Schedule
echo "[+] Installing nightly consolidation schedule..."
"${TARGET_BIN}" schedule --install --time "02:00"

echo ""
echo "=========================================================="
echo "  [✔] Experienced LLMs Successfully Installed & Active!"
echo "=========================================================="
"${TARGET_BIN}" status
echo ""
echo "Quick Commands:"
echo "  experienced-llms log \"Always run npm in WSL\" --category user_preference"
echo "  experienced-llms show"
echo "  experienced-llms status"
echo "  experienced-llms setup --provider [gemini|claude|openai|ollama]"
echo "  experienced-llms consolidate"
echo "=========================================================="
