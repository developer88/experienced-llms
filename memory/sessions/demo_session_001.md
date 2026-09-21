# Session Memory: demo_session_001
Extracted at: 2026-09-21T20:57:50.180341

## Extracted Key Points
- **[USER_PREFERENCE]** (global): ALWAYS execute npm and node commands inside WSL. Never call node directly from Windows host.
  *Reason/Context*: User environment constraint and strict rule
- **[TECHNICAL_DECISION]** (experienced-llms): Use SQLite with PRAGMA foreign_keys = ON instead of PostgreSQL for state management.
  *Reason/Context*: Optimized for single 8GB GPU local setups with zero server overhead
- **[MISTAKE_CORRECTION]** (experienced-llms): Omit ArtifactMetadata when calling write_to_file for normal workspace code files.
  *Reason/Context*: Providing ArtifactMetadata triggers invalid_args artifact path enforcement

---
## Original Transcript Snippet
```text
User: In this project, ALWAYS execute npm commands inside WSL. Never call node or npm directly from Windows.
Assistant: Understood. I will run npm and node scripts exclusively inside WSL via bash.
User: Also, we decided to use SQLite with strict foreign keys enabled rather than PostgreSQL for the local memory database because the user has 8GB VRAM and wants zero network overhead.
Assistant: Got it, configured SQLite with PRAGMA foreign_keys = ON in data/memory.db.
User: Note that when calling write_to_file for normal workspace code, do NOT include ArtifactMetadata, otherwise the tool errors out thinking it is an artifact.
Assistant: Fixed, ArtifactMetadata will only be supplied when creating files in the artifacts directory.

```
