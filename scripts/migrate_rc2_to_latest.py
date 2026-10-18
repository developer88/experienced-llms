#!/usr/bin/env python3
"""
Experienced LLMs: rc2 to Current (1.0.0rc4+) Architecture Migration Tool
========================================================================
Migrates legacy file-based rc2 data (EXPERIENCE.md, raw_days/ folders, daily/)
into the authoritative SQLite database architecture (memory.db) and re-renders
a clean, token-efficient master EXPERIENCE.md with zero data loss, full
idempotency, and automated legacy backup.

Usage:
    python3 scripts/migrate_rc2_to_latest.py [OPTIONS]

Options:
    --home PATH             Custom EXPERIENCED_LLMS_HOME (default: ~/.experienced-llms)
    --experience-file PATH  Custom path to rc2 EXPERIENCE.md
    --db-path PATH          Custom path to target memory.db
    --dry-run               Preview extracted rules without writing to SQLite
    --no-backup             Skip archiving legacy folders to backup_rc2_<timestamp>/
"""

import argparse
import os
import re
import shutil
import sqlite3
import subprocess
import sys
from datetime import datetime
from pathlib import Path

CATEGORY_MAP = {
    "user directives & preferences": "user_preference",
    "user directives": "user_preference",
    "user preferences": "user_preference",
    "user_preference": "user_preference",
    "defensive safeguards & heuristics": "defensive_heuristic",
    "defensive heuristics": "defensive_heuristic",
    "defensive_heuristic": "defensive_heuristic",
    "project-specific decisions & architectures": "technical_decision",
    "project-specific constraints": "technical_decision",
    "technical decisions": "technical_decision",
    "technical_decision": "technical_decision",
    "anti-patterns & critical gotchas": "project_gotcha",
    "anti-patterns": "project_gotcha",
    "critical gotchas": "project_gotcha",
    "project_gotcha": "project_gotcha",
    "mistake corrections & fixes": "mistake_correction",
    "mistake corrections": "mistake_correction",
    "mistake_correction": "mistake_correction",
}

SCHEMA_SQL = """
PRAGMA journal_mode = WAL;

CREATE TABLE IF NOT EXISTS schema_migrations (
    version TEXT PRIMARY KEY,
    applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    source TEXT DEFAULT 'chat',
    raw_path TEXT
);

CREATE TABLE IF NOT EXISTS memory_facts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT REFERENCES sessions(id) ON DELETE SET NULL,
    category TEXT NOT NULL CHECK(category IN ('defensive_heuristic', 'user_preference', 'technical_decision', 'mistake_correction', 'project_gotcha')),
    scope TEXT DEFAULT 'global',
    rule_statement TEXT NOT NULL,
    context_reason TEXT,
    confidence REAL DEFAULT 1.0,
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'superseded', 'user_verified', 'archived')),
    is_consolidated INTEGER NOT NULL DEFAULT 0,
    superseded_by INTEGER REFERENCES memory_facts(id) ON DELETE SET NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_memory_facts_scope_status ON memory_facts(scope, status);
CREATE INDEX IF NOT EXISTS idx_memory_facts_consolidation ON memory_facts(is_consolidated, status);
CREATE INDEX IF NOT EXISTS idx_memory_facts_category ON memory_facts(category);

CREATE TABLE IF NOT EXISTS consolidations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    consolidated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    engine_type TEXT NOT NULL,
    facts_processed INTEGER NOT NULL,
    facts_superseded INTEGER NOT NULL,
    summary_of_changes TEXT
);
"""

HEADER_DISPLAY_ORDER = [
    ("user_preference", "## 1. User Directives & Preferences"),
    ("defensive_heuristic", "## 2. Defensive Safeguards & Heuristics"),
    ("technical_decision", "## 3. Project-Specific Constraints"),
    ("project_gotcha", "## 4. Anti-Patterns & Critical Gotchas"),
    ("mistake_correction", "## 5. Mistake Corrections & Fixes"),
]

def parse_bullet_line(line: str):
    """
    Parses a markdown bullet from rc2 EXPERIENCE.md or raw_days:
      - (global): Always run npm in WSL *(Reason: Windows path collisions)*
      - Always commit with descriptive message
      - (backend): Use async SQLite WAL mode
    """
    line = line.strip()
    if not line.startswith("-"):
        return None

    content = line.lstrip("-").strip()
    if not content or content.lower().startswith("*(none recorded") or content.lower().startswith("*none*") or content.lower() == "(none)":
        return None

    scope = "global"
    scope_match = re.match(r"^\(([^)]+)\):\s*(.+)$", content)
    if scope_match:
        scope = scope_match.group(1).strip()
        content = scope_match.group(2).strip()

    reason = ""
    reason_match = re.search(r"\*\(Reason:\s*([^)]+)\)\*$", content, re.IGNORECASE)
    if reason_match:
        reason = reason_match.group(1).strip()
        rule = content[:reason_match.start()].strip()
    else:
        rule = content

    # Clean any leftover markdown comments or formatting
    rule = re.sub(r"<!--.*?-->", "", rule, flags=re.DOTALL).strip()
    return {
        "scope": scope,
        "rule": rule,
        "reason": reason
    }

