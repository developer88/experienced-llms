import json
import tempfile
import unittest
from pathlib import Path

import src.config as config
from src.db import init_db, get_connection, insert_fact, get_active_facts, get_pending_facts, BASELINE_VERSION
from src.models import MemoryFact, FactCategory, FactStatus
from src.llm_client import ExternalCommandLLMClient, MockLLMClient
from src.consolidator.interday_consolidator import (
    InterdayConsolidator,
    clean_markdown_experience,
    render_active_experience_markdown,
)

class TestConsolidatorEngine(unittest.TestCase):
    def setUp(self):
        self.test_dir = tempfile.TemporaryDirectory()
        self.test_path = Path(self.test_dir.name)

        config.DB_PATH = self.test_path / "memory.db"
        config.EXPERIENCED_HOME = self.test_path / "home"
        config.MASTER_EXPERIENCE_FILE = config.EXPERIENCED_HOME / "EXPERIENCE.md"
        config.ensure_directories()
        init_db(config.DB_PATH)

    def tearDown(self):
        self.test_dir.cleanup()

    def test_clean_markdown_experience_strips_comments_and_timestamps(self):
        dirty_md = """# Master AI Experience
<!-- Consolidated: 2026-10-15 02:00:00 (Deterministic fallback) -->
<!-- Internal note: test comment -->
Generated at: 2026-10-15T02:00:00Z

## 1. User Directives & Preferences
- (global): Always run npm in WSL *(Reason: User rule)*

<!-- Trailing comment -->
"""
        cleaned = clean_markdown_experience(dirty_md)
        self.assertNotIn("<!--", cleaned)
        self.assertNotIn("Consolidated:", cleaned)
        self.assertNotIn("Generated at:", cleaned)
        self.assertIn("# Master AI Experience", cleaned)
        self.assertIn("- (global): Always run npm in WSL", cleaned)

    def test_render_active_experience_markdown(self):
        conn = get_connection(config.DB_PATH)
        insert_fact(
            MemoryFact(category=FactCategory.USER_PREFERENCE, rule_statement="Always use npm in WSL.", scope="global"),
            conn=conn
        )
        insert_fact(
            MemoryFact(category=FactCategory.DEFENSIVE_HEURISTIC, rule_statement="Never call node from Windows directly.", scope="global"),
            conn=conn
        )
        conn.close()

        rendered = render_active_experience_markdown()
        self.assertIn("# Master AI Experience", rendered)
        self.assertIn("## 1. User Directives & Preferences", rendered)
        self.assertIn("Always use npm in WSL.", rendered)
        self.assertIn("## 2. Defensive Safeguards & Heuristics", rendered)
        self.assertIn("Never call node from Windows directly.", rendered)
        self.assertNotIn("<!--", rendered)

    def test_external_command_llm_client_execution(self):
        # Test piping through a simple python one-liner mock CLI
        mock_cli = 'python3 -c "import sys, json; data = sys.stdin.read(); sys.stdout.write(json.dumps({\'updated_experience_markdown\': \'# Mock Master\\n- test\', \'summary_of_changes\': \'Mock summary\', \'processed_pending_ids\': [], \'superseded_fact_ids\': []}))"'
        client = ExternalCommandLLMClient(command_str=mock_cli)
        response = client.generate("System prompt", "User prompt", json_mode=True)
        self.assertIn("Mock Master", response)
        data = json.loads(response)
        self.assertEqual(data["summary_of_changes"], "Mock summary")

    def test_external_command_llm_client_failure_handling(self):
        # Non-zero exit code
        failing_cli = 'python3 -c "import sys; sys.stderr.write(\'Fatal LLM CLI error\\n\'); sys.exit(2)"'
        client = ExternalCommandLLMClient(command_str=failing_cli)
        with self.assertRaises(RuntimeError) as ctx:
            client.generate("sys", "user")
        self.assertIn("Fatal LLM CLI error", str(ctx.exception))

    def test_build_consolidation_prompt(self):
        conn = get_connection(config.DB_PATH)
        f1 = insert_fact(MemoryFact(category=FactCategory.USER_PREFERENCE, rule_statement="Rule 1", scope="global"), conn=conn)
        # Mark f1 consolidated
        conn.execute("UPDATE memory_facts SET is_consolidated = 1 WHERE id = ?", (f1,))
        # f2 is pending
        f2 = insert_fact(MemoryFact(category=FactCategory.DEFENSIVE_HEURISTIC, rule_statement="Rule 2", scope="global"), conn=conn)
        conn.commit()
        conn.close()

        consolidator = InterdayConsolidator()
        sys_prompt, user_prompt, meta = consolidator.build_consolidation_prompt()

        self.assertIsNotNone(sys_prompt)
        self.assertIn("CANONICAL SECTIONS", sys_prompt)
        self.assertIn("Rule 1", user_prompt)
        self.assertIn("Rule 2", user_prompt)
        self.assertEqual(meta["active_facts_count"], 2)
        self.assertEqual(meta["pending_facts_count"], 1)
        self.assertEqual(meta["pending_fact_ids"], [f2])

    def test_apply_consolidation_result_updates_db_and_file(self):
        conn = get_connection(config.DB_PATH)
        f1 = insert_fact(MemoryFact(category=FactCategory.USER_PREFERENCE, rule_statement="Older rule to supersede"), conn=conn)
        f2 = insert_fact(MemoryFact(category=FactCategory.USER_PREFERENCE, rule_statement="New pending rule"), conn=conn)
        conn.close()

        payload = {
            "updated_experience_markdown": "# Master AI Experience\n\n## 1. User Directives & Preferences\n- New pending rule",
            "processed_pending_ids": [f2],
            "superseded_fact_ids": [f1],
            "summary_of_changes": "Superseded older rule and integrated new rule."
        }

        consolidator = InterdayConsolidator()
        path, summary = consolidator.apply_consolidation_result(json.dumps(payload), engine_type="test_engine")

        self.assertEqual(summary, "Superseded older rule and integrated new rule.")
        self.assertTrue(path.exists())

        # Verify SQLite states
        conn = get_connection(config.DB_PATH)
        row1 = conn.execute("SELECT status FROM memory_facts WHERE id = ?", (f1,)).fetchone()
        row2 = conn.execute("SELECT is_consolidated FROM memory_facts WHERE id = ?", (f2,)).fetchone()
        audits = conn.execute("SELECT * FROM consolidations").fetchall()
        conn.close()

        self.assertEqual(row1["status"], "superseded")
        self.assertEqual(row2["is_consolidated"], 1)
        self.assertEqual(len(audits), 1)
        self.assertEqual(audits[0]["engine_type"], "test_engine")
        self.assertEqual(audits[0]["facts_processed"], 1)
        self.assertEqual(audits[0]["facts_superseded"], 1)

    def test_consolidation_llm_failure_resilience_preserves_facts(self):
        conn = get_connection(config.DB_PATH)
        f1 = insert_fact(MemoryFact(category=FactCategory.USER_PREFERENCE, rule_statement="Unconsolidated fact"), conn=conn)
        conn.close()

        failing_client = ExternalCommandLLMClient('python3 -c "import sys; sys.exit(1)"')
        consolidator = InterdayConsolidator(failing_client)

        with self.assertRaises(RuntimeError):
            consolidator.consolidate()

        # Verify fact remains is_consolidated = 0 and active
        conn = get_connection(config.DB_PATH)
        row = conn.execute("SELECT status, is_consolidated FROM memory_facts WHERE id = ?", (f1,)).fetchone()
        audits = conn.execute("SELECT count(*) FROM consolidations").fetchone()[0]
        conn.close()

        self.assertEqual(row["status"], "active")
        self.assertEqual(row["is_consolidated"], 0)
        self.assertEqual(audits, 0)
        with open(config.MASTER_EXPERIENCE_FILE, "r", encoding="utf-8") as f:
            content = f.read()
        self.assertNotIn("Unconsolidated fact", content)

    def test_max_experience_words_configuration_and_override(self):
        conn = get_connection(config.DB_PATH)
        insert_fact(MemoryFact(category=FactCategory.USER_PREFERENCE, rule_statement="Sample rule"), conn=conn)
        conn.close()

        consolidator = InterdayConsolidator()

        # 1. Default (1200)
        sys_prompt, _, meta = consolidator.build_consolidation_prompt()
        self.assertEqual(meta["max_words"], 1200)
        self.assertIn("under 1200 words", sys_prompt)

        # 2. Config file override (600)
        config.CONFIG_FILE = self.test_path / "config.json"
        config.save_config({"max_experience_words": 600})
        sys_prompt_cfg, _, meta_cfg = consolidator.build_consolidation_prompt()
        self.assertEqual(meta_cfg["max_words"], 600)
        self.assertIn("under 600 words", sys_prompt_cfg)

        # 3. CLI argument override (400)
        sys_prompt_arg, _, meta_arg = consolidator.build_consolidation_prompt(max_words=400)
        self.assertEqual(meta_arg["max_words"], 400)
        self.assertIn("under 400 words", sys_prompt_arg)

        # 4. Render truncation
        rendered = render_active_experience_markdown(max_words=5)
        self.assertLessEqual(len(rendered.split()), 5)

    def test_consolidation_prompt_template_customization(self):
        from src.consolidator.prompt_template import get_consolidation_template, DEFAULT_CONSOLIDATION_TEMPLATE

        # 1. Default fallback
        tmpl_default = get_consolidation_template(home_dir=self.test_path)
        self.assertEqual(tmpl_default, DEFAULT_CONSOLIDATION_TEMPLATE)

        # 2. User home template file (~/.experienced-llms/consolidation_prompt.txt)
        user_prompt_file = config.EXPERIENCED_HOME / "consolidation_prompt.txt"
        user_prompt_file.write_text("CUSTOM USER PROMPT TEMPLATE {max_words}", encoding="utf-8")
        tmpl_user = get_consolidation_template(home_dir=config.EXPERIENCED_HOME)
        self.assertIn("CUSTOM USER PROMPT TEMPLATE", tmpl_user)

        # 3. Explicit custom_path override
        custom_file = self.test_path / "explicit_template.txt"
        custom_file.write_text("EXPLICIT OVERRIDE TEMPLATE {max_words}", encoding="utf-8")
        tmpl_explicit = get_consolidation_template(custom_path=custom_file, home_dir=config.EXPERIENCED_HOME)
        self.assertIn("EXPLICIT OVERRIDE TEMPLATE", tmpl_explicit)

        # 4. In InterdayConsolidator.build_consolidation_prompt with template_path
        conn = get_connection(config.DB_PATH)
        insert_fact(MemoryFact(category=FactCategory.USER_PREFERENCE, rule_statement="Sample"), conn=conn)
        conn.close()

        consolidator = InterdayConsolidator()
        sys_prompt, _, _ = consolidator.build_consolidation_prompt(template_path=custom_file, max_words=500)
        self.assertEqual(sys_prompt, "EXPLICIT OVERRIDE TEMPLATE 500")

if __name__ == "__main__":
    unittest.main()

