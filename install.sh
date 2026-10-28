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

# ------------------------------------------------------------------------------
# Terminal & ANSI Color Capabilities
# ------------------------------------------------------------------------------
ESC=$(printf '\033')
RESET="${ESC}[0m"
BOLD="${ESC}[1m"
DIM="${ESC}[2m"

# Project Palette (Matches assets/logo.jpg):
# Obsidian dark background, electric cyan, neon turquoise, deep violet/indigo, vibrant cobalt, radiant white
COLOR_CYAN="${ESC}[38;2;0;229;255m"
COLOR_TURQ="${ESC}[38;2;29;233;182m"
COLOR_INDIGO="${ESC}[38;2;124;77;255m"
COLOR_VIOLET="${ESC}[38;2;179;136;255m"
COLOR_BLUE="${ESC}[38;2;41;121;255m"
COLOR_WHITE="${ESC}[38;2;255;255;255m"
COLOR_GREEN="${ESC}[38;2;0;230;118m"
COLOR_YELLOW="${ESC}[38;2;255;215;64m"
COLOR_RED="${ESC}[38;2;255;82;82m"
COLOR_GRAY="${ESC}[38;2;148;163;184m"

BLOCK="██"
SP="${RESET}  "

is_interactive() {
  [ -t 1 ] && [ "${TERM:-}" != "dumb" ]
}

test_unicode() {
  case "${LC_ALL:-${LC_CTYPE:-${LANG:-}}}" in
    *utf8*|*UTF8*|*utf-8*|*UTF-8*) return 0 ;;
  esac
  [ -n "${WT_SESSION:-}" ] && return 0
  [ -n "${TERMINAL_EMULATOR:-}" ] && return 0
  case "${TERM_PROGRAM:-}" in
    vscode|WezTerm|Apple_Terminal|iTerm.app) return 0 ;;
  esac
  return 0
}

get_visible_len() {
  local stripped
  stripped=$(printf "%s" "$1" | sed -E "s/\x1B\[[0-9;?]*[ -/]*[@-~]//g")
  printf "%s" "${#stripped}"
}

# ------------------------------------------------------------------------------
# Logo Art & Animated Sequence
# ------------------------------------------------------------------------------
MAP=(
  "...T.T..."
  "..TWCWT.."
  ".I.CWC.B."
  "IVI.C.BCB"
  "I.VCWCB.B"
  "IVI.C.BCB"
  ".I.IWB.B."
  "..IV.CB.."
  "...I.B..."
)

print_static_logo() {
  echo ""
  echo "          ██  ██"
  echo "        ██████████"
  echo "      ██  ██████  ██"
  echo "    ██████  ██  ██████"
  echo "    ██  ██████████  ██"
  echo "    ██████  ██  ██████"
  echo "      ██  ██████  ██"
  echo "        ████  ████"
  echo "          ██  ██"
  echo ""
}

render_logo_frame() {
  local step="${1:-9}"
  local flash_row="${2:--1}"
  local pulse_core="${3:-0}"

  printf "%s" "${ESC}[H"
  for r in 0 1 2 3 4 5 6 7 8; do
    local row="${MAP[$r]}"
    printf "    "
    local dr=$(( r > 4 ? r - 4 : 4 - r ))
    for c in 0 1 2 3 4 5 6 7 8; do
      local ch="${row:$c:1}"
      if [ "$ch" = "." ]; then
        printf "%s" "$SP"
        continue
      fi

      local dc=$(( c > 4 ? c - 4 : 4 - c ))
      local dist=$(( dr + dc ))
      if [ "$dist" -gt "$step" ]; then
        printf "%s" "$SP"
        continue
      fi

      if [ "$flash_row" -eq "$r" ]; then
        printf "%s%s" "$COLOR_WHITE" "$BLOCK"
      elif [ "$pulse_core" -eq 1 ] && [ "$r" -eq 4 ] && [ "$c" -eq 4 ]; then
        printf "%s%s" "$COLOR_WHITE" "$BLOCK"
      else
        case "$ch" in
          T) printf "%s%s" "$COLOR_TURQ" "$BLOCK" ;;
          W) printf "%s%s" "$COLOR_WHITE" "$BLOCK" ;;
          C) printf "%s%s" "$COLOR_CYAN" "$BLOCK" ;;
          I) printf "%s%s" "$COLOR_INDIGO" "$BLOCK" ;;
          V) printf "%s%s" "$COLOR_VIOLET" "$BLOCK" ;;
          B) printf "%s%s" "$COLOR_BLUE" "$BLOCK" ;;
          *) printf "%s" "$SP" ;;
        esac
      fi
    done
    printf "%s\n" "$RESET"
  done
}

