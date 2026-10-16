"""Full-Stack Migration Runner (Omakub / Rails Style).

Manages sequential upgrades across 4 distinct operational layers:
1. SQLite Database Schema & Data
2. Agent Skill Definitions (SKILL.md)
3. Executable Scripts & CLI Tooling
4. Configuration (config.json)
"""

import sys
import os
import re
import sqlite3
import subprocess
import importlib.util
from pathlib import Path
from typing import List, Tuple, Optional, Dict, Any

import src.config as config
from src.db import get_connection, init_db, BASELINE_VERSION

MIGRATIONS_DIR = config.BASE_DIR / "migrations"

def get_applied_migrations(conn: Optional[sqlite3.Connection] = None) -> List[str]:
    """Returns a sorted list of applied migration versions from schema_migrations."""
    should_close = False
    if conn is None:
        if not config.DB_PATH.exists():
            return []
        conn = get_connection(config.DB_PATH)
        should_close = True
    try:
        table_check = conn.execute(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='schema_migrations'"
        ).fetchone()
        if not table_check:
            return []
        rows = conn.execute("SELECT version FROM schema_migrations ORDER BY version ASC").fetchall()
        return [r["version"] for r in rows]
    finally:
        if should_close:
            conn.close()

def get_available_migrations(migrations_dir: Optional[Path] = None) -> List[Tuple[str, Path]]:
    """Returns a list of available migration scripts sorted by version prefix."""
    target_dir = migrations_dir or MIGRATIONS_DIR
    if not target_dir.exists():
        return []

    pattern = re.compile(r"^(\d{3}_[\w\-]+)\.(py|sql|sh|ps1)$")
    migrations = []
    for item in sorted(target_dir.iterdir()):
        if item.is_file():
            m = pattern.match(item.name)
            if m:
                version = m.group(1)
                migrations.append((version, item))
    return sorted(migrations, key=lambda x: x[0])

def get_pending_migrations(
    conn: Optional[sqlite3.Connection] = None,
    migrations_dir: Optional[Path] = None
) -> List[Tuple[str, Path]]:
    """Finds all available migrations that have not yet been recorded in schema_migrations."""
    applied = set(get_applied_migrations(conn))
    available = get_available_migrations(migrations_dir)
    return [(v, p) for v, p in available if v not in applied]

def apply_migration(
    version: str,
    script_path: Path,
    conn: Optional[sqlite3.Connection] = None
) -> bool:
    """
    Executes a single migration script across full-stack layers
    and records the version in schema_migrations.
    """
    should_close = False
    if conn is None:
        conn = get_connection(config.DB_PATH)
        should_close = True
    try:
        suffix = script_path.suffix.lower()
        if suffix == ".py":
            spec = importlib.util.spec_from_file_location(version, script_path)
            if spec and spec.loader:
                module = importlib.util.module_from_spec(spec)
                spec.loader.exec_module(module)
                context = {
                    "conn": conn,
                    "config": config,
                    "home_dir": config.EXPERIENCED_HOME,
                    "db_path": config.DB_PATH,
                }
                if hasattr(module, "migrate"):
                    module.migrate(context)
                elif hasattr(module, "run"):
                    module.run()
        elif suffix == ".sql":
            with open(script_path, "r", encoding="utf-8") as f:
                conn.executescript(f.read())
        elif suffix == ".sh":
            subprocess.run(["bash", str(script_path)], check=True)
        elif suffix == ".ps1":
            subprocess.run(["powershell", "-File", str(script_path)], check=True)
        else:
            raise ValueError(f"Unsupported migration script format: {script_path}")

        conn.execute(
            "INSERT INTO schema_migrations (version) VALUES (?)",
            (version,)
        )
        conn.commit()
        return True
    finally:
        if should_close:
            conn.close()

def run_migrations(
    conn: Optional[sqlite3.Connection] = None,
    migrations_dir: Optional[Path] = None
) -> Tuple[bool, List[str]]:
    """
    Detects fresh vs existing installation:
    - If fresh install: initializes database with schema.sql and registers 001_1_0_0rc3_baseline.
    - If existing install: detects missing migrations and runs them sequentially.
    Returns (is_fresh, applied_versions).
    """
    is_fresh = False
    db_exists = config.DB_PATH.exists()

    if not db_exists:
        is_fresh = True
        config.ensure_directories()
        init_db(config.DB_PATH)
    else:
        applied = get_applied_migrations(conn)
        if not applied:
            init_db(config.DB_PATH)

    pending = get_pending_migrations(conn, migrations_dir)
    applied_versions = []

    for version, path in pending:
        if version == BASELINE_VERSION:
            # Baseline is recorded by init_db; skip if marker script exists
            continue
        print(f"  [>] Applying migration: {version} ({path.name})...")
        apply_migration(version, path, conn)
        print(f"  [✓] Applied migration: {version}")
        applied_versions.append(version)

    return is_fresh, applied_versions

def main():
    print("=== Experienced LLMs Migration Runner ===")
    is_fresh, applied = run_migrations()
    if is_fresh:
        print(f"[+] Fresh installation detected. Initialized baseline: {BASELINE_VERSION}")
    else:
        current = get_applied_migrations()
        print(f"[+] Existing installation detected. Current migrations: {current}")

    if applied:
        print(f"[✓] Successfully applied {len(applied)} migration(s): {applied}")
    else:
        print("[✓] All migrations up to date. Zero pending migrations.")

if __name__ == "__main__":
    main()
