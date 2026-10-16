"""Interday Consolidator: LLM engine that merges daily experiences into EXPERIENCE.md."""

import json
import re
from datetime import datetime
from pathlib import Path
from typing import List, Optional, Tuple, Dict, Any, Union

import src.config as config
from src.db import (
    get_active_facts,
    get_pending_facts,
    mark_facts_consolidated,
    record_consolidation_audit,
    get_connection,
)
from src.llm_client import BaseLLMClient, ExternalCommandLLMClient
from src.models import MemoryFact, FactCategory, FactStatus

DEFAULT_CONSOLIDATION_TEMPLATE = """You are the master cognitive consolidation engine for Experienced LLMs.
Your job is to merge new pending intraday facts with active memory and the current EXPERIENCE.md into a single, authoritative, high-density EXPERIENCE.md document.

CANONICAL SECTIONS (Maintain these exact 5 headers in this order):
# Master AI Experience
## 1. User Directives & Preferences
## 2. Defensive Safeguards & Heuristics
## 3. Project-Specific Constraints
## 4. Anti-Patterns & Critical Gotchas
## 5. Mistake Corrections & Fixes

CRITICAL CONSOLIDATION RULES:
1. Semantic Deduplication: Merge redundant, overlapping, or paraphrased points into sharp, concise directives.
2. Contradiction Resolution: If a pending rule invalidates, corrects, or replaces an older active rule, mark the older rule's ID in superseded_fact_ids.
3. Maximum Word Count: Keep the entire output lean and operational (under {max_words} words).
4. Zero Token Waste: Absolutely ZERO HTML comments (no <!-- ... -->), zero generation timestamps, zero conversational preamble or markdown code fence wrappers in the JSON string.

YOU MUST RETURN A VALID JSON OBJECT MATCHING THIS SCHEMA:
{{
  "updated_experience_markdown": "# Master AI Experience\\n\\n## 1. User Directives & Preferences\\n...",
  "processed_pending_ids": [list of pending fact IDs integrated],
  "superseded_fact_ids": [list of older active fact IDs superseded],
  "summary_of_changes": "1-2 sentences summarizing changes made"
}}
"""

def clean_markdown_experience(raw_md: str) -> str:
    """Strips HTML comments and generation timestamps to enforce zero token waste."""
    # Strip HTML comments: <!-- ... -->
    cleaned = re.sub(r"<!--[\s\S]*?-->", "", raw_md)
    # Strip generation timestamps or metadata lines
    cleaned = re.sub(r"(?i)^.*(last consolidated|generated at|timestamp).*$\n?", "", cleaned, flags=re.MULTILINE)
    # Normalize multiple newlines
    cleaned = re.sub(r"\n{3,}", "\n\n", cleaned)
    return cleaned.strip() + "\n"

def render_active_experience_markdown(facts: Optional[List[MemoryFact]] = None) -> str:
    """Renders clean, authoritative EXPERIENCE.md directly from active SQLite facts without LLM."""
    if facts is None:
        facts = get_active_facts()

    categories = [
        (FactCategory.USER_PREFERENCE, "## 1. User Directives & Preferences"),
        (FactCategory.DEFENSIVE_HEURISTIC, "## 2. Defensive Safeguards & Heuristics"),
        (FactCategory.TECHNICAL_DECISION, "## 3. Project-Specific Constraints"),
        (FactCategory.PROJECT_GOTCHA, "## 4. Anti-Patterns & Critical Gotchas"),
        (FactCategory.MISTAKE_CORRECTION, "## 5. Mistake Corrections & Fixes"),
    ]

    lines = [
        "# Master AI Experience",
        ""
    ]

    seen_statements = set()
    for cat_enum, header in categories:
        matching = [f for f in facts if f.category == cat_enum]
        lines.append(header)
        has_items = False
        for f in matching:
            statement = f.rule_statement.strip()
            if statement.lower() not in seen_statements:
                seen_statements.add(statement.lower())
                has_items = True
                scope_prefix = f"({f.scope}): " if f.scope and f.scope != "global" else ""
                reason_suffix = f" *(Reason: {f.context_reason})*" if f.context_reason else ""
                lines.append(f"- {scope_prefix}{statement}{reason_suffix}")
        if not has_items:
            lines.append("- *(None recorded yet)*")
        lines.append("")

    return "\n".join(lines).strip() + "\n"