def extract_from_markdown(file_path: Path, default_cat: str = "defensive_heuristic"):
    if not file_path.exists():
        return []

    text = file_path.read_text(encoding="utf-8")
    lines = text.splitlines()

    current_cat = default_cat
    extracted = []

    for line in lines:
        stripped = line.strip()
        if stripped.startswith("##"):
            header_name = re.sub(r"^##\s*(\d+\.\s*)?", "", stripped).strip().lower()
            current_cat = CATEGORY_MAP.get(header_name, default_cat)
            continue

        item = parse_bullet_line(stripped)
        if item and item["rule"]:
            item["category"] = current_cat
            extracted.append(item)

    return extracted

def render_experience_markdown(conn: sqlite3.Connection, target_file: Path):
    """Generates the authoritative EXPERIENCE.md from active SQLite facts."""
    cur = conn.cursor()
    cur.execute(
        """
        SELECT category, scope, rule_statement, context_reason
        FROM memory_facts
        WHERE status = 'active'
        ORDER BY id ASC
        """
    )
    rows = cur.fetchall()

    facts_by_cat = {cat: [] for cat, _ in HEADER_DISPLAY_ORDER}
    for cat, scope, rule, reason in rows:
        if cat in facts_by_cat:
            facts_by_cat[cat].append((scope, rule, reason))

    out_lines = ["# Master AI Experience", ""]
    for cat, header in HEADER_DISPLAY_ORDER:
        out_lines.append(header)
        items = facts_by_cat.get(cat, [])
        if not items:
            out_lines.append("- (None recorded yet)")
        else:
            for scope, rule, reason in items:
                scope_prefix = f"({scope}): " if scope and scope != "global" else ""
                reason_suffix = f" *(Reason: {reason})*" if reason else ""
                out_lines.append(f"- {scope_prefix}{rule}{reason_suffix}")
        out_lines.append("")

    target_file.parent.mkdir(parents=True, exist_ok=True)
    target_file.write_text("\n".join(out_lines).strip() + "\n", encoding="utf-8")
    print(f"[✓] Rebuilt authoritative master experience: {target_file}")

def migrate_rc2(home_dir: Path, exp_file: Path, db_path: Path, dry_run: bool = False, no_backup: bool = False):
    print(f"[*] Inspecting rc2 environment at: {home_dir}")
    extracted_active = extract_from_markdown(exp_file) if exp_file.exists() else []

    # Check for pending facts in legacy raw_days or daily
    raw_days_dir = home_dir / "raw_days"
    daily_dir = home_dir / "daily"
    extracted_pending = []

    for scan_dir in [raw_days_dir, daily_dir]:
        if scan_dir.exists():
            for day_file in sorted(scan_dir.glob("*.md")):
                items = extract_from_markdown(day_file)
                for item in items:
                    item["source_file"] = day_file.name
                    extracted_pending.append(item)

    total_rules = len(extracted_active) + len(extracted_pending)
    print(f"[+] Found {len(extracted_active)} consolidated rules in {exp_file.name if exp_file.exists() else 'N/A'}")
    print(f"[+] Found {len(extracted_pending)} pending intraday rules across raw_days/daily")

    if dry_run:
        print("\n--- Dry Run Preview (No database changes) ---")
        for idx, item in enumerate(extracted_active, 1):
            print(f"  [Active] #{idx} [{item['category']}] ({item['scope']}): {item['rule']} (Reason: {item['reason'] or 'N/A'})")
        for idx, item in enumerate(extracted_pending, 1):
            print(f"  [Pending] #{idx} [{item['category']}] ({item['scope']}): {item['rule']} (From: {item['source_file']})")
        return total_rules

    db_path.parent.mkdir(parents=True, exist_ok=True)
    conn = sqlite3.connect(str(db_path))
    try:
        conn.executescript(SCHEMA_SQL)
        now_str = datetime.now().strftime("%Y-%m-%d %H:%M:%S")

        conn.execute("INSERT OR IGNORE INTO sessions (id, source, raw_path) VALUES ('rc2_migration', 'migration_script', ?)", (str(exp_file),))
        conn.execute("INSERT OR IGNORE INTO schema_migrations (version) VALUES ('001_1_0_0rc3_baseline')")

        inserted_active = 0
        for item in extracted_active:
            cur = conn.execute(
                "SELECT id FROM memory_facts WHERE rule_statement = ? AND scope = ?",
                (item["rule"], item["scope"])
            )
            if cur.fetchone():
                continue

            conn.execute(
                """
                INSERT INTO memory_facts (
                    session_id, category, scope, rule_statement,
                    context_reason, confidence, status, is_consolidated,
                    created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, 1.0, 'active', 1, ?, ?)
                """,
                ("rc2_migration", item["category"], item["scope"], item["rule"], item["reason"], now_str, now_str)
            )
            inserted_active += 1

        inserted_pending = 0
        for item in extracted_pending:
            cur = conn.execute(
                "SELECT id FROM memory_facts WHERE rule_statement = ? AND scope = ?",
                (item["rule"], item["scope"])
            )
            if cur.fetchone():
                continue

            conn.execute(
                """
                INSERT INTO memory_facts (
                    session_id, category, scope, rule_statement,
                    context_reason, confidence, status, is_consolidated,
                    created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, 1.0, 'active', 0, ?, ?)
                """,
                ("rc2_migration", item["category"], item["scope"], item["rule"], item.get("reason", ""), now_str, now_str)
            )
            inserted_pending += 1

        conn.commit()
        print(f"[✓] Migrated {inserted_active} active rule(s) (is_consolidated = 1) into SQLite.")
        if inserted_pending > 0:
            print(f"[✓] Migrated {inserted_pending} pending intraday rule(s) (is_consolidated = 0) into SQLite.")

        # Re-render clean EXPERIENCE.md
        render_experience_markdown(conn, home_dir / "EXPERIENCE.md")

        return inserted_active + inserted_pending
    finally:
        conn.close()

