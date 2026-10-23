## Release 1.0.0rc5 - Self-Update, Streamlined CLI & Autonomous Skill Bootstrap

Experienced LLMs provides an autonomous, self-learning cognitive heuristic ledger and memory consolidation engine for AI coding agents.

### 🌟 What's New in 1.0.0rc5

- **Native In-Place Self-Update (`experienced-llms update`)**:
  - Upgrading to the latest release no longer requires copy-pasting installation URLs.
  - Run `experienced-llms update` to automatically check GitHub Releases, download the platform-specific prebuilt binary, and atomically self-replace the executable in-place.
  - Supports `--check` flag to inspect available updates and `--yes` / `-y` for non-interactive environments.
  - Automatically runs post-update database schema migrations and asset synchronization.
- **100% Self-Contained Binary (Embedded Assets)**:
  - Both [`SKILL.md`](skills/experienced-llm/SKILL.md) and the default consolidation [`template.md`](src/config.rs) are compiled directly into the Rust binary via `include_str!`.
  - Zero runtime dependencies on cloned repositories or external asset downloads.
- **Safe Asset Prompt & Automated Backup Engine**:
  - On `init` and `update`, if existing user customizations are detected in `template.md` or `SKILL.md`, the engine prompts interactively (`[o]verwrite / [s]kip`).
  - When overwritten, the engine automatically creates timestamped backups (e.g. `template.md.bak_20261023_080000`) before modifying the file.
- **Zero Workspace Pollution Guarantee**:
  - Completely purged legacy injection logic modifying `CLAUDE.md`, `AGENTS.md`, `.cursorrules`, and `.github/copilot-instructions.md`.
  - Experienced LLMs is strictly contained to:
    1. `~/.experienced-llms/` (SQLite database, compiled rulebook, custom template)
    2. `~/.gemini/config/skills/experienced-llms/SKILL.md` (canonical agent skill)
- **Autonomous Skill Self-Bootstrap**:
  - `SKILL.md` Startup Bootstrap now includes automated binary detection (`experienced-llms --version`).
  - If installed via AgenticSkills or external registries without the binary, agents are guided to self-bootstrap or prompt the user with the official 1-line installer.
- **Radically Streamlined 5-Command CLI Surface**:
  - Dropped obsolete prototype commands: `setup` (edit `config.json` directly or pass flags), `show` (inspect `EXPERIENCE.md` directly), and `prompt` (agents load skill natively).
  - Internal routines (`render`, `migrate`, `integrate`) hidden from public CLI help.
  - Clean 5-command core: `log`, `consolidate`, `status`, `schedule`, and `update` (plus `init`).
- **Self-Healing Consolidation**:
  - If `EXPERIENCE.md` is deleted or missing, running `experienced-llms consolidate` automatically rebuilds and self-heals the master experience document directly from active SQLite facts.

---

### 📦 Quick 1-Line Installation

**POSIX (Linux, macOS, Ubuntu, Arch, Debian, WSL)**:
```bash
curl -fsSL https://github.com/developer88/experienced-llms/releases/latest/download/install.sh | bash
```

**Windows 11 (PowerShell)**:
```powershell
irm https://github.com/developer88/experienced-llms/releases/latest/download/install.ps1 | iex
```

### 🔄 Upgrading Existing Installations

If you already have `experienced-llms` installed:
```bash
experienced-llms update
```
*(or run the 1-line installer script above)*
