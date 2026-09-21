import os
import sys
from pathlib import Path

# Add project root to sys.path
BASE_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(BASE_DIR))

from src.config import ensure_directories, CORE_PROFILE_FILE, SKILLS_DIR, SESSIONS_DIR, DAILY_DIR
from src.db import init_db, get_active_facts, get_active_skills
from src.llm_client import MockLLMClient
from src.extractors.session_extractor import SessionExtractor
from src.consolidator.daily_consolidator import DailyConsolidator
from src.consolidator.skill_compiler import SkillCompiler

def run():
    print("=== Experienced LLMs: Full Pipeline Demo ===")
    init_db()

    # Step 1: Session Extraction
    print("\n--- Step 1: Extracting Session Facts ---")
    transcript_path = BASE_DIR / "examples" / "sample_transcript.txt"
    with open(transcript_path, "r", encoding="utf-8") as f:
        transcript = f.read()

    extraction_mock_response = """{
        "facts": [
            {
                "category": "user_preference",
                "scope": "global",
                "rule_statement": "ALWAYS execute npm and node commands inside WSL. Never call node directly from Windows host.",
                "context_reason": "User environment constraint and strict rule",
                "confidence": 1.0
            },
            {
                "category": "technical_decision",
                "scope": "experienced-llms",
                "rule_statement": "Use SQLite with PRAGMA foreign_keys = ON instead of PostgreSQL for state management.",
                "context_reason": "Optimized for single 8GB GPU local setups with zero server overhead",
                "confidence": 0.95
            },
            {
                "category": "mistake_correction",
                "scope": "experienced-llms",
                "rule_statement": "Omit ArtifactMetadata when calling write_to_file for normal workspace code files.",
                "context_reason": "Providing ArtifactMetadata triggers invalid_args artifact path enforcement",
                "confidence": 1.0
            }
        ]
    }"""

    extractor = SessionExtractor(MockLLMClient(extraction_mock_response))
    result = extractor.extract_from_text(
        transcript=transcript,
        session_id="demo_session_001",
        scope_default="experienced-llms"
    )
    print(f"Extracted {len(result.facts)} facts into {SESSIONS_DIR / 'demo_session_001.md'}:")
    for f in result.facts:
        print(f"  + [{f.category.value}] ({f.scope}): {f.rule_statement}")

    # Step 2: Daily Consolidation
    print("\n--- Step 2: Daily Memory Consolidation ---")
    daily_mock_response = """{
        "superseded_pairs": [],
        "daily_summary": "Enforced WSL execution rules, standardized SQLite database backend, and resolved tool argument constraints."
    }"""
    consolidator = DailyConsolidator(MockLLMClient(daily_mock_response))
    daily_file = consolidator.consolidate_date("2026-09-21")
    print(f"Daily log created: {daily_file}")

    # Step 3: Skill & Core Profile Compilation
    print("\n--- Step 3: Compiling JIT Skills & Core Profile ---")
    skill_mock_response = """{
        "skills_to_generate": [
            {
                "key": "wsl_node_runner",
                "role": "Ensures all node and npm scripts are executed inside WSL via bash.",
                "rules": [
                    "Wrap any npm/npx/node command with 'wsl -e bash -c ...'",
                    "Never run node.exe on Windows host",
                    "Handle /mnt/c path translation"
                ],
                "input_contract": "Windows path or npm command",
                "output_contract": "WSL-safe execution string"
            }
        ]
    }"""
    core_profile_mock = """- [Directive] NEVER run node/npm on Windows; always invoke inside WSL.
- [Directive] Omit ArtifactMetadata when writing workspace project files.
- [Architecture] Use local SQLite (PRAGMA foreign_keys = ON) for zero-overhead local state."""

    class DemoCompilerClient(MockLLMClient):
        def generate(self, system_prompt: str, user_prompt: str, json_mode: bool = False) -> str:
            if json_mode:
                return skill_mock_response
            return core_profile_mock

    compiler = SkillCompiler(DemoCompilerClient())
    skills = compiler.compile_all()
    print(f"Generated {len(skills)} auto-skills:")
    for s in skills:
        print(f"  * Key: {s.key}")
        print(f"    Path: {s.file_path}")

    # Step 4: Display Runtime Context for 7B Model
    print("\n--- Step 4: Runtime Context Ready for 7B Local LLM ---")
    with open(CORE_PROFILE_FILE, "r", encoding="utf-8") as f:
        print(f.read().strip())

    print("\n=== Demo Finished Successfully ===")

if __name__ == "__main__":
    run()