show_experienced_logo_animation() {
  if ! is_interactive; then
    print_static_logo
    return
  fi

  trap 'printf "%s\033[?25h\n" "$RESET"; trap - INT TERM; exit 130' INT TERM
  printf '%s%s' "${ESC}[?25l" "${ESC}[2J${ESC}[H"

  # 1. Neural Spark & Axon Ripple outwards from core
  for s in 0 1 2 3 4 5 6 7 8; do
    render_logo_frame "$s" -1 0
    sleep 0.055
  done

  # 2. Central synaptic core pulse
  render_logo_frame 8 -1 1; sleep 0.12
  render_logo_frame 8 -1 0; sleep 0.09
  render_logo_frame 8 -1 1; sleep 0.14
  render_logo_frame 8 -1 0; sleep 0.10

  # 3. Quantum energy wave cascading from top to bottom
  for r in 0 1 2 3 4 5 6 7 8; do
    render_logo_frame 8 "$r" 0
    sleep 0.04
  done
  render_logo_frame 8 -1 0
  sleep 0.18

  printf '%s\033[?25h\n' "$RESET"
  trap - INT TERM
}

print_installer_title() {
  if is_interactive; then
    printf '%s  Experienced LLMs Installer%s\n%s  Autonomous Cognitive Memory & Heuristic Ledger for AI Agents%s\n\n' \
      "$BOLD" "$RESET" "$COLOR_TURQ" "$RESET"
  else
    echo "=========================================================="
    echo "  Experienced LLMs: Autonomous Cognitive Memory Installer"
    echo "=========================================================="
    echo ""
  fi
}

# ------------------------------------------------------------------------------
# Progress Reporting & Status Output
# ------------------------------------------------------------------------------
write_step_success() {
  local msg="$1"
  local detail="${2:-}"
  local check="✔"
  if ! test_unicode; then check="+"; fi
  if is_interactive; then
    local detail_str=""
    if [ -n "$detail" ]; then detail_str=" ${COLOR_GRAY}(${detail})${RESET}"; fi
    printf "  %s%s%s %s%s%s%s\n" "$COLOR_GREEN" "$check" "$RESET" "$BOLD" "$msg" "$RESET" "$detail_str"
  else
    local detail_str=""
    if [ -n "$detail" ]; then detail_str=" (${detail})"; fi
    echo "[✔] ${msg}${detail_str}"
  fi
}

write_step_warning() {
  local msg="$1"
  local warn="⚠"
  if ! test_unicode; then warn="!"; fi
  if is_interactive; then
    printf "  %s%s%s %s%s%s\n" "$COLOR_YELLOW" "$warn" "$RESET" "$COLOR_YELLOW" "$msg" "$RESET"
  else
    echo "[!] ${msg}"
  fi
}

write_step_error() {
  local msg="$1"
  local cross="✖"
  if ! test_unicode; then cross="x"; fi
  if is_interactive; then
    printf "  %s%s%s %s%s%s\n" "$COLOR_RED" "$cross" "$RESET" "$COLOR_RED" "$msg" "$RESET"
  else
    echo "[-] Error: ${msg}"
  fi
}

# Download helper using curl or wget (headless fallback)
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

# Download helper with animated scanning comet progress bar
download_file_animated() {
  local url="$1"
  local dest="$2"
  local title="${3:-Downloading release binary}"

  if ! is_interactive; then
    download_file "$url" "$dest"
    return $?
  fi

  local log_file
  log_file="$(mktemp "${TMPDIR:-/tmp}/exp_dl_XXXXXX.log")"

  # Run download in background
  (
    if command -v curl >/dev/null 2>&1; then
      curl -fsSL "$url" -o "$dest" 2>"$log_file"
    elif command -v wget >/dev/null 2>&1; then
      wget -qO "$dest" "$url" 2>"$log_file"
    else
      exit 1
    fi
  ) &
  local dl_pid=$!

  local step=0
  local frames=("⠋" "⠙" "⠹" "⠸" "⠼" "⠴" "⠦" "⠧" "⠇" "⠏")
  local frame_count=${#frames[@]}
  local width=24
  local trail=6

  while kill -0 "$dl_pid" 2>/dev/null; do
    local frame="${frames[$((step % frame_count))]}"
    local head=$((step % (width + trail)))
    local bar=""
    for (( i=0; i<width; i++ )); do
      local age=$((head - i))
      if [ "$age" -ge 0 ] && [ "$age" -lt "$trail" ]; then
        case "$age" in
          0|1) cell="${COLOR_WHITE}#${RESET}" ;;
          2|3) cell="${COLOR_CYAN}#${RESET}" ;;
          4)   cell="${COLOR_TURQ}#${RESET}" ;;
          *)   cell="${COLOR_INDIGO}#${RESET}" ;;
        esac
      else
        cell="${DIM}-${RESET}"
      fi
      bar="${bar}${cell}"
    done

    printf "\r  %s%s%s %s %s%s%s " \
      "$COLOR_CYAN" "$frame" "$RESET" "$bar" "$BOLD" "$title" "$RESET"
    step=$((step + 1))
    sleep 0.05
  done

  printf "\r\033[K"
  wait "$dl_pid"
  local status=$?
  rm -f "$log_file"
  return "$status"
}

