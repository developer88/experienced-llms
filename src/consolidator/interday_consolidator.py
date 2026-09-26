"""Interday Consolidator: Nightly cron LLM engine that merges multi-day experiences into EXPERIENCE.md."""

import json
import re
from datetime import datetime
from pathlib import Path
from typing import List, Optional, Tuple

import src.config as config
from src.db import get_active_facts, supersede_fact, record_daily_consolidation, get_connection
from src.llm_client import BaseLLMClient
from src.models import MemoryFact, FactCategory, FactStatus

INTERDAY_CONSOLIDATION_PROMPT = """You are the master cognitive consolidation engine for an AI memory system.
Your job is to merge new daily experiences with the existing master experience document into a single, clean, authoritative EXPERIENCE.md.

YOUR CRITICAL RESPONSIBILITIES:
1. Contradiction Resolution: If a new rule invalidates or supersedes an older rule, remove or overwrite the older rule.
2. Deduplication: Merge redundant or repeated points into single, sharp, concise directives.
3. Organization: Group into:
   # Master AI Experience
   ## 1. User Directives & Preferences (Non-negotiable workflow rules)
   ## 2. Project-Specific Decisions & Architectures (Grouped by project)
   ## 3. Anti-Patterns & Critical Gotchas (Mistakes not to repeat)
4. Keep the output lean and authoritative (< 1,200 words). Every bullet must be operational.

Return a JSON object:
{
  "updated_experience_markdown": "# Master AI Experience\\n\\n...",
  "superseded_facts": [
    {"older_rule": "Use pnpm", "newer_rule": "Use npm in WSL", "reason": "User standardized on npm in WSL"}
  ],
  "summary_of_changes": "1-2 sentences summarizing what was merged or changed"
}
"""

class InterdayConsolidator:
    def __init__(self, llm_client: BaseLLMClient):
        self.llm_client = llm_client

    def consolidate(self, target_date: Optional[str] = None) -> Tuple[Path, str]:
        """
        Merges recent raw daily logs and active SQLite facts with the existing EXPERIENCE.md.
        Updates EXPERIENCE.md and marks superseded rules in SQLite.
        """
        config.ensure_directories()
        now = datetime.now()
        date_str = target_date or now.strftime("%Y-%m-%d")

        # 1. Read existing Master Experience
        master_content = ""
        if config.MASTER_EXPERIENCE_FILE.exists():
            with open(config.MASTER_EXPERIENCE_FILE, "r", encoding="utf-8") as f:
                master_content = f.read().strip()
        else:
            master_content = "# Master AI Experience\n\n*Initial empty experience.*"

        # 2. Collect raw daily logs that haven't been archived
        raw_files = sorted(config.RAW_DAYS_DIR.glob("*.md"))
        new_experiences_text = ""
        for rf in raw_files:
            with open(rf, "r", encoding="utf-8") as f:
                new_experiences_text += f"\n\n--- Source: {rf.name} ---\n" + f.read()

        # Also get active facts from SQLite
        facts = get_active_facts()
        facts_summary = "\n".join(
            f"- [ID:{f.id}] [{f.category.value}] ({f.scope}): {f.rule_statement} (Reason: {f.context_reason or 'N/A'})"
            for f in facts
        )

        if not new_experiences_text.strip() and not facts:
            return config.MASTER_EXPERIENCE_FILE, "No new experiences or facts to consolidate."

        # 3. Formulate consolidation prompt
        user_prompt = f"""=== CURRENT MASTER EXPERIENCE ===
{master_content}

=== NEW EXPERIENCES TO MERGE (Date: {date_str}) ===
{new_experiences_text}

=== ACTIVE STRUCTURED FACTS ===
{facts_summary}
"""

        try:
            raw_response = self.llm_client.generate(
                system_prompt=INTERDAY_CONSOLIDATION_PROMPT,
                user_prompt=user_prompt,
                json_mode=True
            )
            updated_md, summary, superseded = self._parse_consolidation_response(raw_response, master_content)
        except Exception as e:
            updated_md, summary, superseded = self._deterministic_fallback_consolidation(facts, str(e))

        # 4. Write updated EXPERIENCE.md
        with open(config.MASTER_EXPERIENCE_FILE, "w", encoding="utf-8") as f:
            f.write(updated_md + "\n")

        # 5. Handle superseded facts in SQLite if matched
        conn = get_connection(config.DB_PATH)
        try:
            for item in superseded:
                old_text = item.get("older_rule", "").lower()
                for f in facts:
                    if old_text and old_text in f.rule_statement.lower():
                        # Mark as superseded
                        conn.execute(
                            "UPDATE memory_facts SET status = 'superseded', updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                            (f.id,)
                        )
            conn.commit()
        finally:
            conn.close()

        # 6. Record consolidation entry in daily_consolidations
        record_daily_consolidation(
            date_str=date_str,
            file_path=str(config.MASTER_EXPERIENCE_FILE),
            facts_count=len(facts),
            skills_updated=0
        )

        return config.MASTER_EXPERIENCE_FILE, summary

    def _parse_consolidation_response(self, raw_json: str, fallback_md: str) -> Tuple[str, str, List[dict]]:
        try:
            cleaned = raw_json.strip()
            if cleaned.startswith("```"):
                cleaned = re.sub(r"^```(?:json)?\n?", "", cleaned)
                cleaned = re.sub(r"\n?```$", "", cleaned)
            data = json.loads(cleaned)
            md = data.get("updated_experience_markdown", "").strip() or fallback_md
            summary = data.get("summary_of_changes", "Consolidation complete.")
            superseded = data.get("superseded_facts", [])
            return md, summary, superseded
        except Exception:
            # Fallback if invalid JSON was returned
            return fallback_md, "Consolidation finished with fallback.", []

    def _deterministic_fallback_consolidation(self, facts: List[MemoryFact], error_reason: str) -> Tuple[str, str, List[dict]]:
        """Deduplicates facts and compiles a structured EXPERIENCE.md without an external LLM."""
        lines = [
            "# Master AI Experience",
            f"<!-- Consolidated: {datetime.now().strftime('%Y-%m-%d %H:%M')} (Deterministic fallback) -->\n"
        ]

        # Group facts by category, deduplicating identical statements
        categories = [
            (FactCategory.USER_PREFERENCE, "## 1. User Directives & Preferences"),
            (FactCategory.DEFENSIVE_HEURISTIC, "## 2. Defensive Safeguards & Heuristics"),
            (FactCategory.TECHNICAL_DECISION, "## 3. Project-Specific Constraints"),
            (FactCategory.PROJECT_GOTCHA, "## 4. Anti-Patterns & Critical Gotchas"),
            (FactCategory.MISTAKE_CORRECTION, "## 5. Mistake Corrections & Fixes"),
        ]

        seen_rules = set()
        total_unique = 0

        for cat_enum, header in categories:
            matching = [f for f in facts if f.category == cat_enum]
            lines.append(header)
            cat_has_entries = False
            for f in matching:
                rule_clean = f.rule_statement.strip()
                if rule_clean.lower() not in seen_rules:
                    seen_rules.add(rule_clean.lower())
                    cat_has_entries = True
                    total_unique += 1
                    line = f"- ({f.scope}): {rule_clean}"
                    if f.context_reason:
                        line += f" *(Reason: {f.context_reason})*"
                    lines.append(line)
            if not cat_has_entries:
                lines.append("- *(None recorded yet)*")
            lines.append("")

        summary = f"Consolidated {total_unique} unique facts deterministically (LLM note: {error_reason[:75]}...)"
        return "\n".join(lines), summary, []