class InterdayConsolidator:
    def __init__(self, llm_client: Optional[BaseLLMClient] = None):
        self.llm_client = llm_client

    def build_consolidation_prompt(
        self,
        target_date: Optional[str] = None,
        max_words: Optional[int] = None,
        template_path: Optional[Path] = None
    ) -> Tuple[Optional[str], Optional[str], Dict[str, Any]]:
        """
        Gathers active facts, pending intraday facts, and existing EXPERIENCE.md.
        Constructs the system and user prompt for LLM consolidation (Pathway A & B step 1).
        """
        cfg = config.load_config()
        limit_words = max_words or cfg.get("max_experience_words", 1200)

        # 1. Read existing Master Experience
        master_content = ""
        if config.MASTER_EXPERIENCE_FILE.exists():
            with open(config.MASTER_EXPERIENCE_FILE, "r", encoding="utf-8") as f:
                master_content = f.read().strip()
        else:
            master_content = "# Master AI Experience\n\n*(Initial empty state)*"

        # 2. Gather active and pending facts
        active_facts = get_active_facts()
        pending_facts = get_pending_facts()

        if not active_facts and not pending_facts:
            return None, None, {"status": "noop", "message": "No active or pending facts to consolidate."}

        active_summary = "\n".join(
            f"- [ID:{f.id}] [{f.category.value}] ({f.scope}): {f.rule_statement}" +
            (f" (Reason: {f.context_reason})" if f.context_reason else "")
            for f in active_facts
        ) or "- *(None active)*"

        pending_summary = "\n".join(
            f"- [ID:{f.id}] [{f.category.value}] ({f.scope}): {f.rule_statement}" +
            (f" (Reason: {f.context_reason})" if f.context_reason else "")
            for f in pending_facts
        ) or "- *(None pending)*"

        # 3. Load prompt template
        template = DEFAULT_CONSOLIDATION_TEMPLATE
        custom_file = template_path or (config.EXPERIENCED_HOME / "consolidation_prompt.txt")
        if custom_file and Path(custom_file).exists():
            with open(custom_file, "r", encoding="utf-8") as f:
                template = f.read()

        system_prompt = template.replace("{max_words}", str(limit_words))

        now_str = target_date or datetime.now().strftime("%Y-%m-%d")
        user_prompt = f"""=== CURRENT MASTER EXPERIENCE.MD ===
{master_content}

=== ACTIVE CONSOLIDATED FACTS ({len(active_facts)}) ===
{active_summary}

=== PENDING NEW INTRADAY FACTS TO CONSOLIDATE ({len(pending_facts)}) ===
{pending_summary}
"""

        metadata = {
            "status": "ready",
            "date": now_str,
            "max_words": limit_words,
            "active_facts_count": len(active_facts),
            "pending_facts_count": len(pending_facts),
            "pending_fact_ids": [f.id for f in pending_facts if f.id is not None],
        }

        return system_prompt, user_prompt, metadata

    def apply_consolidation_result(
        self,
        consolidation_data: Union[str, Dict[str, Any]],
        engine_type: str = "llm"
    ) -> Tuple[Path, str]:
        """
        Parses structured LLM output, applies updates to SQLite (superseding facts, marking
        pending facts consolidated, recording audit ledger), and writes clean EXPERIENCE.md.
        """
        if isinstance(consolidation_data, str):
            cleaned = consolidation_data.strip()
            if cleaned.startswith("```"):
                cleaned = re.sub(r"^```(?:json)?\n?", "", cleaned)
                cleaned = re.sub(r"\n?```$", "", cleaned)
            try:
                data = json.loads(cleaned)
            except Exception as e:
                raise ValueError(f"Failed to parse consolidation JSON payload: {e}")
        elif isinstance(consolidation_data, dict):
            data = consolidation_data
        else:
            raise TypeError("consolidation_data must be a JSON string or dict")

        updated_md = data.get("updated_experience_markdown", "").strip()
        if not updated_md:
            raise ValueError("Consolidation data did not contain valid 'updated_experience_markdown'.")

        summary = data.get("summary_of_changes", "Consolidation applied.")
        processed_pending_ids = data.get("processed_pending_ids", [])
        superseded_ids = data.get("superseded_fact_ids", [])
        legacy_superseded = data.get("superseded_facts", [])

        # Clean markdown to guarantee zero token waste
        cleaned_md = clean_markdown_experience(updated_md)

        # Write EXPERIENCE.md
        config.ensure_directories()
        with open(config.MASTER_EXPERIENCE_FILE, "w", encoding="utf-8") as f:
            f.write(cleaned_md)

        # SQLite transaction
        conn = get_connection(config.DB_PATH)
        try:
            # 1. Supersede explicitly identified active facts
            for sid in superseded_ids:
                conn.execute(
                    "UPDATE memory_facts SET status = 'superseded', updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                    (sid,)
                )

            # 2. Support legacy superseded_facts with older_rule matching
            for item in legacy_superseded:
                if isinstance(item, dict):
                    older_rule = item.get("older_rule", "").strip().lower()
                    if older_rule:
                        conn.execute(
                            "UPDATE memory_facts SET status = 'superseded', updated_at = CURRENT_TIMESTAMP WHERE LOWER(rule_statement) LIKE ? AND status = 'active'",
                            (f"%{older_rule}%",)
                        )

            # 3. Mark pending facts consolidated
            if not processed_pending_ids:
                # If LLM didn't return explicit IDs, mark all currently pending facts
                pending_rows = conn.execute("SELECT id FROM memory_facts WHERE is_consolidated = 0").fetchall()
                processed_pending_ids = [r["id"] for r in pending_rows]

            if processed_pending_ids:
                mark_facts_consolidated(processed_pending_ids, conn=conn)

            # 4. Record audit in consolidations table
            record_consolidation_audit(
                engine_type=engine_type,
                facts_processed=len(processed_pending_ids),
                facts_superseded=len(superseded_ids) + len(legacy_superseded),
                summary=summary,
                conn=conn
            )
            conn.commit()
        finally:
            conn.close()

        return config.MASTER_EXPERIENCE_FILE, summary

    def consolidate(
        self,
        target_date: Optional[str] = None,
        max_words: Optional[int] = None,
        template_path: Optional[Path] = None
    ) -> Tuple[Path, str]:
        """
        Runs full LLM-only consolidation (Pathway A).
        Aborts cleanly without modifying SQLite or EXPERIENCE.md if LLM fails or errors.
        """
        if self.llm_client is None:
            raise RuntimeError("Consolidation requires an LLM client. Pass --llm '<cmd>' or configure provider.")

        config.ensure_directories()
        sys_prompt, user_prompt, meta = self.build_consolidation_prompt(
            target_date=target_date,
            max_words=max_words,
            template_path=template_path
        )
        if sys_prompt is None:
            return config.MASTER_EXPERIENCE_FILE, "No active or pending facts to consolidate."

        raw_response = self.llm_client.generate(
            system_prompt=sys_prompt,
            user_prompt=user_prompt,
            json_mode=True
        )

        if not raw_response or not raw_response.strip():
            raise RuntimeError("LLM returned empty consolidation response. Aborting without mutating state.")

        engine_type = "cli_external_cmd" if isinstance(self.llm_client, ExternalCommandLLMClient) else "api_client"
        return self.apply_consolidation_result(raw_response, engine_type=engine_type)
