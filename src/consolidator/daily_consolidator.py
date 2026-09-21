import json
import re
from datetime import datetime
from pathlib import Path
from typing import List, Optional, Dict, Any

import src.config as config
from src.db import get_facts_by_date, supersede_fact, record_daily_consolidation, get_connection
from src.llm_client import BaseLLMClient
from src.models import MemoryFact, FactStatus

RECONCILIATION_SYSTEM_PROMPT = """You are an expert cognitive consolidator for an AI memory system.
Your job is to analyze facts collected from multiple sessions during a day, deduplicate them, and detect contradictions.

If Fact B invalidates, replaces, or supersedes Fact A (e.g., "Switched from Library X to Library Y"), identify that Fact A is superseded by Fact B.

Output JSON structure:
{
  "superseded_pairs": [
    {"old_fact_id": 1, "new_fact_id": 4, "reason": "User explicitly migrated away from tool X to tool Y"}
  ],
  "daily_summary": "Short 2-3 sentence overview of the technical advancements and lessons of the day."
}
"""

class DailyConsolidator:
    def __init__(self, llm_client: BaseLLMClient):
        self.llm_client = llm_client

    def consolidate_date(self, date_str: Optional[str] = None) -> Path:
        config.ensure_directories()
        if not date_str:
            date_str = datetime.now().strftime("%Y-%m-%d")

        facts = get_facts_by_date(date_str)
        if not facts:
            # Create an empty or minimal daily log
            daily_file = config.DAILY_DIR / f"{date_str}.md"
            with open(daily_file, "w", encoding="utf-8") as f:
                f.write(f"# Daily Experience Log: {date_str}\n\n*No sessions or memory facts recorded for this date.*\n")
            record_daily_consolidation(date_str, str(daily_file), 0, 0)
            return daily_file

        # Check for contradictions/superseding via LLM if there are multiple facts
        summary_text = "Consolidation complete."
        if len(facts) > 1:
            fact_payload = [
                {
                    "id": f.id,
                    "category": f.category.value,
                    "scope": f.scope,
                    "rule": f.rule_statement,
                    "context": f.context_reason
                }
                for f in facts
            ]
            response = self.llm_client.generate(
                system_prompt=RECONCILIATION_SYSTEM_PROMPT,
                user_prompt=f"Analyze these facts from {date_str}:\n{json.dumps(fact_payload, indent=2)}",
                json_mode=True
            )
            summary_text = self._apply_reconciliation(response)

        # Re-fetch updated facts after reconciliation
        refreshed_facts = get_facts_by_date(date_str)
        daily_file = config.DAILY_DIR / f"{date_str}.md"
        content = self._render_daily_markdown(date_str, refreshed_facts, summary_text)

        with open(daily_file, "w", encoding="utf-8") as f:
            f.write(content)

        record_daily_consolidation(date_str, str(daily_file), len(refreshed_facts), 0)
        return daily_file

    def _apply_reconciliation(self, raw_json: str) -> str:
        try:
            cleaned = raw_json.strip()
            if cleaned.startswith("```"):
                cleaned = re.sub(r"^```(?:json)?\n?", "", cleaned)
                cleaned = re.sub(r"\n?```$", "", cleaned)
            data = json.loads(cleaned)
        except Exception:
            return "Consolidated daily interaction records."

        pairs = data.get("superseded_pairs", [])
        conn = get_connection()
        try:
            for pair in pairs:
                old_id = pair.get("old_fact_id")
                new_id = pair.get("new_fact_id")
                if old_id and new_id:
                    supersede_fact(old_id, new_id, conn=conn)
        finally:
            conn.close()

        return data.get("daily_summary", "Consolidated daily interaction records.")

    def _render_daily_markdown(self, date_str: str, facts: List[MemoryFact], summary: str) -> str:
        lines = [
            f"# Daily Consolidated Experience: {date_str}",
            f"Compiled: {datetime.now().isoformat()}",
            "",
            f"**Executive Summary**: {summary}",
            "",
            "## Active Lessons & Directives",
        ]

        active_facts = [f for f in facts if f.status in (FactStatus.ACTIVE, FactStatus.USER_VERIFIED)]
        superseded_facts = [f for f in facts if f.status == FactStatus.SUPERSEDED]

        if not active_facts:
            lines.append("- *None active.*")
        else:
            for f in active_facts:
                lines.append(f"- **[{f.category.value}]** ({f.scope}): {f.rule_statement}")
                if f.context_reason:
                    lines.append(f"  - *Context*: {f.context_reason}")

        if superseded_facts:
            lines.extend([
                "",
                "## Superseded / Deprecated Directives",
            ])
            for f in superseded_facts:
                lines.append(f"- ~~[{f.category.value}] {f.rule_statement}~~ (Replaced by Fact #{f.superseded_by})")

        lines.append("")
        return "\n".join(lines)
