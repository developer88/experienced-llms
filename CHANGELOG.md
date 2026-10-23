# Changelog

All notable changes to the Experienced LLMs project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [1.0.0rc5] - 2026-10-23

### Added
- **Native In-Place Self-Update (`experienced-llms update`)**: Automatically check GitHub Releases, download platform-specific prebuilt binaries, and atomically replace the running executable in-place.
- Supports `--check` for non-mutating version queries and `--yes` (`-y`) for non-interactive automation.
- Automatically executes SQLite schema migrations and synchronizes agent assets after an update.
- **Embedded Static Assets**: `SKILL.md` and default consolidation `template.md` compiled directly into the binary via `include_str!`, eliminating external asset dependency.
- **Safe Asset Prompt & Backup Engine**: Prompts `[o]verwrite / [s]kip` on `init` and `update` when user customizations are present, automatically generating timestamped `.bak_<timestamp>` backups.
- **Autonomous Skill Self-Bootstrap**: `SKILL.md` Section 1 verifies binary availability in `PATH` and executes or guides the user with 1-line installers if missing.
- **Self-Healing Consolidation**: Automatically rebuilds missing `EXPERIENCE.md` from active SQLite facts when running `consolidate`.

### Changed
- Streamlined public CLI surface to 5 core commands: `log`, `consolidate`, `status`, `schedule`, and `update` (plus `init`).
- Internal routines (`render`, `migrate`, `integrate`, `extract`, `compileskills`, `showcontext`) hidden from public CLI help.

### Removed
- Deprecated subcommands: `setup` (edit `config.json` directly or use flags), `show` (inspect `EXPERIENCE.md` directly), and `prompt` (skills loaded natively by agents).
- Purged legacy workspace file injection logic modifying `CLAUDE.md`, `AGENTS.md`, `.cursorrules`, and `.github/copilot-instructions.md`.

---

## [1.0.0rc4] - 2026-10-18

### Added
- **Standalone Native Rust Engine**: Complete rewrite from Python into pure Rust with sub-5ms CLI startup and zero memory overhead.
- Pure-Rust TLS via `rustls` with `ring`, and bundled C SQLite in WAL mode via `rusqlite` — zero external runtime dependencies.
- **Zero-Friction 1-Line Terminal Installers**:
  - POSIX: `curl -fsSL https://github.com/developer88/experienced-llms/releases/latest/download/install.sh | bash`
  - Windows: `irm https://github.com/developer88/experienced-llms/releases/latest/download/install.ps1 | iex`
- **Prebuilt Multi-Platform Binaries**: Released binaries for `linux-musl`, `linux-gnu`, and `windows-x86_64.exe`.
- **Hermetic Docker Release Pipeline**: Local multi-target cross-compilation pipeline with 0 GitHub Actions cloud runner costs.
- **Legacy rc2 to SQLite Migration Utility**: `scripts/migrate_rc2_to_latest.py` for safe transition of legacy markdown logs into SQLite.

### Changed
- Strict 2-file containment: all data isolated to user home (`~/.experienced-llms/memory.db` and `~/.experienced-llms/EXPERIENCE.md`).

---

## [1.0.0rc3] - 2026-10-16

### Added
- Pure SQLite state storage replacing file-based markdown logs.
- Dedicated LLM consolidation engine with structured prompt templates.
- AgenticSkills methodology compliance and metadata specification.
