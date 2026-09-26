"""Intraday Collector: Deterministic, zero-LLM, append-only logger for daytime sessions."""

from datetime import datetime
from pathlib import Path
from typing import Optional, List

import src.config as config
from src.db import insert_fact, get_connection
from src.models import MemoryFact, FactCategory, FactStatus

def log_intraday(
    rule_statement: str,
    category: str = "user_preference",
    scope: str = "global",
    context_reason: str = "",
    session_id: str = ""
) -> MemoryFact:
    """
    Appends a new interaction rule/lesson directly to today's raw markdown file
    and saves it in SQLite.
    NO LLM is called here: 100% deterministic, instant, free, and works offline.
    """
    config.ensure_directories()
    now = datetime.now()
    date_str = now.strftime("%Y-%m-%d")
    time_str = now.strftime("%H:%M:%S")

    # Map category string to FactCategory
    try:
        cat_enum = FactCategory(category.lower())
    except ValueError:
        cat_enum = FactCategory.USER_PREFERENCE

    # 1. Append directly to today's raw day markdown file
    raw_day_file = config.RAW_DAYS_DIR / f"{date_str}.md"
    file_exists = raw_day_file.exists()

    entry_lines = [
        f"### [{time_str}] [{cat_enum.value.upper()}] ({scope})",
        f"- **Rule**: {rule_statement}",
    ]
    if context_reason:
        entry_lines.append(f"- **Reason/Context**: {context_reason}")
    if session_id:
        entry_lines.append(f"- **Session**: {session_id}")
    entry_lines.append("")

    with open(raw_day_file, "a", encoding="utf-8") as f:
        if not file_exists:
            f.write(f"# Raw Intraday Experience: {date_str}\n\n")
        f.write("\n".join(entry_lines) + "\n")

    # 2. Record in SQLite for indexing and status tracking
    actual_session_id = session_id or f"intraday_{date_str}"

    conn = get_connection(config.DB_PATH)
    try:
        from src.db import insert_session
        insert_session(session_id=actual_session_id, source="intraday", raw_path=str(raw_day_file), conn=conn)

        fact = MemoryFact(
            session_id=actual_session_id,
            category=cat_enum,
            scope=scope,
            rule_statement=rule_statement,
            context_reason=context_reason,
            status=FactStatus.ACTIVE,
        )
        fact_id = insert_fact(fact, conn=conn)
        fact.id = fact_id
    finally:
        conn.close()

    return fact

def get_today_raw_file() -> Path:
    date_str = datetime.now().strftime("%Y-%m-%d")
    return config.RAW_DAYS_DIR / f"{date_str}.md"

def get_raw_day_files() -> List[Path]:
    config.ensure_directories()
    return sorted(config.RAW_DAYS_DIR.glob("*.md"))
