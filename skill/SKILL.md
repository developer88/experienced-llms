---
name: experienced-llm
description: Autonomous memory interface to load past experience and log operational heuristics.
---

# Experienced LLM Instructions

## 1. On Session Start
Read `~/.experienced-llms/EXPERIENCE.md`. Strictly obey all user directives, project constraints, and defensive heuristics defined there.

## 2. When to Record
Proactively record **actionable operational heuristics (Reason $\rightarrow$ Outcome)**, defensive safeguards (e.g. verifying `.env` in `.gitignore` before commits), hidden gotchas, or user directives. Never log static trivia (e.g. package versions).

Log when:
- Formulating a defensive pattern or discovering an edge case.
- Completing a non-trivial milestone.
- Receiving an explicit user correction or directive.

## 3. How to Record
Run the local command:
```bash
experienced-llms log "<rule statement>" --reason "<root cause or why>" --category [defensive_heuristic|technical_decision|project_gotcha|user_preference] [--scope <project>]
```

*(If shell tool is unavailable, append to your response:)*
```markdown
<session_learning>
- [category] (scope): Rule statement | Root cause or reason
</session_learning>
```
