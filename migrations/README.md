# Full-Stack Migration System (Omakub / Rails Style)

Experienced LLMs uses an automated, sequential migration framework inspired by Ruby on Rails and DHH's Omakub (`bin/omakub-sub/upgrade`).

Rather than limiting migrations to database schemas, Experienced LLMs migrations are **full-stack**: a single migration script can apply coordinated updates across four distinct operational layers.

---

## The 4 Full-Stack Migration Layers

Each migration script has access to a structured context and can execute upgrades across:

1. **SQLite Database Layer**:
   - Table alterations, new columns, index creations, data normalization.
   - Executed via `context["conn"]` within an atomic transaction.
2. **Agent Skills Layer**:
   - Updates `SKILL.md` instructions across configured AI agent directories (`~/.gemini/config/skills/experienced-llms/`, `~/.claude/CLAUDE.md`, `.cursorrules`, etc.).
   - Ensures running agents immediately adopt new CLI flags, categories, or operating directives.
3. **CLI Scripts & Tooling Layer**:
   - Updates executable shims (`~/.local/bin/experienced-llms`, `experienced-llms.cmd`).
   - Updates cron jobs, systemd units, or Windows Task Scheduler task definitions.
4. **Configuration Layer**:
   - Injects new configuration keys and default values into `~/.experienced-llms/config.json`.
   - Strictly preserves existing user settings, custom paths, and API keys.

---

## File Naming & Ordering Convention

Migration files reside in this `migrations/` directory and must follow a sequential 3-digit prefix:

```
migrations/
├── README.md
├── 001_1_0_0rc3_baseline.py  (Baseline marker)
├── 002_add_max_words.py      (Example future migration)
└── 003_custom_templates.py   (Example future migration)
```

- Pattern: `^\d{3}_[\w\-]+\.(py|sql|sh|ps1)$`
- Migration order is strictly alphanumeric (`001`, `002`, `003`, etc.).
- Applied migrations are recorded in the `schema_migrations` SQLite table (`version` primary key).

---

## Baseline Specification (1.0.0rc3)

Version `1.0.0rc3` establishes the clean-slate baseline recorded as:
```
001_1_0_0rc3_baseline
```
On initial setup, `init_db()` registers this baseline automatically. Fresh installations record the baseline and execute zero legacy migration scripts.

---

## Migration Script Structure (Python Example)

```python
"""Migration 002: Add max_words setting to configuration and prompt template."""

def migrate(context):
    conn = context["conn"]
    config = context["config"]
    home_dir = context["home_dir"]

    # 1. Database updates (if applicable)
    # conn.execute("ALTER TABLE ...")

    # 2. Config updates
    cfg = config.load_config()
    if "max_experience_words" not in cfg:
        cfg["max_experience_words"] = 1200
        config.save_config(cfg)

    # 3. Agent Skill updates (if applicable)
    # Update local agent instruction files

    # 4. Tooling updates (if applicable)
    print("  [✓] Migration 002 applied successfully across full-stack layers.")
```
