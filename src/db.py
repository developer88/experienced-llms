import sqlite3
from pathlib import Path
from typing import List, Optional
import src.config as config
from src.models import MemoryFact, FactCategory, FactStatus, Skill

def get_connection(db_path: Optional[Path] = None) -> sqlite3.Connection:
    target = db_path or config.DB_PATH
    target.parent.mkdir(parents=True, exist_ok=True)
    conn = sqlite3.connect(str(target))
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA foreign_keys = ON")
    return conn

def init_db(db_path: Optional[Path] = None):
    config.ensure_directories()
    conn = get_connection(db_path)
    schema_path = config.BASE_DIR / "schema.sql"
    with open(schema_path, "r", encoding="utf-8") as f:
        conn.executescript(f.read())
    conn.commit()
    conn.close()

def insert_session(session_id: str, source: str = "chat", raw_path: str = "", conn: Optional[sqlite3.Connection] = None):
    should_close = False
    if conn is None:
        conn = get_connection()
        should_close = True
    try:
        conn.execute(
            """
            INSERT INTO sessions (id, source, raw_path)
            VALUES (?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET raw_path=excluded.raw_path
            """,
            (session_id, source, raw_path)
        )
        conn.commit()
    finally:
        if should_close:
            conn.close()

def insert_fact(fact: MemoryFact, conn: Optional[sqlite3.Connection] = None) -> int:
    should_close = False
    if conn is None:
        conn = get_connection()
        should_close = True
    try:
        cursor = conn.execute(
            """
            INSERT INTO memory_facts (
                session_id, category, scope, rule_statement,
                context_reason, confidence, status, superseded_by
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                fact.session_id,
                fact.category.value if isinstance(fact.category, FactCategory) else str(fact.category),
                fact.scope,
                fact.rule_statement,
                fact.context_reason,
                fact.confidence,
                fact.status.value if isinstance(fact.status, FactStatus) else str(fact.status),
                fact.superseded_by,
            )
        )
        conn.commit()
        return cursor.lastrowid
    finally:
        if should_close:
            conn.close()

def supersede_fact(old_fact_id: int, new_fact_id: int, conn: Optional[sqlite3.Connection] = None):
    should_close = False
    if conn is None:
        conn = get_connection()
        should_close = True
    try:
        conn.execute(
            """
            UPDATE memory_facts
            SET status = 'superseded', superseded_by = ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            """,
            (new_fact_id, old_fact_id)
        )
        conn.commit()
    finally:
        if should_close:
            conn.close()

def get_active_facts(scope: Optional[str] = None, conn: Optional[sqlite3.Connection] = None) -> List[MemoryFact]:
    should_close = False
    if conn is None:
        conn = get_connection()
        should_close = True
    try:
        query = "SELECT * FROM memory_facts WHERE status IN ('active', 'user_verified')"
        params = []
        if scope:
            query += " AND (scope = ? OR scope = 'global')"
            params.append(scope)
        query += " ORDER BY id ASC"

        rows = conn.execute(query, params).fetchall()
        return [_row_to_fact(row) for row in rows]
    finally:
        if should_close:
            conn.close()

def get_facts_by_date(date_str: str, conn: Optional[sqlite3.Connection] = None) -> List[MemoryFact]:
    should_close = False
    if conn is None:
        conn = get_connection()
        should_close = True
    try:
        rows = conn.execute(
            """
            SELECT * FROM memory_facts
            WHERE strftime('%Y-%m-%d', created_at) = ?
            ORDER BY id ASC
            """,
            (date_str,)
        ).fetchall()
        return [_row_to_fact(row) for row in rows]
    finally:
        if should_close:
            conn.close()

def upsert_skill(skill: Skill, conn: Optional[sqlite3.Connection] = None):
    should_close = False
    if conn is None:
        conn = get_connection()
        should_close = True
    try:
        conn.execute(
            """
            INSERT INTO skills (key, file_path, role_description, rules_summary, token_count, status)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(key) DO UPDATE SET
                file_path=excluded.file_path,
                role_description=excluded.role_description,
                rules_summary=excluded.rules_summary,
                token_count=excluded.token_count,
                status=excluded.status,
                updated_at=CURRENT_TIMESTAMP
            """,
            (skill.key, skill.file_path, skill.role_description, skill.rules_summary, skill.token_count, skill.status)
        )
        conn.commit()
    finally:
        if should_close:
            conn.close()

def get_active_skills(conn: Optional[sqlite3.Connection] = None) -> List[Skill]:
    should_close = False
    if conn is None:
        conn = get_connection()
        should_close = True
    try:
        rows = conn.execute("SELECT * FROM skills WHERE status = 'active' ORDER BY key ASC").fetchall()
        return [
            Skill(
                key=r["key"],
                file_path=r["file_path"],
                role_description=r["role_description"],
                rules_summary=r["rules_summary"],
                token_count=r["token_count"],
                status=r["status"],
            )
            for r in rows
        ]
    finally:
        if should_close:
            conn.close()

def record_daily_consolidation(date_str: str, file_path: str, facts_count: int, skills_updated: int, conn: Optional[sqlite3.Connection] = None):
    should_close = False
    if conn is None:
        conn = get_connection()
        should_close = True
    try:
        conn.execute(
            """
            INSERT INTO daily_consolidations (date, file_path, facts_count, skills_updated)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(date) DO UPDATE SET
                file_path=excluded.file_path,
                facts_count=excluded.facts_count,
                skills_updated=excluded.skills_updated
            """,
            (date_str, file_path, facts_count, skills_updated)
        )
        conn.commit()
    finally:
        if should_close:
            conn.close()

def _row_to_fact(row: sqlite3.Row) -> MemoryFact:
    return MemoryFact(
        id=row["id"],
        session_id=row["session_id"],
        category=FactCategory(row["category"]),
        scope=row["scope"],
        rule_statement=row["rule_statement"],
        context_reason=row["context_reason"],
        confidence=row["confidence"],
        status=FactStatus(row["status"]),
        superseded_by=row["superseded_by"],
    )
