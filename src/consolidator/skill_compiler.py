import json
import re
from datetime import datetime
from pathlib import Path
from typing import List

import src.config as config
from src.db import get_active_facts, upsert_skill, get_connection
from src.llm_client import BaseLLMClient
from src.models import MemoryFact, FactCategory, Skill

SKILL_SYNTHESIS_PROMPT = """You are an expert prompt engineer specializing in small local models (7B/14B parameters).
Small models require ultra-compact, deterministic "Just-In-Time" (JIT) skill wrappers (150-300 tokens).

Analyze the provided active memory facts and procedural lessons. Identify if any cluster of rules warrants creating or updating an operational skill wrapper (e.g. coding conventions, framework rules, tooling commands).

Return a JSON object:
{
  "skills_to_generate": [
    {
      "key": "snake_case_skill_identifier",
      "role": "Single sentence describing the operational role",
      "rules": [
        "Rule 1...",
        "Rule 2..."
      ],
      "input_contract": "What the model receives",
      "output_contract": "What the model must return"
    }
  ]
}
If no procedural skills need to be generated (e.g. rules are only general personal preferences), return {"skills_to_generate": []}.
"""

CORE_PROFILE_PROMPT = """You are a memory compaction engine.
Given the list of user rules and preferences, condense them into an ultra-compact master profile.
STRICT LIMIT: The entire output must be under 150 words (approx 200 tokens).
Format as concise bullet points grouped by:
- Directives & Constraints
- Technical Preferences
Do NOT output conversational filler.
"""

class SkillCompiler:
    def __init__(self, llm_client: BaseLLMClient):
        self.llm_client = llm_client

    def compile_all(self) -> List[Skill]:
        config.ensure_directories()
        facts = get_active_facts()
        if not facts:
            return []

        # 1. Compile Core Profile (< 200 tokens)
        self.compile_core_profile(facts)

        # 2. Synthesize JIT Skills
        skills = self.synthesize_jit_skills(facts)

        # 3. Export LLM-ready context file
        from src.prompt_builder import export_active_context_file
        export_active_context_file()

        return skills

    def compile_core_profile(self, facts: List[MemoryFact]) -> Path:
        fact_lines = [f"- [{f.category.value}] {f.rule_statement}" for f in facts]
        user_input = "Active Rules and Facts:\n" + "\n".join(fact_lines)

        compact_profile = self.llm_client.generate(
            system_prompt=CORE_PROFILE_PROMPT,
            user_prompt=user_input,
            json_mode=False
        ).strip()

        # Sanitize word count
        words = compact_profile.split()
        if len(words) > config.CORE_PROFILE_MAX_WORDS * 1.5:
            compact_profile = " ".join(words[:int(config.CORE_PROFILE_MAX_WORDS * 1.5)]) + "..."

        header = f"# Core User & System Profile\n<!-- Auto-generated: {datetime.now().strftime('%Y-%m-%d %H:%M')} -->\n\n"
        full_content = header + compact_profile + "\n"

        with open(config.CORE_PROFILE_FILE, "w", encoding="utf-8") as f:
            f.write(full_content)

        return config.CORE_PROFILE_FILE

    def synthesize_jit_skills(self, facts: List[MemoryFact]) -> List[Skill]:
        # Filter for technical decisions, mistakes, and project gotchas
        relevant_facts = [
            f for f in facts
            if f.category in (FactCategory.TECHNICAL_DECISION, FactCategory.MISTAKE_CORRECTION, FactCategory.PROJECT_GOTCHA)
        ]
        if not relevant_facts:
            return []

        fact_payload = [
            {
                "id": f.id,
                "scope": f.scope,
                "rule": f.rule_statement,
                "context": f.context_reason
            }
            for f in relevant_facts
        ]

        raw_response = self.llm_client.generate(
            system_prompt=SKILL_SYNTHESIS_PROMPT,
            user_prompt=f"Memory Facts for Skill Synthesis:\n{json.dumps(fact_payload, indent=2)}",
            json_mode=True
        )

        generated_skills = self._parse_skills(raw_response)
        conn = get_connection()
        try:
            for skill in generated_skills:
                # Write skill markdown file
                skill_path = Path(skill.file_path)
                skill_path.parent.mkdir(parents=True, exist_ok=True)
                md_content = self._render_skill_markdown(skill)
                with open(skill_path, "w", encoding="utf-8") as f:
                    f.write(md_content)

                skill.token_count = len(md_content.split())
                upsert_skill(skill, conn=conn)
        finally:
            conn.close()

        return generated_skills

    def _parse_skills(self, raw_json: str) -> List[Skill]:
        try:
            cleaned = raw_json.strip()
            if cleaned.startswith("```"):
                cleaned = re.sub(r"^```(?:json)?\n?", "", cleaned)
                cleaned = re.sub(r"\n?```$", "", cleaned)
            data = json.loads(cleaned)
        except Exception:
            return []

        results = []
        for item in data.get("skills_to_generate", []):
            raw_key = item.get("key", "").strip()
            key = re.sub(r"[^a-zA-Z0-9_]", "_", raw_key).lower()
            if not key.startswith("auto_"):
                key = f"auto_{key}"

            role = item.get("role", "").strip()
            rules = item.get("rules", [])
            in_c = item.get("input_contract", "").strip()
            out_c = item.get("output_contract", "").strip()

            rules_summary = "\n".join(f"- {r}" for r in rules)
            file_path = str(config.SKILLS_DIR / f"{key}.md")

            skill = Skill(
                key=key,
                file_path=file_path,
                role_description=role,
                rules_summary=f"{rules_summary}\n\n**Contract**:\nInput: {in_c}\nOutput: {out_c}",
                status="active"
            )
            results.append(skill)

        return results

    def _render_skill_markdown(self, skill: Skill) -> str:
        return f"""# SKILL: {skill.key}
## Role
{skill.role_description}

## Rules
{skill.rules_summary}
"""
