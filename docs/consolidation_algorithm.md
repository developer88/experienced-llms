# Consolidation Algorithm & Prompt Customization Guide

`Experienced LLMs` uses an interday cognitive consolidation engine to distill atomic, intraday operational learnings into a high-density, authoritative `EXPERIENCE.md` document while maintaining an immutable SQLite ledger in `~/.experienced-llms/memory.db`.

---

## 1. The Consolidation Cycle

Consolidation runs either autonomously via a scheduled daemon (cron or Windows Task Scheduler) or on demand via the CLI:

```
       [ Intraday Capture ]
                 │
                 ▼
     [ memory.db (SQLite) ]
     - Active Facts
     - Pending Intraday Facts
                 │
                 ▼
      [ Consolidation Engine ]
  (Reads ~/.experienced-llms/template.md)
                 │
       ┌─────────┴─────────┐
       ▼                   ▼
[ SQLite Ledger ]     [ EXPERIENCE.md ]
- Superseded IDs      - Clean, token-dense
- Consolidated IDs      operational directives
- Audit log entry     - Zero timestamps/comments
```

---

## 2. The 4 Core Consolidation Rules

The consolidation prompt enforces four core heuristics to prevent prompt bloat and maintain factual integrity:

### 1. Semantic Deduplication
Intraday logs often capture similar lessons across multiple sessions or projects (e.g., repeated notes about test flags or package managers). The engine clusters semantically overlapping directives and condenses them into a single, sharp operational rule rather than appending redundant bullet points.

### 2. Contradiction Resolution
As projects evolve, newer directives may invalidate earlier decisions (e.g., switching from npm to pnpm, or revising a deployment flow). When the LLM detects a conflict, it preserves the newer directive in `updated_experience_markdown` and returns the superseded fact's ID in `superseded_fact_ids`. The engine automatically transitions those rows in SQLite from `active` to `superseded`.

### 3. Canonical Section Preservation
By default, the engine maintains 5 canonical sections in fixed order:
```markdown
# Master AI Experience
## 1. User Directives & Preferences
## 2. Defensive Safeguards & Heuristics
## 3. Project-Specific Constraints
## 4. Anti-Patterns & Critical Gotchas
## 5. Mistake Corrections & Fixes
```
Maintaining standard headers ensures that downstream agents and IDE prompt injectors can reliably extract specific subsections.

### 4. Maximum Word Count & Token Density
Agent context windows are finite and expensive.
- Output is constrained to `{max_words}` (default `1200` words, configurable via `~/.experienced-llms/config.json` or `--max-words <n>`).
- Strict token hygiene: zero HTML comments (`<!-- ... -->`), zero generation timestamps, and zero conversational fluff.

---

## 3. Consolidation JSON Output Schema

When the LLM runs consolidation, it produces a structured JSON response:

```json
{
  "updated_experience_markdown": "# Master AI Experience\n\n## 1. User Directives & Preferences\n...",
  "processed_pending_ids": [101, 102, 103],
  "superseded_fact_ids": [45, 78],
  "summary_of_changes": "Merged WSL npm execution rules, superseded legacy direct-node guidance."
}
```

### Schema Properties

| Field | Type | Description |
|---|---|---|
| `updated_experience_markdown` | `string` | The complete, standalone markdown content written directly to `~/.experienced-llms/EXPERIENCE.md`. |
| `processed_pending_ids` | `array[int]` | List of pending fact IDs that were integrated into the markdown. These are marked `is_consolidated = 1`. |
| `superseded_fact_ids` | `array[int]` | List of previously active fact IDs rendered obsolete by new learnings. These are marked `status = 'superseded'`. |
| `summary_of_changes` | `string` | 1–2 sentence human-readable summary recorded in the `consolidations` audit table. |

---

## 4. Customizing the Prompt Template

Users can customize or completely replace the consolidation instructions to suit specialized environments (e.g., enterprise compliance, game development, microservice architectures).

### Template Resolution Order & Fail-Hard Guarantee

1. **CLI Flag (`--template <path>`)**: Explicit path passed to `experienced-llms consolidate`.
2. **Default File on Disk (`~/.experienced-llms/template.md`)**: Created automatically during `experienced-llms init`.

> [!IMPORTANT]
> Experienced LLMs will **fail hard** with an exit error if `~/.experienced-llms/template.md` is missing and no `--template` path is provided. No hidden fallback prompt is used.

### Template Variables

Your custom template can include the following placeholder:
- `{max_words}`: Substituted dynamically with the active maximum word ceiling (e.g., `1200`).

### Example Custom Template

Create `~/custom_template.md`:

```markdown
You are the consolidation engine for our Mobile Development team.
Synthesize new learnings and update EXPERIENCE.md under {max_words} words.

SECTIONS:
# Master AI Experience
## 1. Flutter & Dart Best Practices
## 2. Platform Channels & Native Interop
## 3. CI/CD & Fastlane Rules
## 4. Performance & Memory Safeguards

RULES:
- Resolve any conflicting architectural patterns by favoring newer rules.
- Return older superseded rule IDs in 'superseded_fact_ids'.
- Return all integrated pending IDs in 'processed_pending_ids'.

YOU MUST RETURN JSON:
{
  "updated_experience_markdown": "# Master AI Experience\n...",
  "processed_pending_ids": [],
  "superseded_fact_ids": [],
  "summary_of_changes": "..."
}
```

### Running Consolidation with a Custom Template

```bash
# Using external CLI command
experienced-llms consolidate --template ~/custom_template.md --llm "ollama run mistral"

# Or with configured default command
experienced-llms consolidate --template ~/custom_template.md
```
