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

# Download helper using curl or wget
download_file() {
    local url="$1"
    local dest="$2"
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "${url}" -o "${dest}" || return 1
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "${dest}" "${url}" || return 1
    else
        return 1
    fi
    return 0
}

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

# 2. Check for local build or prebuilt artifact first (source repo installation)
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

    DOWNLOAD_SUCCESS=false
    if download_file "${DOWNLOAD_URL}" "${TARGET_BIN}"; then
        DOWNLOAD_SUCCESS=true
    elif [ "${OS}" = "Linux" ] && [ "${LIBC_FLAVOR}" = "musl" ]; then
        FALLBACK_ASSET="experienced-llms-linux-x86_64-gnu"
        FALLBACK_URL="https://github.com/${REPO}/releases/${VERSION_TAG}/download/${FALLBACK_ASSET}"
        echo "[*] Trying GNU libc fallback: ${FALLBACK_URL}"
        if download_file "${FALLBACK_URL}" "${TARGET_BIN}"; then
            DOWNLOAD_SUCCESS=true
        fi
    fi

    if [ "${DOWNLOAD_SUCCESS}" = false ]; then
        echo ""
        echo "[!] Prebuilt binary not available for ${OS} (${ARCH})."
        if command -v cargo >/dev/null 2>&1; then
            echo "[+] Cargo detected! Building and installing directly from source via Cargo..."
            cargo install --git "https://github.com/${REPO}" --bin experienced-llms --root "${HOME}/.local"
            DOWNLOAD_SUCCESS=true
        else
            echo "[-] Error: No prebuilt binary found and Cargo is not installed on this system."
            echo ""
            echo "To build Experienced LLMs from source for your platform:"
            echo "  1. Install Rust toolchain: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
            echo "  2. Install via Cargo:      cargo install --git https://github.com/${REPO}"
            echo ""
            exit 1
        fi
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
if "${TARGET_BIN}" init --help 2>&1 | grep -q -- '--yes'; then
    "${TARGET_BIN}" init --yes
else
    "${TARGET_BIN}" init
fi

# 6. Configure AI Agent integrations (Antigravity skill)
echo "[+] Configuring AI Agent integrations..."
"${TARGET_BIN}" integrate --target all

echo ""
echo "=========================================================="
echo "  [✔] Experienced LLMs Successfully Installed & Active!"
echo "=========================================================="
"${TARGET_BIN}" status
echo ""
echo "Quick Commands:"
echo "  experienced-llms log \"Always run npm in WSL\" --category user_preference"
echo "  experienced-llms status"
echo "  experienced-llms consolidate --llm \"<cmd>\""
echo "  experienced-llms update"
echo "=========================================================="
