## Release 1.0.0rc4 - Native Rust Standalone Engine & Zero-Friction 1-Line Installers

Experienced LLMs is a self-learning cognitive heuristic ledger and autonomous memory consolidation engine for AI coding agents.

### 🌟 What's New in 1.0.0rc4

- **Standalone Native Rust Engine**:
  - Full rewrite in Rust for sub-5ms startup latency and zero memory overhead.
  - Zero Python, zero Node, and zero compiler requirements for end users.
  - Pure-Rust cryptography & TLS (`rustls` via `ring`) — zero external OpenSSL or MSVC runtime dependencies.
  - Bundled C SQLite in WAL mode (`rusqlite`) — zero system `libsqlite3` dependencies.
- **Zero-Friction 1-Line Terminal Installers**:
  - **POSIX**: `curl -fsSL https://github.com/developer88/experienced-llms/releases/latest/download/install.sh | bash`
  - **Windows 11**: `irm https://github.com/developer88/experienced-llms/releases/latest/download/install.ps1 | iex`
  - Fully automated platform detection, prebuilt binary download, persistent PATH setup, SQLite baseline init, agent skills integration, and nightly schedule registration.
- **Cross-Platform Prebuilt Release Binaries**:
  - `experienced-llms-linux-x86_64-musl` (Static Linux binary, runs anywhere including Arch Linux and Alpine).
  - `experienced-llms-linux-x86_64-gnu` (Standard Linux GNU libc binary).
  - `experienced-llms-windows-x86_64.exe` (Standalone Windows 11 executable).
- **Hermetic Local Release Pipeline (Zero Cloud CI Costs)**:
  - Dockerized test runner and multi-target cross-compilation pipeline (`scripts/pipeline.sh`).
  - 100% local compilation and verification — 0 GitHub Actions runner minutes consumed.
- **Legacy rc2 to SQLite Migration Tool**:
  - Standalone migration utility (`scripts/migrate_rc2_to_latest.py`) to transition legacy markdown-based data into SQLite with automated backups and idempotency.
- **Strict 2-File Containment & Privacy**:
  - Everything lives in `~/.experienced-llms/memory.db` and `~/.experienced-llms/EXPERIENCE.md`.
  - Zero repository pollution, zero hidden dotfiles in workspaces, and zero telemetry.

### 📦 Quick 1-Line Installation

**POSIX (Linux, macOS, Ubuntu, Arch, Debian, WSL)**:
```bash
curl -fsSL https://github.com/developer88/experienced-llms/releases/latest/download/install.sh | bash
```

**Windows 11 (PowerShell)**:
```powershell
irm https://github.com/developer88/experienced-llms/releases/latest/download/install.ps1 | iex
```
