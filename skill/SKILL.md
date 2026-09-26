---
name: experienced-llm
description: Self-learning memory interface that continuously loads accumulated experience from past sessions and records new directives, corrections, and project quirks.
---

# Experienced LLM Agent Skill

This skill equips any LLM (Gemini, Claude, GPT, or local models) with an autonomous long-term memory and self-learning loop.

## 1. Session Initialization (Read Experience)

At the start of your interaction or project task, inspect your accumulated master experience:
- Read the file: `~/.experienced-llms/EXPERIENCE.md`
- Adopt all **User Directives & Preferences**, **Project-Specific Constraints**, and **Anti-Patterns** outlined in that document.
- Never violate constraints listed in `EXPERIENCE.md`.

## 2. Intraday Learning (Zero-Cost Logging)

Whenever any of the following events occur during your session:
1. **User Preference**: The user specifies a workflow, formatting, or tooling preference (e.g., *"Always run npm inside WSL"*).
2. **Mistake Correction**: You attempted an approach that failed or was corrected by the user/compiler.
3. **Project Gotcha**: You discovered a specific quirk of the repository, environment, or dependencies.

**Immediately record the lesson** using one of two methods:

### Method A: Direct Command (Preferred if shell tool is available)
Run the local CLI logger (instant, offline, 0 LLM tokens):
```bash
experienced-llms log --category "<category>" --scope "<scope>" "<rule statement>" --reason "<why>"
```
*Categories*: `user_preference`, `technical_decision`, `mistake_correction`, `project_gotcha`.
*Scope*: `global` or the current repository/project name.

### Method B: Self-Reporting Tag (If running without shell access)
Emit a `<session_learning>` block at the end of your response:
```markdown
<session_learning>
- [category] (scope): Operational rule statement | Context or reason
</session_learning>
```

## 3. Background Nightly Consolidation

You do not need to manually compress or clean up memory files during conversations. An autonomous nightly cron daemon consolidates multi-day experiences, resolves contradictions, and keeps `EXPERIENCE.md` sharp and authoritative.
