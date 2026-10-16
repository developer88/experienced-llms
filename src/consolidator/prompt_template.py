"""Consolidation Prompt Template and Loader.

Provides the canonical consolidation instructions, section schemas, and
custom template resolution mechanism for Experienced LLMs.
"""

from pathlib import Path
from typing import Optional
import src.config as config

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

def get_consolidation_template(
    custom_path: Optional[Path] = None,
    home_dir: Optional[Path] = None
) -> str:
    """
    Resolves consolidation template with priority:
    1. Explicit custom_path if provided and exists.
    2. ~/.experienced-llms/consolidation_prompt.txt if exists.
    3. DEFAULT_CONSOLIDATION_TEMPLATE.
    """
    if custom_path and Path(custom_path).exists():
        with open(custom_path, "r", encoding="utf-8") as f:
            return f.read()

    home = home_dir or config.EXPERIENCED_HOME
    user_custom = home / "consolidation_prompt.txt"
    if user_custom.exists():
        with open(user_custom, "r", encoding="utf-8") as f:
            return f.read()

    return DEFAULT_CONSOLIDATION_TEMPLATE
