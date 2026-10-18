---
name: experienced-llm
description: Autonomous memory interface to load past experience, log operational heuristics, and execute scheduled consolidation.
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

Experienced LLMs provides an autonomous, self-learning long-term memory layer for AI agents with a strict 2-file footprint:
1. `~/.experienced-llms/memory.db`: SQLite ledger of structured facts, sessions, migrations, and audit logs.
2. `~/.experienced-llms/EXPERIENCE.md`: Single, authoritative, consolidated operational rulebook.

---

## 1. Startup Bootstrap (Session Start)

Before taking any actions or proposing solutions:
1. Read the active master experience file: `~/.experienced-llms/EXPERIENCE.md`
2. Strictly obey all User Directives, Project Constraints, and Defensive Heuristics defined there.
3. If the file does not exist yet, run `experienced-llms render` or `experienced-llms init` to initialize the baseline state.

---

## 2. Daytime Operational Logging (Fast, Instant, Offline)

Proactively record actionable operational heuristics, defensive safeguards, hidden gotchas, or user directives whenever discovered during a session.

### When to Record:
- **User Directives & Preferences**: User specifies an explicit preference, environment constraint, or command rule (e.g. "ALWAYS run npm inside WSL").
- **Defensive Safeguards & Heuristics**: Formulating a preventative pattern (e.g. "Check .env in .gitignore before git commit").
- **Project-Specific Constraints**: Architecture patterns, ports, DB engines, or frameworks chosen for this workspace.
- **Anti-Patterns & Critical Gotchas**: Hidden bugs, platform-specific traps (e.g. Windows CRLF vs Linux LF), or tool quirks.
- **Mistake Corrections & Fixes**: A tool call fails or code fails to compile and you resolve the root cause.

### What NOT to Record:
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

## 3. Consolidation Engine (Pathway A & Pathway B)

Consolidation merges pending intraday facts (`is_consolidated = 0`) into active memory and updates `EXPERIENCE.md` with zero token waste (no comments, no timestamps).

### Pathway A: Tooling / Scheduler Driven (Nightly Daemon)
Runs automatically via cron or Task Scheduler, or manually:
```bash
# Using external CLI tool (e.g. agy, claude, ollama)
experienced-llms consolidate --llm "agy query"

# Or using configured API provider (gemini, claude, openai, ollama)
experienced-llms consolidate --provider gemini
```

### Pathway B: Agentic In-Session Consolidation
When tasked with nightly memory consolidation as an autonomous agent:
1. **Prepare Prompt**: Run `experienced-llms consolidate --prepare` to fetch the structured JSON prompt payload containing active and pending facts.
2. **Reason**: Synthesize, deduplicate, resolve contradictions, and format the updated markdown.
3. **Apply**: Apply your consolidation result:
```bash
experienced-llms consolidate --apply '<json_or_file_path>'
```

### Consolidation Configuration Options:
- `--max-words <int>`: Override word count limit for `EXPERIENCE.md` (default: 1,200 words, configurable via `config.json`).
- `--template <path>`: Supply a custom prompt template to override default consolidation logic.

### Manual Re-render:
Rebuild `EXPERIENCE.md` directly from active SQLite facts at any time:
```bash
experienced-llms render [--max-words <int>]
```

