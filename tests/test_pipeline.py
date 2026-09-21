import os
import tempfile
import unittest
from pathlib import Path

from src.models import FactCategory, FactStatus, MemoryFact
from src.db import init_db, get_active_facts, insert_fact, supersede_fact, get_connection
from src.llm_client import MockLLMClient
from src.extractors.session_extractor import SessionExtractor
from src.consolidator.daily_consolidator import DailyConsolidator
from src.consolidator.skill_compiler import SkillCompiler
import src.config as config

class TestExperiencedLLMsPipeline(unittest.TestCase):
    def setUp(self):
        # Create a temporary directory for tests
        self.test_dir = tempfile.TemporaryDirectory()
        self.test_path = Path(self.test_dir.name)

        # Override config paths for isolation
        config.DB_PATH = self.test_path / "test_memory.db"
        config.MEMORY_ROOT = self.test_path / "memory"
        config.SESSIONS_DIR = config.MEMORY_ROOT / "sessions"
        config.DAILY_DIR = config.MEMORY_ROOT / "daily"
        config.SKILLS_DIR = self.test_path / "skills"
        config.CORE_PROFILE_FILE = config.MEMORY_ROOT / "core_profile.md"

        init_db(config.DB_PATH)

    def tearDown(self):
        self.test_dir.cleanup()

    def test_database_initialization(self):
        conn = get_connection(config.DB_PATH)
        tables = conn.execute("SELECT name FROM sqlite_master WHERE type='table'").fetchall()
        table_names = [r["name"] for r in tables]
        conn.close()

        self.assertIn("sessions", table_names)
        self.assertIn("memory_facts", table_names)
        self.assertIn("skills", table_names)
        self.assertIn("daily_consolidations", table_names)

    def test_session_extraction(self):
        mock_response = """{
            "facts": [
                {
                    "category": "user_preference",
                    "scope": "global",
                    "rule_statement": "Always run npm inside WSL, never on Windows directly.",
                    "context_reason": "User enforced WSL environment rule",
                    "confidence": 1.0
                },
                {
                    "category": "mistake_correction",
                    "scope": "experienced-llms",
                    "rule_statement": "Omit ArtifactMetadata when calling write_to_file for workspace files.",
                    "context_reason": "Tool raised invalid_args artifact path error",
                    "confidence": 0.95
                }
            ]
        }"""
        mock_client = MockLLMClient(response_text=mock_response)
        extractor = SessionExtractor(mock_client)

        sample_transcript = "User: ALWAYS use npm in WSL. Model: Understood."
        result = extractor.extract_from_text(
            transcript=sample_transcript,
            session_id="2026-09-21_session_01",
            scope_default="experienced-llms"
        )

        self.assertEqual(len(result.facts), 2)
        self.assertTrue((config.SESSIONS_DIR / "2026-09-21_session_01.md").exists())

        active_facts = get_active_facts(conn=get_connection(config.DB_PATH))
        self.assertEqual(len(active_facts), 2)
        self.assertEqual(active_facts[0].category, FactCategory.USER_PREFERENCE)
        self.assertEqual(active_facts[1].category, FactCategory.MISTAKE_CORRECTION)

    def test_superseding_and_daily_consolidation(self):
        conn = get_connection(config.DB_PATH)
        from src.db import insert_session
        insert_session("session_old", conn=conn)
        insert_session("session_new", conn=conn)
        fact1_id = insert_fact(
            MemoryFact(
                category=FactCategory.TECHNICAL_DECISION,
                rule_statement="Use pnpm for package management.",
                context_reason="Initial choice",
                session_id="session_old"
            ),
            conn=conn
        )
        fact2_id = insert_fact(
            MemoryFact(
                category=FactCategory.TECHNICAL_DECISION,
                rule_statement="Migrated from pnpm to npm in WSL.",
                context_reason="User preference changed",
                session_id="session_new"
            ),
            conn=conn
        )
        conn.close()

        # Mock reconciler response saying fact2 supersedes fact1
        reconciliation_response = f"""{{
            "superseded_pairs": [
                {{"old_fact_id": {fact1_id}, "new_fact_id": {fact2_id}, "reason": "Switched package managers"}}
            ],
            "daily_summary": "Standardized on npm in WSL across all node tasks."
        }}"""
        mock_client = MockLLMClient(response_text=reconciliation_response)
        consolidator = DailyConsolidator(mock_client)

        date_str = "2026-09-21"
        daily_path = consolidator.consolidate_date(date_str)

        self.assertTrue(daily_path.exists())
        with open(daily_path, "r", encoding="utf-8") as f:
            content = f.read()

        self.assertIn("Standardized on npm in WSL", content)
        self.assertIn("Migrated from pnpm to npm in WSL", content)
        self.assertIn("Superseded / Deprecated Directives", content)

        # Ensure old fact is not active
        active_facts = get_active_facts(conn=get_connection(config.DB_PATH))
        self.assertEqual(len(active_facts), 1)
        self.assertEqual(active_facts[0].id, fact2_id)

    def test_skill_compiler_and_core_profile(self):
        conn = get_connection(config.DB_PATH)
        insert_fact(
            MemoryFact(
                category=FactCategory.MISTAKE_CORRECTION,
                rule_statement="Always execute npm commands inside WSL via wsl -e bash.",
                context_reason="Windows node path failure",
                scope="global"
            ),
            conn=conn
        )
        conn.close()

        core_profile_mock = "- Strictly run npm in WSL.\n- Never call node on Windows host."
        skill_mock = """{
            "skills_to_generate": [
                {
                    "key": "wsl_npm_runner",
                    "role": "Wraps node/npm commands to execute in WSL",
                    "rules": [
                        "Prefix all npm calls with wsl",
                        "Never run node.exe on Windows"
                    ],
                    "input_contract": "npm script command",
                    "output_contract": "wsl bash command string"
                }
            ]
        }"""

        class MultiMockClient(MockLLMClient):
            def generate(self, system_prompt: str, user_prompt: str, json_mode: bool = False) -> str:
                if json_mode:
                    return skill_mock
                return core_profile_mock

        client = MultiMockClient()
        compiler = SkillCompiler(client)
        skills = compiler.compile_all()

        self.assertEqual(len(skills), 1)
        skill = skills[0]
        self.assertEqual(skill.key, "auto_wsl_npm_runner")
        self.assertTrue(Path(skill.file_path).exists())

        # Check core_profile.md
        self.assertTrue(config.CORE_PROFILE_FILE.exists())
        with open(config.CORE_PROFILE_FILE, "r", encoding="utf-8") as f:
            core_content = f.read()
        self.assertIn("Strictly run npm in WSL", core_content)

        # Check ACTIVE_CONTEXT.md was exported
        self.assertTrue((config.MEMORY_ROOT / "ACTIVE_CONTEXT.md").exists())

    def test_explicit_session_learning_tag_extraction(self):
        transcript_with_tag = """
        User: Always prefix npm with wsl.
        Model: Understood.
        <session_learning>
        - [user_preference] (global): Always run npm in WSL | Windows path resolution failure
        </session_learning>
        """
        # Mock client should not even need to be called if tags are parsed
        client = MockLLMClient(response_text="{}")
        extractor = SessionExtractor(client)
        res = extractor.extract_from_text(transcript_with_tag, session_id="test_tag_session")

        self.assertEqual(len(res.facts), 1)
        self.assertEqual(res.facts[0].rule_statement, "Always run npm in WSL")
        self.assertEqual(res.facts[0].category, FactCategory.USER_PREFERENCE)
        self.assertEqual(len(client.calls), 0) # Zero LLM calls needed!

    def test_prompt_builder(self):
        from src.prompt_builder import build_orchestrator_prompt, build_worker_prompt
        conn = get_connection(config.DB_PATH)
        insert_fact(
            MemoryFact(
                category=FactCategory.TECHNICAL_DECISION,
                rule_statement="Standardize on SQLite with WAL mode.",
                scope="global"
            ),
            conn=conn
        )
        conn.close()

        orch_prompt = build_orchestrator_prompt()
        self.assertIn("Standardize on SQLite with WAL mode", orch_prompt)
        self.assertIn("<session_learning>", orch_prompt)
        self.assertIn("Experienced LLM Operating Protocol", orch_prompt)

        worker_prompt = build_worker_prompt()
        self.assertIn("Strict execution rule", worker_prompt)

if __name__ == "__main__":
    unittest.main()