# ------------------------------------------------------------------------------
# Summary Card
# ------------------------------------------------------------------------------
print_finish_card() {
  local target_bin="$1"
  local exp_home="$2"
  local version="$3"

  local bin_len=${#target_bin}
  local card_width=84
  if [ "$((bin_len + 16))" -gt "$card_width" ]; then
    card_width=$((bin_len + 16))
  fi

  local ch_tl="╭" ch_tr="╮" ch_bl="╰" ch_br="╯" ch_h="─" ch_v="│" ch_ml="├" ch_mr="┤" ch_check="✔"
  if ! test_unicode; then
    ch_tl="+" ch_tr="+" ch_bl="+" ch_br="+" ch_h="-" ch_v="|" ch_ml="+" ch_mr="+" ch_check="+"
  fi

  local h_line=""
  for (( i=0; i<card_width-2; i++ )); do h_line="${h_line}${ch_h}"; done
  local top_line="${ch_tl}${h_line}${ch_tr}"
  local mid_line="${ch_ml}${h_line}${ch_mr}"
  local bot_line="${ch_bl}${h_line}${ch_br}"

  format_line() {
    local content="$1"
    local vis_len
    vis_len=$(get_visible_len "$content")
    local pad_len=$((card_width - 4 - vis_len))
    local pad=""
    for (( j=0; j<pad_len; j++ )); do pad="${pad} "; done
    printf "  %s%s%s  %s%s%s%s%s\n" "$COLOR_CYAN" "$ch_v" "$RESET" "$content" "$pad" "$COLOR_CYAN" "$ch_v" "$RESET"
  }

  if is_interactive; then
    printf "\n"
    printf "  %s%s%s\n" "$COLOR_CYAN" "$top_line" "$RESET"
    format_line "${COLOR_GREEN}${ch_check}${RESET} ${BOLD}${COLOR_WHITE}Experienced LLMs v${version} is ready!${RESET}"
    format_line ""
    format_line "${COLOR_TURQ}Binary:${RESET}   ${COLOR_GRAY}${target_bin}${RESET}"
    format_line "${COLOR_TURQ}Storage:${RESET}  ${COLOR_GRAY}${exp_home}${RESET}"
    printf "  %s%s%s\n" "$COLOR_CYAN" "$mid_line" "$RESET"
    format_line "${BOLD}${COLOR_WHITE}Quick Commands:${RESET}"
    format_line "  ${COLOR_YELLOW}experienced-llms status${RESET}      ${COLOR_GRAY}Inspect active facts & schedule${RESET}"
    format_line "  ${COLOR_YELLOW}experienced-llms log <text>${RESET}  ${COLOR_GRAY}Record operational wisdom${RESET}"
    format_line "  ${COLOR_YELLOW}experienced-llms config${RESET}      ${COLOR_GRAY}Configure LLM CLI & options${RESET}"
    format_line "  ${COLOR_YELLOW}experienced-llms update${RESET}      ${COLOR_GRAY}In-place self-update to latest${RESET}"
    printf "  %s%s%s\n" "$COLOR_CYAN" "$bot_line" "$RESET"
    printf "\n"
  else
    echo ""
    echo "=========================================================="
    echo "  [✔] Experienced LLMs Successfully Installed & Active!"
    echo "=========================================================="
    echo "Binary:  ${target_bin}"
    echo "Storage: ${exp_home}"
    echo ""
    echo "Quick Commands:"
    echo "  experienced-llms log \"Always run npm in WSL\" --category user_preference"
    echo "  experienced-llms status"
    echo "  experienced-llms config"
    echo "  experienced-llms update"
    echo "=========================================================="
  fi
}

# ==============================================================================
# Installer Execution Entrypoint
# ==============================================================================
mkdir -p "${BIN_DIR}"
mkdir -p "${EXPERIENCED_HOME}"

show_experienced_logo_animation
print_installer_title

# 1. Detect OS and Architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "${OS}" in
  Linux*)
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

write_step_success "Detected platform" "${OS} ${ARCH}${LIBC_FLAVOR:+ ($LIBC_FLAVOR)}"

