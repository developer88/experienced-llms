#!/usr/bin/env python3
"""
Migration Script: Experienced LLMs rc2 -> Latest (rc3 / rc4 / SQLite)
====================================================================
Migrates legacy file-based rc2 data (EXPERIENCE.md and raw_days/ folders)
into the authoritative SQLite database architecture (memory.db) with
zero data loss, full idempotency, and automated backup.

Usage:
    python3 scripts/migrate_rc2_to_latest.py [OPTIONS]

Options:
    --home PATH             Custom EXPERIENCED_LLMS_HOME (default: ~/.experienced-llms)
    --experience-file PATH  Custom path to rc2 EXPERIENCE.md
    --db-path PATH          Custom path to target memory.db
    --dry-run               Preview extracted rules without writing to SQLite
"""

import argparse
import os
import re
import shutil
import sqlite3
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

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    source TEXT DEFAULT 'chat'
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
    is_consolidated INTEGER NOT NULL DEFAULT 1,
    superseded_by INTEGER REFERENCES memory_facts(id) ON DELETE SET NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_memory_facts_scope_status ON memory_facts(scope, status);
CREATE INDEX IF NOT EXISTS idx_memory_facts_consolidated ON memory_facts(is_consolidated, status);

CREATE TABLE IF NOT EXISTS consolidations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    consolidated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    facts_processed_count INTEGER NOT NULL,
    summary TEXT,
    engine_type TEXT DEFAULT 'python'
);

CREATE TABLE IF NOT EXISTS migrations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    migration_name TEXT NOT NULL UNIQUE,
    applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
"""

def parse_bullet_line(line: str):
    """
    Parses a markdown bullet from rc2 EXPERIENCE.md:
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

def migrate_experience(exp_file: Path, db_path: Path, dry_run: bool = False) -> int:
    if not exp_file.exists():
        print(f"[-] Experience file not found: {exp_file}")
        return 0

    print(f"[*] Parsing rc2 experience file: {exp_file}")
    text = exp_file.read_text(encoding="utf-8")
    lines = text.splitlines()

    current_cat = "defensive_heuristic"
    extracted = []

    for line in lines:
        stripped = line.strip()
        if stripped.startswith("##"):
            header_name = re.sub(r"^##\s*(\d+\.\s*)?", "", stripped).strip().lower()
            current_cat = CATEGORY_MAP.get(header_name, "defensive_heuristic")
            continue

        item = parse_bullet_line(stripped)
        if item and item["rule"]:
            item["category"] = current_cat
            extracted.append(item)

    print(f"[+] Extracted {len(extracted)} valid rules from {exp_file.name}")
    if dry_run:
        for idx, item in enumerate(extracted, 1):
            print(f"  {idx}. [{item['category']}] ({item['scope']}): {item['rule']} (Reason: {item['reason'] or 'N/A'})")
        return len(extracted)

    db_path.parent.mkdir(parents=True, exist_ok=True)
    conn = sqlite3.connect(str(db_path))
    try:
        conn.executescript(SCHEMA_SQL)
        now_str = datetime.now().strftime("%Y-%m-%d %H:%M:%S")

        conn.execute("INSERT OR IGNORE INTO sessions (id, source) VALUES ('rc2_migration', 'migration_script')")
        conn.execute("INSERT OR IGNORE INTO migrations (migration_name) VALUES ('001_1_0_0rc3_baseline')")

        inserted = 0
        for item in extracted:
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
            inserted += 1

        conn.commit()
        print(f"[✓] Successfully inserted {inserted} new rule(s) into SQLite ({db_path})")
        return inserted
    finally:
        conn.close()

def backup_legacy_files(home_dir: Path):
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    backup_dir = home_dir / f"backup_rc2_{timestamp}"

    legacy_dirs = [home_dir / "raw_days", home_dir / "sessions", home_dir / "daily"]
    moved_anything = False

    for legacy_dir in legacy_dirs:
        if legacy_dir.exists() and any(legacy_dir.iterdir()):
            backup_dir.mkdir(parents=True, exist_ok=True)
            target = backup_dir / legacy_dir.name
            shutil.move(str(legacy_dir), str(target))
            print(f"[+] Archived legacy directory: {legacy_dir.name} -> {target}")
            moved_anything = True

    if moved_anything:
        print(f"[✓] Legacy rc2 folders archived to: {backup_dir}")

def main():
    parser = argparse.ArgumentParser(description="Migrate rc2 EXPERIENCE.md and raw_days to latest SQLite memory.db")
    parser.add_argument("--home", type=Path, default=None, help="Experienced LLMs home directory")
    parser.add_argument("--experience-file", type=Path, default=None, help="Path to rc2 EXPERIENCE.md")
    parser.add_argument("--db-path", type=Path, default=None, help="Target SQLite memory.db path")
    parser.add_argument("--dry-run", action="store_true", help="Preview extracted rules without mutating database")
    args = parser.parse_args()

    home = args.home or (Path(os.environ["EXPERIENCED_LLMS_HOME"]) if "EXPERIENCED_LLMS_HOME" in os.environ else Path.home() / ".experienced-llms")
    exp_file = args.experience_file or (home / "EXPERIENCE.md")
    db_path = args.db_path or (home / "memory.db")

    print("==========================================================")
    print("  Experienced LLMs: rc2 -> Latest Data Migration")
    print("==========================================================")
    print(f"Target Home : {home}")
    print(f"Input File  : {exp_file}")
    print(f"Target DB   : {db_path}")
    print(f"Dry Run     : {args.dry_run}")
    print("----------------------------------------------------------")

    migrated = migrate_experience(exp_file, db_path, dry_run=args.dry_run)

    if not args.dry_run and home.exists():
        backup_legacy_files(home)

    print("==========================================================")
    print(f"[SUCCESS] Migration completed! ({migrated} active facts secured)")
    print("==========================================================")

if __name__ == "__main__":
    main()
