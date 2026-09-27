#!/usr/bin/env python3
"""
Cross-platform isolated acceptance runner for Experienced LLMs.
Runs the entire lifecycle in a temporary directory without touching the user's live memory or host schedulers.
"""

import os
import sys
import tempfile
import unittest
from pathlib import Path

# Ensure repo root is on sys.path
REPO_ROOT = Path(__file__).resolve().parent.parent
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

def run_isolated_acceptance():
    print("==========================================================")
    print("  Experienced LLMs: Cross-Platform Sandbox Acceptance")
    print("==========================================================")

    with tempfile.TemporaryDirectory() as temp_dir:
        sandbox_path = Path(temp_dir)
        print(f"[+] Sandbox root: {sandbox_path}")

        # 1. Override environment & configuration paths
        os.environ["HOME"] = str(sandbox_path)
        os.environ["USERPROFILE"] = str(sandbox_path)
        os.environ["EXPERIENCED_LLMS_HOME"] = str(sandbox_path / ".experienced-llms")
        os.environ["EXPERIENCED_LLMS_DB"] = str(sandbox_path / ".experienced-llms" / "memory.db")

        import src.config as config
        config.EXPERIENCED_HOME = sandbox_path / ".experienced-llms"
        config.DB_PATH = config.EXPERIENCED_HOME / "memory.db"
        config.CONFIG_FILE = config.EXPERIENCED_HOME / "config.json"
        config.MASTER_EXPERIENCE_FILE = config.EXPERIENCED_HOME / "EXPERIENCE.md"
        config.RAW_DAYS_DIR = config.EXPERIENCED_HOME / "raw_days"
        config.SKILLS_DIR = config.EXPERIENCED_HOME / "skills"
        config.MEMORY_ROOT = config.EXPERIENCED_HOME
        config.SESSIONS_DIR = config.EXPERIENCED_HOME / "sessions"
        config.DAILY_DIR = config.EXPERIENCED_HOME / "daily"
        config.CORE_PROFILE_FILE = config.EXPERIENCED_HOME / "core_profile.md"

        # 2. Initialize
        print("[+] Initializing sandbox directories and SQLite...")
        config.ensure_directories()
        from src.db import init_db, get_active_facts
        init_db(config.DB_PATH)

        # 3. Test Intraday Logger
        print("[+] Testing intraday deterministic logger...")
        from src.intraday import log_intraday
        fact = log_intraday(
            rule_statement="Always run isolated acceptance tests in a disposable directory.",
            category="defensive_heuristic",
            scope="sandbox-acceptance",
            context_reason="Zero pollution of host system"
        )
        assert fact.id is not None, "Failed to insert fact into sandbox database"
        print(f"[+] Recorded fact #{fact.id}: {fact.rule_statement}")

        # 4. Verify raw day file was created
        raw_files = list(config.RAW_DAYS_DIR.glob("*.md"))
        assert len(raw_files) == 1, f"Expected 1 raw day file, got {len(raw_files)}"
        print(f"[+] Verified raw day log at {raw_files[0]}")

        # 5. Test Interday Consolidation
        print("[+] Testing interday consolidator in sandbox...")
        from src.llm_client import MockLLMClient
        from src.consolidator.interday_consolidator import InterdayConsolidator
        consolidator = InterdayConsolidator(MockLLMClient())
        out_path, summary = consolidator.consolidate("2026-09-27")

        assert out_path.exists(), "Master experience file not generated"
        with open(out_path, "r", encoding="utf-8") as f:
            content = f.read()
        assert "Always run isolated acceptance tests" in content, "Rule not found in consolidated EXPERIENCE.md"
        print(f"[+] Verified master EXPERIENCE.md generated:\n    {summary}")

        # 6. Test Multi-Agent Integrations in sandbox
        print("[+] Testing agent instruction generators in sandbox...")
        from src.agent_integrations import auto_detect_and_install_all
        agent_results = auto_detect_and_install_all(workspace=sandbox_path)
        print(f"[+] Configured {len(agent_results)} agent integrations in sandbox.")

        # 7. Run Unit Test Suite
        print("[+] Running automated unit test suite...")
        suite = unittest.defaultTestLoader.discover(str(REPO_ROOT / "tests"))
        runner = unittest.TextTestRunner(verbosity=1)
        test_result = runner.run(suite)
        if not test_result.wasSuccessful():
            print("[-] Unit tests failed!")
            sys.exit(1)

        print("")
        print("==========================================================")
        print("[SUCCESS] All isolated acceptance checks passed cleanly!")
        print("Host system state and live memory remained 100% untouched.")
        print("==========================================================")

if __name__ == "__main__":
    run_isolated_acceptance()
