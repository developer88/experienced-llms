---
name: experienced-llm
description: Autonomous operational experience interface to load past heuristics, log runtime lessons, and execute scheduled consolidation.
version: 1.0.0rc5
author: developer88
repository: https://github.com/developer88/experienced-llms
license: MIT
keywords:
  - memory
  - persistent-memory
  - agentic-memory
  - self-learning
---

# Experienced LLM Operating Protocol

Experienced LLMs provides an autonomous, self-educating operational experience engine for AI agents with a clean 2-component architecture:
1. **Engine & Local State** (`~/.experienced-llms/`): SQLite ledger of structured facts, consolidation audits, custom templates, and the compiled `EXPERIENCE.md` rulebook.
2. **Injected Agent Instructions**: Protocol to read `EXPERIENCE.md` at session start and record runtime lessons with `experienced-llms log`.

---

## 1. Startup Bootstrap (Session Start)

Before taking any actions or proposing solutions:
1. **Engine Verification**: Check if `experienced-llms` is available in `PATH` (e.g. run `experienced-llms --version`).
   - If not installed, execute or prompt the user with the official 1-line installer:
     - **Linux / macOS**: `curl -fsSL https://github.com/developer88/experienced-llms/releases/latest/download/install.sh | bash`
     - **Windows**: `irm https://github.com/developer88/experienced-llms/releases/latest/download/install.ps1 | iex`
2. Read the active master experience file: `~/.experienced-llms/EXPERIENCE.md`.
3. Strictly obey all directives, safeguards, and preferences defined there.
4. If `EXPERIENCE.md` does not exist yet, run `experienced-llms init` to initialize the baseline state and compile `EXPERIENCE.md`.

---

## 2. Daytime Operational Logging (Fast, Instant)

Proactively record actionable operational heuristics, defensive safeguards, hidden gotchas, or user directives whenever discovered during a session.

### When to Record:
- **User Directives & Preferences**: User specifies an explicit preference, environment constraint, or command rule (e.g. "ALWAYS run npm inside WSL").
- **Defensive Safeguards & Heuristics**: Formulating a preventative pattern (e.g. "Check .env in .gitignore before git commit").
- **Anti-Patterns & Critical Gotchas**: Hidden bugs, platform-specific traps (e.g. Windows CRLF vs Linux LF), or tool quirks.
- **Mistake Corrections & Fixes**: A tool call fails or code fails to compile and you resolve the root cause.

### What NOT to Record (Strict Anti-Patterns):
- **Ephemeral Test/Verification Failures**: NEVER log raw failure notifications (e.g. `"Verification failure on task_3 (/tmp/...)"`, `"Test failed at line 12"`, `"SyntaxError: unexpected token"`). Only log once you have diagnosed the root cause and synthesized a **durable, actionable heuristic**. If no generalizable rule exists, do NOT log anything.
- **Temporary / Disposable File Paths**: NEVER log rules referencing `/tmp/`, `temp/`, scratch folders, or transient test sandboxes.
- **Task & Milestone Execution State**: NEVER log task IDs (`task_3`), milestone identifiers (`ms_9bd96fe9`), run hashes, or ephemeral debugging steps.
- **Raw Unanalyzed Errors & Stack Traces**: NEVER dump raw compiler logs or stack traces as statements. Heuristics must be clear, imperative operational rules (e.g. "Always run npm in WSL", never "npm exited with code 1").
- **Project-Specific Constraints & Framework Trivia**: Port numbers, database choices, framework conventions, or business logic belong in repository guidelines or dedicated project skills, not user-level operational memory.
- **Static Trivia & Public Documentation**: Standard programming language syntax or public library docs readily available online.
- **Conversational Chatter**: Speculative musings, session greetings, or non-operational commentary.

### Operational Logging Syntax:
```bash
experienced-llms log "<concise operational rule>" \
  --reason "<root cause or rationale>" \
  --category [defensive_heuristic | user_preference | technical_decision | mistake_correction | project_gotcha] \
  [--scope <workspace_or_project>] \
  [--session-id <session_identifier>]
```

---

## 3. Consolidation Engine

Consolidation merges pending intraday facts (`is_consolidated = 0`) into active operational heuristics and updates `EXPERIENCE.md` with zero token waste (no comments, no timestamps). If `EXPERIENCE.md` is ever missing, running `consolidate` automatically regenerates it.

### Running Consolidation:
Runs automatically via cron or Windows Task Scheduler, or on demand via terminal:
```bash
# Run consolidation with external CLI tool (e.g. agy, claude, ollama)
experienced-llms consolidate --llm "agy query"
```

### Consolidation Options:
- `--max-words <int>`: Override word count limit for `EXPERIENCE.md` (default: 1,200 words, configurable in `~/.experienced-llms/config.json`).
- `--date <YYYY-MM-DD>`: Target a specific session date for historical consolidation.
