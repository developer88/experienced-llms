import json
import re
from datetime import datetime
from pathlib import Path
from typing import List, Optional

import src.config as config
from src.db import insert_session, insert_fact, get_connection
from src.llm_client import BaseLLMClient
from src.models import FactCategory, FactStatus, MemoryFact, SessionExtractResult

EXTRACTION_SYSTEM_PROMPT = """You are an expert cognitive extractor for an autonomous AI memory system.
Your job is to analyze interaction transcripts between a user and an AI coding assistant, and extract high-signal lessons, user preferences, technical decisions, and mistake corrections.

STRICT RULES:
1. Ignore casual greetings, generic polite filler, and standard programming knowledge (e.g. do not log that Python uses def).
2. Extract ONLY persistent rules, user-enforced workflows, explicit corrections, and project-specific gotchas.
3. Categorize each item into exactly one of:
   - "user_preference": Directives on how the user prefers code or communication (e.g., "Prefers concise comments", "Always use npm in WSL").
   - "technical_decision": Architecture or dependency choices (e.g., "Use SQLite instead of Postgres for local agent state").
   - "mistake_correction": When the model erred and the user or runtime corrected it (e.g., "Failed on CRLF line endings, switched to LF").
   - "project_gotcha": Specific quirks of the current repo or environment (e.g., "Running node from Windows fails; must run inside WSL").

Output MUST be a JSON object with this exact structure:
{
  "facts": [
    {
      "category": "user_preference" | "technical_decision" | "mistake_correction" | "project_gotcha",
      "scope": "global" | "<project-name>",
      "rule_statement": "Concise operational rule statement (1-2 sentences max)",
      "context_reason": "Why this rule exists or what failed",
      "confidence": 0.95
    }
  ]
}
If no relevant facts or lessons exist in the transcript, return {"facts": []}.
"""

class SessionExtractor:
    def __init__(self, llm_client: BaseLLMClient):
        self.llm_client = llm_client

    def extract_from_text(
        self,
        transcript: str,
        session_id: Optional[str] = None,
        source: str = "chat",
        scope_default: str = "global",
    ) -> SessionExtractResult:
        config.ensure_directories()
        if not session_id:
            session_id = datetime.now().strftime("%Y-%m-%d_%H%M%S")

        explicit_facts = self._extract_explicit_learnings(transcript, session_id, scope_default)
        if explicit_facts:
            facts = explicit_facts
        else:
            prompt = f"Transcript to analyze (Default Scope: {scope_default}):\n\n{transcript}"
            raw_response = self.llm_client.generate(
                system_prompt=EXTRACTION_SYSTEM_PROMPT,
                user_prompt=prompt,
                json_mode=True
            )
            facts = self._parse_response(raw_response, session_id, scope_default)

        # Write session markdown file
        session_md_path = config.SESSIONS_DIR / f"{session_id}.md"
        md_content = self._render_markdown(session_id, facts, transcript)
        with open(session_md_path, "w", encoding="utf-8") as f:
            f.write(md_content)

        # Store in SQLite
        conn = get_connection()
        try:
            insert_session(session_id=session_id, source=source, raw_path=str(session_md_path), conn=conn)
            for fact in facts:
                fact_id = insert_fact(fact, conn=conn)
                fact.id = fact_id
        finally:
            conn.close()

        return SessionExtractResult(session_id=session_id, facts=facts, raw_markdown=md_content)

    def _extract_explicit_learnings(self, transcript: str, session_id: str, scope_default: str) -> List[MemoryFact]:
        pattern = r"<session_learning>(.*?)</session_learning>"
        matches = re.findall(pattern, transcript, re.DOTALL | re.IGNORECASE)
        if not matches:
            return []

        facts = []
        for block in matches:
            for line in block.strip().splitlines():
                line = line.strip().lstrip("-* ").strip()
                if not line:
                    continue
                m = re.match(r"^\[(\w+)\](?:\s*\(([^)]+)\))?:\s*([^|]+)(?:\|\s*(.*))?$", line)
                if m:
                    cat_str, scope, rule, reason = m.groups()
                    try:
                        category = FactCategory(cat_str.lower())
                    except ValueError:
                        category = FactCategory.USER_PREFERENCE
                    facts.append(
                        MemoryFact(
                            session_id=session_id,
                            category=category,
                            scope=scope.strip() if scope else scope_default,
                            rule_statement=rule.strip(),
                            context_reason=reason.strip() if reason else "Explicitly emitted by LLM during session",
                            confidence=1.0,
                            status=FactStatus.ACTIVE,
                        )
                    )
        return facts

    def _parse_response(self, raw_json: str, session_id: str, default_scope: str) -> List[MemoryFact]:
        facts = []
        try:
            cleaned = raw_json.strip()
            # Handle markdown json code fences if present
            if cleaned.startswith("```"):
                cleaned = re.sub(r"^```(?:json)?\n?", "", cleaned)
                cleaned = re.sub(r"\n?```$", "", cleaned)
            data = json.loads(cleaned)
        except Exception:
            # Fallback if invalid JSON returned
            return []

        raw_facts = data.get("facts", [])
        for item in raw_facts:
            cat_str = item.get("category", "user_preference")
            try:
                category = FactCategory(cat_str)
            except ValueError:
                category = FactCategory.USER_PREFERENCE

            facts.append(
                MemoryFact(
                    session_id=session_id,
                    category=category,
                    scope=item.get("scope", default_scope),
                    rule_statement=item.get("rule_statement", "").strip(),
                    context_reason=item.get("context_reason", "").strip(),
                    confidence=float(item.get("confidence", 1.0)),
                    status=FactStatus.ACTIVE,
                )
            )
        return facts

    def _render_markdown(self, session_id: str, facts: List[MemoryFact], transcript: str) -> str:
        lines = [
            f"# Session Memory: {session_id}",
            f"Extracted at: {datetime.now().isoformat()}",
            "",
            "## Extracted Key Points",
        ]
        if not facts:
            lines.append("- *No persistent rules or corrections identified.*")
        else:
            for f in facts:
                lines.append(f"- **[{f.category.value.upper()}]** ({f.scope}): {f.rule_statement}")
                if f.context_reason:
                    lines.append(f"  *Reason/Context*: {f.context_reason}")

        lines.extend([
            "",
            "---",
            "## Original Transcript Snippet",
            "```text",
            transcript[:1500] + ("..." if len(transcript) > 1500 else ""),
            "```",
            ""
        ])
        return "\n".join(lines)
