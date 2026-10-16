import os
import tempfile
import unittest
from datetime import datetime
from pathlib import Path

from src.models import FactCategory, FactStatus, MemoryFact
from src.db import init_db, get_active_facts, insert_fact, supersede_fact, get_connection
from src.llm_client import MockLLMClient
from src.extractors.session_extractor import SessionExtractor
from src.consolidator.skill_compiler import SkillCompiler
import src.config as config

class TestExperiencedLLMsPipeline(unittest.TestCase):
    def setUp(self):
        # Create a temporary directory for tests
        self.test_dir = tempfile.TemporaryDirectory()
        self.test_path = Path(self.test_dir.name)

        # Override config paths for isolation
        config.DB_PATH = self.test_path / "test_memory.db"
        config.EXPERIENCED_HOME = self.test_path / "home"
        config.MASTER_EXPERIENCE_FILE = config.EXPERIENCED_HOME / "EXPERIENCE.md"
        config.MEMORY_ROOT = config.EXPERIENCED_HOME
        config.SKILLS_DIR = self.test_path / "skills"
        config.CORE_PROFILE_FILE = config.EXPERIENCED_HOME / "core_profile.md"

        config.ensure_directories()
        init_db(config.DB_PATH)

    def tearDown(self):
        self.test_dir.cleanup()

    def test_database_initialization(self):
        conn = get_connection(config.DB_PATH)
        tables = conn.execute("SELECT name FROM sqlite_master WHERE type='table'").fetchall()
        table_names = [r["name"] for r in tables]
        conn.close()

        self.assertIn("schema_migrations", table_names)
        self.assertIn("sessions", table_names)
        self.assertIn("memory_facts", table_names)
        self.assertIn("consolidations", table_names)

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

        active_facts = get_active_facts(conn=get_connection(config.DB_PATH))
        self.assertEqual(len(active_facts), 2)
        self.assertEqual(active_facts[0].category, FactCategory.USER_PREFERENCE)
        self.assertEqual(active_facts[1].category, FactCategory.MISTAKE_CORRECTION)

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
        self.assertIn("experienced-llms log", orch_prompt)
        self.assertIn("Experienced LLM Operating Protocol", orch_prompt)

        worker_prompt = build_worker_prompt()
        self.assertIn("Strict execution rule", worker_prompt)

    def test_intraday_deterministic_logging(self):
        from src.intraday import log_intraday
        config.ensure_directories()

        fact = log_intraday(
            rule_statement="Never hardcode credentials in source code.",
            category="user_preference",
            scope="security",
            context_reason="Leak prevention",
            session_id="session_intraday_test"
        )

        self.assertIsNotNone(fact.id)
        self.assertEqual(fact.category, FactCategory.USER_PREFERENCE)
        self.assertEqual(fact.is_consolidated, 0)

        # Check SQLite record has is_consolidated = 0
        conn = get_connection(config.DB_PATH)
        row = conn.execute("SELECT is_consolidated, status, rule_statement FROM memory_facts WHERE id = ?", (fact.id,)).fetchone()
        conn.close()

        self.assertEqual(row["is_consolidated"], 0)
        self.assertEqual(row["status"], "active")
        self.assertEqual(row["rule_statement"], "Never hardcode credentials in source code.")

        # Zero raw markdown files created
        md_files = [p for p in self.test_path.rglob("*.md") if p != config.MASTER_EXPERIENCE_FILE]
        self.assertEqual(len(md_files), 0)

    def test_interday_consolidation(self):
        from src.consolidator.interday_consolidator import InterdayConsolidator
        config.ensure_directories()

        # Seed an intraday log
        from src.intraday import log_intraday
        log_intraday(
            rule_statement="Always run npm inside WSL.",
            category="user_preference",
            scope="global",
            session_id="session_1"
        )

        mock_json = """{
            "updated_experience_markdown": "# Master AI Experience\\n\\n## 1. User Directives\\n- Always run npm inside WSL.",
            "superseded_facts": [],
            "summary_of_changes": "Added WSL execution rule."
        }"""
        client = MockLLMClient(response_text=mock_json)
        consolidator = InterdayConsolidator(client)

        path, summary = consolidator.consolidate("2026-09-26")
        self.assertTrue(path.exists())
        self.assertEqual(summary, "Added WSL execution rule.")

        with open(path, "r", encoding="utf-8") as f:
            saved_md = f.read()
        self.assertIn("# Master AI Experience", saved_md)
        self.assertIn("Always run npm inside WSL", saved_md)

    def test_config_save_load(self):
        config.CONFIG_FILE = self.test_path / "config.json"
        test_cfg = {
            "provider": "claude",
            "claude_api_key": "sk-ant-test1234",
            "claude_model": "claude-3-5-sonnet-latest"
        }
        config.save_config(test_cfg)
        loaded = config.load_config()
        self.assertEqual(loaded["provider"], "claude")
        self.assertEqual(loaded["claude_model"], "claude-3-5-sonnet-latest")

    def test_agent_integrations(self):
        from src.agent_integrations import (
            install_antigravity, install_claude, install_copilot,
            install_cursor, install_pi_generic
        )
        ws = self.test_path / "workspace"
        ws.mkdir(parents=True, exist_ok=True)

        # Antigravity
        ok, msg = install_antigravity(self.test_path / "antigravity_skill")
        self.assertTrue(ok)
        self.assertTrue((self.test_path / "antigravity_skill" / "SKILL.md").exists())

        # Claude
        ok, msg = install_claude(ws)
        self.assertTrue(ok)
        self.assertTrue((ws / "CLAUDE.md").exists())

        # Copilot
        ok, msg = install_copilot(ws)
        self.assertTrue(ok)
        self.assertTrue((ws / ".github" / "copilot-instructions.md").exists())

        # Cursor
        ok, msg = install_cursor(ws)
        self.assertTrue(ok)
        self.assertTrue((ws / ".cursorrules").exists())

        # Pi / Generic AGENTS.md
        ok, msg = install_pi_generic(ws)
        self.assertTrue(ok)
        self.assertTrue((ws / "AGENTS.md").exists())

if __name__ == "__main__":
    unittest.main()
