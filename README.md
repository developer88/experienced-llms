<p align="center">
  <img src="assets/logo.jpg" alt="Experienced LLMs Logo" width="520" />
</p>

# Experienced LLMs

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Version: 1.0.0rc5](https://img.shields.io/badge/Version-1.0.0rc5-green.svg)](https://github.com/developer88/experienced-llms/releases)
[![AgenticSkills: S-Rank](https://img.shields.io/badge/AgenticSkills-Compliant-orange.svg)](https://agenticskills.io/methodology)

Let's give AI a tool to learn from our feedback day to day. It does not replace `AGENTS.md`, nor Skills or MCPs, but significantly improves your local agents and makes them tailored to your personal workflows.

---

## 🔒 2-Component Architecture & Privacy

Experienced LLMs operates with a strict privacy guarantee and a clean 2-component footprint:

1. **Local Skill & Storage Engine (`~/.experienced-llms/`)**:
   - `memory.db`: Local SQLite database (WAL mode) tracking all operational heuristics, sessions, and consolidation history.
   - `EXPERIENCE.md`: Single, authoritative, consolidated operational rulebook rendered for agents.
   - `template.md`: Default consolidation instructions and taxonomy required on disk.
   - Zero repository scanning: Never indexes, inspects, or uploads workspace code or git history.
   - Zero workspace pollution: No hidden dotfiles or config directories in project folders.

2. **Injected Agent Instructions**:
   - Lightweight skill instructions injected into your LLM agents (Antigravity, Claude, Copilot, Cursor, Pi).
   - Directs the agent to consult `EXPERIENCE.md` on startup and log corrections or workflow preferences during development.

---

## 🏗️ How It Works

1. **Daytime Operational Logging**: During normal work, you or your agent log lessons and constraints using `experienced-llms log "<directive>" --reason "<why>"`. It executes locally in milliseconds into SQLite with zero token waste.
2. **Consolidation (LLM)**: On a nightly schedule (or on demand), `experienced-llms consolidate` reads pending facts, synthesizes and deduplicates them using `~/.experienced-llms/template.md`, and supersedes outdated rules.
3. **Continuous Execution**: The resulting high-density `EXPERIENCE.md` is immediately available to all your local AI agents on their next prompt.

For detailed architecture diagrams and design specifications, see [System Architecture](docs/ARCHITECTURE.md).

---

## ⚡ Quick Start: 1-Line Terminal Installation

No compiler, Python, or multi-step dependency setup required. Copy and paste a single command into your terminal:

### POSIX (Linux, macOS, Ubuntu, Arch, Debian, WSL)
```bash
curl -fsSL https://github.com/developer88/experienced-llms/releases/latest/download/install.sh | bash
```

### Windows 11 (PowerShell)
```powershell
irm https://github.com/developer88/experienced-llms/releases/latest/download/install.ps1 | iex
```

---

### Source & Local Installation

If you prefer building from source:

```bash
# Clone the repository
git clone https://github.com/developer88/experienced-llms.git
cd experienced-llms

# Build and install standalone binary via Cargo
cargo install --path .

# Or run the local installer script (auto-detects local repository & builds if needed)
bash install.sh        # On Linux / macOS / WSL
.\install.ps1          # On Windows PowerShell
```

---

## 🚀 Usage Reference

### 1. Log an Operational Rule or Preference
```bash
experienced-llms log "Always run npm inside WSL, never directly on Windows" \
  --category user_preference \
  --reason "Avoid Windows path resolution issues"
```

### 2. Inspect Active Rulebook
```bash
experienced-llms show
```

### 3. Run Consolidation
```bash
# Using external CLI tool (e.g. agy, claude, ollama)
experienced-llms consolidate --llm "agy query"

# Or configure default LLM CLI command once:
experienced-llms setup --llm "agy query"
experienced-llms consolidate
```

### 4. Re-render EXPERIENCE.md from SQLite Facts
```bash
experienced-llms render
```

---

## 🧠 Consolidation Algorithm & Customization

The cognitive consolidation engine distills daytime learnings into high-density operational guidance while preserving factual integrity via an immutable SQLite ledger.

### Core Consolidation Rules
1. **Semantic Deduplication**: Merges overlapping heuristics into sharp, concise directives.
2. **Contradiction Resolution**: Detects obsolete rules and marks them `superseded` in SQLite.
3. **Canonical Categorization**: Preserves standard operational sections and taxonomy across consolidation cycles.
4. **Token Density & Word Ceiling**: Restricts output length (`--max-words`, default 1200 words) with zero HTML comments or timestamps.

### Prompt Template
Consolidation instructions and section taxonomies are governed by `~/.experienced-llms/template.md` (created during `experienced-llms init`). The engine requires this file to exist on disk and will fail hard if it is missing.

For detailed architecture, JSON schemas, and template customization, see [Consolidation Algorithm Guide](docs/consolidation_algorithm.md).

---

## 🛠️ CLI Command Reference

| Command | Arguments / Flags | Description |
|---|---|---|
| `experienced-llms log "<rule>"` | `--category`, `--scope`, `--reason`, `--session-id` | Log an operational heuristic (fast, local SQLite capture) |
| `experienced-llms consolidate` | `--date`, `--llm`, `--max-words` | Consolidate pending facts and self-heal `EXPERIENCE.md` |
| `experienced-llms status` | *(None)* | Show active facts, ledger counts, and schedule state |
| `experienced-llms schedule` | `--install`, `--uninstall`, `--status`, `--time`, `--type` | Manage scheduled nightly consolidation daemon |
| `experienced-llms update` | `--check`, `--yes`, `--tag` | Perform automated in-place self-update to latest release |
| `experienced-llms init` | `--db-path`, `--yes` | Initialize SQLite schema, directories, template, and initial EXPERIENCE.md |
| `experienced-llms --version` | `-v` | Display version information (`1.0.0rc5`) |

---

## 🧩 AgenticSkills Standards Compliance

Experienced LLMs is packaged in full accordance with the [agenticskills.io/methodology](https://agenticskills.io/methodology) standard:
- **Canonical Skill Location**: `skills/experienced-llm/SKILL.md`
- **Spec-Compliant Frontmatter**: Name, description (< 200 chars), version (`1.0.0rc5`), author, repository, license, and keywords.
- **Safety & Containment**: Strict local containment in user home, zero capability escalation, zero private data leakage.

---

## 📄 License

MIT © [Andrey Eremin (developer88)](https://github.com/developer88)
