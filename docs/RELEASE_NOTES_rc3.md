## Release 1.0.0rc3 - Pure SQLite & LLM-Only Memory Engine

Experienced LLMs is a self-learning heuristic ledger and autonomous cognitive consolidation engine for local AI coding agents.

### 🌟 Key Highlights & Architecture

- **Strict 2-File Containment & Zero Workspace Pollution**:
  - Contained entirely in `~/.experienced-llms/memory.db` and `~/.experienced-llms/EXPERIENCE.md`.
  - Zero project repository dotfiles, zero raw daily markdown logs, and zero workspace pollution.
- **Pure SQLite Architecture**:
  - Thread-safe, high-concurrency WAL mode (`PRAGMA journal_mode=WAL`).
  - Full-stack migration engine (`schema_migrations`) managing schema and config evolutions safely.
- **Autonomous Interday Consolidation Engine**:
  - **Pathway A (Tooling / Daemon)**: Automated nightly consolidation via cron or Windows Task Scheduler.
  - **Pathway B (Agentic In-Session Chat Loop)**: LLMs invoked in chat run `experienced-llms consolidate --prepare`, reason over active and pending facts in their own context window, and commit via `experienced-llms consolidate --apply`.
- **Zero Token Waste & Token Density**:
  - Automated strip filters remove HTML comments and timestamps.
  - Configurable word count ceiling (`max_experience_words: 1200`, `--max-words <int>`).
- **Customizable Consolidation Algorithm**:
  - External prompt template loader supporting global (`~/.experienced-llms/consolidation_prompt.txt`) and CLI flag (`--template <path>`) overrides.
- **Audited 12-Subcommand CLI Suite**:
  - Full suite covering `log`, `consolidate`, `render`, `show`, `status`, `setup`, `schedule`, `prompt`, `integrate`, `migrate`, `init`, and `--version`.
- **Standards Compliant**:
  - Fully compliant with the [agenticskills.io/methodology](https://agenticskills.io/methodology) standard for AI coding agents.

### 📦 Installation

**POSIX (macOS, Ubuntu, Arch Linux, Debian)**:
```bash
curl -fsSL https://github.com/developer88/experienced-llms/releases/download/1.0.0rc3/install.sh | bash
```

**Windows 11 (PowerShell)**:
```powershell
irm https://github.com/developer88/experienced-llms/releases/download/1.0.0rc3/install.ps1 | iex
```
