"""Intraday Collector: Deterministic, zero-LLM, offline logger directly into SQLite."""

from datetime import datetime
from typing import Optional

import src.config as config
from src.db import insert_fact, get_connection, insert_session
from src.models import MemoryFact, FactCategory, FactStatus

def log_intraday(
    rule_statement: str,
    category: str = "user_preference",
    scope: str = "global",
    context_reason: str = "",
    session_id: str = ""
) -> MemoryFact:
    """
    Records an operational rule or mistake correction directly into SQLite memory_facts
    with is_consolidated = 0.
    Strictly touches NO markdown log files (2-file privacy guarantee).
    NO LLM is called here: 100% deterministic, instant, free, and works offline.
    """
    config.ensure_directories()
    now = datetime.now()
    date_str = now.strftime("%Y-%m-%d")

    # Map category string to FactCategory
    try:
        cat_enum = FactCategory(category.lower())
    except ValueError:
        cat_enum = FactCategory.USER_PREFERENCE

    actual_session_id = session_id or f"intraday_{date_str}"

    conn = get_connection(config.DB_PATH)
    try:
        insert_session(session_id=actual_session_id, source="intraday", raw_path="", conn=conn)

        fact = MemoryFact(
            session_id=actual_session_id,
            category=cat_enum,
            scope=scope,
            rule_statement=rule_statement,
            context_reason=context_reason,
            status=FactStatus.ACTIVE,
            is_consolidated=0,
        )
        fact_id = insert_fact(fact, conn=conn)
        fact.id = fact_id
    finally:
        conn.close()

    return fact