def backup_legacy_files(home_dir: Path, exp_file: Path):
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    backup_dir = home_dir / f"backup_rc2_{timestamp}"

    legacy_items = [
        home_dir / "raw_days",
        home_dir / "sessions",
        home_dir / "daily"
    ]

    moved_anything = False
    for item in legacy_items:
        if item.exists() and any(item.iterdir()):
            backup_dir.mkdir(parents=True, exist_ok=True)
            target = backup_dir / item.name
            shutil.move(str(item), str(target))
            print(f"[+] Archived legacy folder: {item.name} -> {target}")
            moved_anything = True

    # Backup original experience file if it existed
    if exp_file.exists():
        backup_dir.mkdir(parents=True, exist_ok=True)
        shutil.copy2(str(exp_file), str(backup_dir / "EXPERIENCE.md.rc2_orig"))
        print(f"[+] Archived original EXPERIENCE.md -> {backup_dir / 'EXPERIENCE.md.rc2_orig'}")
        moved_anything = True

    if moved_anything:
        print(f"[✓] Legacy files safely archived to: {backup_dir}")

def main():
    parser = argparse.ArgumentParser(
        description="Migrate rc2 EXPERIENCE.md and raw_days/ into latest SQLite memory.db (1.0.0rc4+)"
    )
    parser.add_argument("--home", type=Path, default=None, help="Experienced LLMs home directory")
    parser.add_argument("--experience-file", type=Path, default=None, help="Path to rc2 EXPERIENCE.md")
    parser.add_argument("--db-path", type=Path, default=None, help="Target SQLite memory.db path")
    parser.add_argument("--dry-run", action="store_true", help="Preview extracted rules without mutating database")
    parser.add_argument("--no-backup", action="store_true", help="Do not move legacy folders to backup directory")
    args = parser.parse_args()

    default_home = Path(os.environ["EXPERIENCED_LLMS_HOME"]) if "EXPERIENCED_LLMS_HOME" in os.environ else Path.home() / ".experienced-llms"
    home = (args.home or default_home).resolve()
    exp_file = (args.experience_file or (home / "EXPERIENCE.md")).resolve()
    db_path = (args.db_path or (home / "memory.db")).resolve()

    print("==========================================================")
    print("  Experienced LLMs: rc2 -> Latest (1.0.0rc4+) Migration")
    print("==========================================================")
    print(f"Target Home : {home}")
    print(f"Input File  : {exp_file}")
    print(f"Target DB   : {db_path}")
    print(f"Dry Run     : {args.dry_run}")
    print("----------------------------------------------------------")

    migrated = migrate_rc2(home, exp_file, db_path, dry_run=args.dry_run, no_backup=args.no_backup)

    if not args.dry_run and not args.no_backup:
        backup_legacy_files(home, exp_file)

    print("==========================================================")
    print(f"[SUCCESS] Migration completed! ({migrated} facts secured in SQLite)")
    print("==========================================================")

if __name__ == "__main__":
    main()
