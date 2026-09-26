---
name: experienced-llm
description: Self-learning memory interface that continuously loads accumulated experience from past sessions and autonomously records operational wisdom, defensive habits, and cause-effect heuristics.
---

# Experienced LLM Agent Skill

This skill equips any LLM (Gemini, Claude, GPT, or local models) with an autonomous long-term memory and self-learning loop.

## 1. Session Initialization (Read Experience)

At the start of your interaction or project task, inspect your accumulated master experience:
- Read the file: `~/.experienced-llms/EXPERIENCE.md`
- Adopt all **User Directives & Preferences**, **Project-Specific Constraints**, and **Defensive Heuristics** outlined in that document.
- Never violate constraints listed in `EXPERIENCE.md`.

## 2. Proactive Experience Logging (Reason $\rightarrow$ Outcome)

Do **NOT** treat memory merely as a log of user scoldings or static facts (e.g. do NOT log trivia like "we use Next.js v16"—you can read `package.json` for that).

Instead, record **operational wisdom, defensive habits, and cause-and-effect discoveries**:

### What to Record (The "Reason $\rightarrow$ Outcome" Pattern):
1. **Defensive Safeguards & Best Practices**:
   - *Example*: "Always check that `.env` is listed in `.gitignore` before making any git commit" (Reason: Prevent accidental secret exposure even if the user didn't explicitly remind you).
2. **Hidden Traps & Root Causes**:
   - *Example*: "Always regenerate client code after changing schemas" (Reason: Stale generated types cause silent runtime mismatches).
3. **Environmental Gotchas**:
   - *Example*: "Always run node/npm inside WSL instead of Windows PowerShell" (Reason: Path resolution breaks across Windows/Linux filesystem boundary).
4. **User Working Directives**:
   - *Example*: "Do not write boilerplate comments; write terse, direct code" (Reason: User preferred coding style).

### When to Log:
- **Proactively during execution**: When you discover a tricky edge-case or formulate a defensive safeguard while solving a problem.
- **At milestone completion**: When a feature or refactor succeeds, reflect on what non-obvious step prevented failure.
- **When corrected**: When the user corrects your approach or enforces a preference.

---

### How to Record:

#### Method A: Direct CLI Command (Preferred if shell tool is available)
Run the local CLI logger (instant, offline, 0 LLM tokens):
```bash
experienced-llms log "<operational rule / habit>" --reason "<underlying root cause / reason>" --category "<category>" --scope "<scope>"
```
*Categories*:
- `defensive_heuristic`: Proactive safety checks, pre-commit guards, cleanup rules.
- `technical_decision`: Architectural choices, why approach A was picked over B.
- `project_gotcha`: Non-obvious traps, hidden dependencies, compiler quirks.
- `user_preference`: Directives on how the user prefers code or communication.

*Scope*: `global` or the current repository/project name.

#### Method B: Self-Reporting Tag (If running without shell access)
Emit a `<session_learning>` block at the end of your response:
```markdown
<session_learning>
- [category] (scope): Operational rule statement | Reason and root cause
</session_learning>
```

---

## 3. Autonomous Nightly Consolidation

All intraday entries are merged, deduplicated, and organized into `~/.experienced-llms/EXPERIENCE.md` automatically by a nightly background cron daemon. You do not need to clean up past memory notes during your conversation.