# 2. Check for local build or prebuilt artifact first (source repo installation)
INSTALLED_LOCAL=false
SCRIPT_DIR=""
if [ -n "${BASH_SOURCE[0]:-}" ] && [ -f "${BASH_SOURCE[0]:-}" ]; then
  SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fi

if [ -n "${SCRIPT_DIR}" ]; then
  if [ -f "${SCRIPT_DIR}/dist/${ASSET_NAME}" ]; then
    write_step_success "Installing from local dist directory" "${SCRIPT_DIR}/dist/${ASSET_NAME}"
    cp "${SCRIPT_DIR}/dist/${ASSET_NAME}" "${TARGET_BIN}"
    chmod +x "${TARGET_BIN}"
    INSTALLED_LOCAL=true
  elif [ -f "${SCRIPT_DIR}/target/release/experienced-llms" ]; then
    write_step_success "Installing from local Cargo release build" "${SCRIPT_DIR}/target/release/experienced-llms"
    cp "${SCRIPT_DIR}/target/release/experienced-llms" "${TARGET_BIN}"
    chmod +x "${TARGET_BIN}"
    INSTALLED_LOCAL=true
  elif [ -f "${SCRIPT_DIR}/Cargo.toml" ] && command -v cargo >/dev/null 2>&1; then
    write_step_success "Compiling standalone binary from local source via Cargo..."
    (cd "${SCRIPT_DIR}" && cargo build --release)
    cp "${SCRIPT_DIR}/target/release/experienced-llms" "${TARGET_BIN}"
    chmod +x "${TARGET_BIN}"
    INSTALLED_LOCAL=true
  fi
fi

# 3. If not installed from local artifacts, download prebuilt binary from latest GitHub release
if [ "${INSTALLED_LOCAL}" = false ]; then
  DOWNLOAD_URL="https://github.com/${REPO}/releases/${VERSION_TAG}/download/${ASSET_NAME}"
  DOWNLOAD_SUCCESS=false

  if download_file_animated "${DOWNLOAD_URL}" "${TARGET_BIN}" "Downloading ${ASSET_NAME}"; then
    DOWNLOAD_SUCCESS=true
  elif [ "${OS}" = "Linux" ] && [ "${LIBC_FLAVOR:-}" = "musl" ]; then
    FALLBACK_ASSET="experienced-llms-linux-x86_64-gnu"
    FALLBACK_URL="https://github.com/${REPO}/releases/${VERSION_TAG}/download/${FALLBACK_ASSET}"
    write_step_warning "Trying GNU libc fallback: ${FALLBACK_ASSET}"
    if download_file_animated "${FALLBACK_URL}" "${TARGET_BIN}" "Downloading GNU fallback"; then
      DOWNLOAD_SUCCESS=true
    fi
  fi

  if [ "${DOWNLOAD_SUCCESS}" = false ]; then
    echo ""
    echo "[!] Prebuilt binary not available for ${OS} (${ARCH})."
    if command -v cargo >/dev/null 2>&1; then
      write_step_success "Cargo detected! Building and installing directly from source via Cargo..."
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

write_step_success "Installed standalone native binary" "${TARGET_BIN}"

# 4. PATH Configuration check
if [[ ":$PATH:" != *":${BIN_DIR}:"* ]]; then
  export PATH="${BIN_DIR}:${PATH}"
  SHELL_PROFILE=""
  if [ -n "${ZSH_VERSION:-}" ] || [ -f "${HOME}/.zshrc" ]; then
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
      write_step_success "Added ~/.local/bin to PATH" "${SHELL_PROFILE}"
    fi
  fi
fi

# 5. Initialize SQLite storage and directories
if "${TARGET_BIN}" init --help 2>&1 | grep -q -- '--yes'; then
  "${TARGET_BIN}" init --yes >/dev/null 2>&1 || "${TARGET_BIN}" init --yes
else
  "${TARGET_BIN}" init >/dev/null 2>&1 || "${TARGET_BIN}" init
fi
write_step_success "SQLite memory engine initialized" "${EXPERIENCED_HOME}"

# 6. Extract installed version
INSTALLED_VER="1.0.0rc6"
VER_STR="$("${TARGET_BIN}" --version 2>&1 || true)"
if [[ "${VER_STR}" =~ experienced-llms[[:space:]]+([0-9a-zA-Z\.\-]+) ]]; then
  INSTALLED_VER="${BASH_REMATCH[1]}"
fi

# 7. Display Finish Card & Status
print_finish_card "${TARGET_BIN}" "${EXPERIENCED_HOME}" "${INSTALLED_VER}"
"${TARGET_BIN}" status
