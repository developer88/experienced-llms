"""Prompt builder that packages memory, skills, and self-learning protocols for LLMs."""

from typing import Optional, List
import src.config as config
from src.db import get_active_facts, get_active_skills
from src.models import Skill, MemoryFact

def build_orchestrator_prompt(scope: Optional[str] = None) -> str:
    """
    Builds a complete, self-explanatory system prompt for high-level orchestrators
    (Gemini 3.8 Flash, Claude 3.5 Sonnet, GPT-4o, etc.).
    """
    facts = get_active_facts(scope=scope)
    skills = get_active_skills()

    # Format rules (deduplicating identical active statements)
    rules_text = ""
    if facts:
        seen = set()
        unique_facts = []
        for f in facts:
            rule_key = (f.scope, f.rule_statement.lower().strip())
            if rule_key not in seen:
                seen.add(rule_key)
                unique_facts.append(f)
        rules_text = "\n".join(f"- [{f.category.value}] ({f.scope}): {f.rule_statement}" for f in unique_facts)
    else:
        rules_text = "- (No past constraints recorded yet. Learn as you interact)."

    # Format skills catalog
    skills_text = ""
    if skills:
        skills_text = "\n".join(f"- `{s.key}`: {s.role_description}" for s in skills)
    else:
        skills_text = "- (No custom skills compiled yet)."

    prompt = f"""# Experienced LLM Operating Protocol

You are an adaptive AI orchestrator equipped with a persistent memory and self-learning engine.
Your behavior is guided by accumulated experience from past sessions.

## 1. Active User Profile & Operational Directives
Obey these rules strictly. They represent verified preferences, architectural choices, and lessons learned from past mistakes:
{rules_text}

## 2. Available JIT Skills Catalog
When decomposing tasks or dispatching instructions to local workers or tool calls, reference these modular skills when applicable:
{skills_text}

## 3. Operational Heuristic Logging
Whenever a user corrects your code/approach, specifies an operational preference, or an edge case is discovered, execute:
```bash
experienced-llms log "<rule statement>" --reason "<context or why>" --category [defensive_heuristic|user_preference|technical_decision|project_gotcha|mistake_correction]
```
This deterministically updates your long-term memory with zero token overhead.
"""
    return prompt.strip()

def build_worker_prompt(active_skill_key: Optional[str] = None) -> str:
    """
    Builds an ultra-compact prompt (<300 tokens) for small local worker models (7B/14B).
    Injects only core profile and exactly one skill definition.
    """
    core_content = ""
    if config.CORE_PROFILE_FILE.exists():
        with open(config.CORE_PROFILE_FILE, "r", encoding="utf-8") as f:
            core_content = f.read().strip()
    else:
        core_content = "Execute tasks precisely with minimal boilerplate."

    skill_content = ""
    if active_skill_key:
        skills = get_active_skills()
        matching = [s for s in skills if s.key == active_skill_key]
        if matching:
            s = matching[0]
            skill_content = f"\n\n## ACTIVE TASK SKILL: {s.key}\n{s.rules_summary}"

    prompt = f"""{core_content}{skill_content}

Strict execution rule: Follow constraints without introductory text or conversational filler."""
    return prompt.strip()

def export_active_context_file() -> str:
    """Exports ACTIVE_CONTEXT.md to the memory directory for easy inclusion in any LLM tool."""
    content = build_orchestrator_prompt()
    out_path = config.MEMORY_ROOT / "ACTIVE_CONTEXT.md"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as f:
        f.write(content + "\n")
    return str(out_path)
