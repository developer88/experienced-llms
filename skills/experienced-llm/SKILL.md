---
name: experienced-llm
description: Autonomous operational experience interface to load past heuristics, log runtime lessons, and execute scheduled consolidation.
version: 1.0.0rc4
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
1. Read the active master experience file: `~/.experienced-llms/EXPERIENCE.md`
2. Strictly obey all User Directives, Project Constraints, and Defensive Heuristics defined there.
3. If the file does not exist yet, run `experienced-llms init` to initialize the baseline state and render EXPERIENCE.md.

---

## 2. Daytime Operational Logging (Fast, Instant, Offline)

Proactively record actionable operational heuristics, defensive safeguards, hidden gotchas, or user directives whenever discovered during a session.

### When to Record:
- **User Directives & Preferences**: User specifies an explicit preference, environment constraint, or command rule (e.g. "ALWAYS run npm inside WSL").
- **Defensive Safeguards & Heuristics**: Formulating a preventative pattern (e.g. "Check .env in .gitignore before git commit").
- **Anti-Patterns & Critical Gotchas**: Hidden bugs, platform-specific traps (e.g. Windows CRLF vs Linux LF), or tool quirks.
- **Mistake Corrections & Fixes**: A tool call fails or code fails to compile and you resolve the root cause.

### What NOT to Record:
- Never log project-specific constraints (e.g. workspace ports, DB engines, frameworks, or architecture patterns) — these belong in dedicated project skills or repository guidelines, not general operational heuristics.
- Never log static trivia, language syntax readily available online, or temporary session state.
- Never log conversational chatter or non-operational commentary.

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

Consolidation merges pending intraday facts (`is_consolidated = 0`) into active operational heuristics and updates `EXPERIENCE.md` with zero token waste (no comments, no timestamps).

### Running Consolidation:
Runs automatically via cron or Windows Task Scheduler, or on demand via terminal:
```bash
# Using external CLI tool (e.g. agy, claude, ollama)
experienced-llms consolidate --llm "agy query"

# Or configure default LLM command once:
experienced-llms setup --llm "agy query"
experienced-llms consolidate
```

### Consolidation Configuration Options:
- `--max-words <int>`: Override word count limit for `EXPERIENCE.md` (default: 1,200 words, configurable via `config.json`).
- `--template <path>`: Override the default prompt template located at `~/.experienced-llms/template.md`.

### Manual Re-render:
Rebuild `EXPERIENCE.md` directly from active SQLite facts at any time:
```bash
experienced-llms render [--max-words <int>]
```
