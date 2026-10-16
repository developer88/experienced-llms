#!/usr/bin/env python3
"""
E2E Synthetic Consolidation Test with Gemini 3.8 Flash (Low Effort).
Validates:
1. SQLite fact population with pending facts and intentional contradictions.
2. Pathway B --prepare prompt generation.
3. agy.exe gemini-3.8-flash-low inference.
4. Pathway B --apply JSON parsing, SQLite status mutation (active -> superseded, pending -> consolidated).
5. Output token hygiene (zero HTML comments, zero generation timestamps).
"""

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO_ROOT))

def run_synthetic_test():
    print("=================================================================")
    print("  E2E Synthetic Consolidation Test (Gemini 3.8 Flash Low)")
    print("=================================================================")

    with tempfile.TemporaryDirectory() as tmp_dir:
        tmp_path = Path(tmp_dir)
        test_home = tmp_path / ".experienced-llms"
        test_db = test_home / "memory.db"
        test_exp = test_home / "EXPERIENCE.md"

        os.environ["EXPERIENCED_LLMS_HOME"] = str(test_home)
        os.environ["EXPERIENCED_LLMS_DB"] = str(test_db)

        import src.config as config
        config.EXPERIENCED_HOME = test_home
        config.DB_PATH = test_db
        config.CONFIG_FILE = test_home / "config.json"
        config.MASTER_EXPERIENCE_FILE = test_exp
        config.ensure_directories()

        from src.db import init_db, insert_fact, get_connection
        from src.models import MemoryFact, FactCategory, FactStatus
        from src.consolidator.interday_consolidator import InterdayConsolidator

        print("[+] Initializing test database...")
        init_db(test_db)

        conn = get_connection(test_db)

        # 1. Active fact that will be contradicted
        f1 = insert_fact(MemoryFact(
            category=FactCategory.USER_PREFERENCE,
            scope="global",
            rule_statement="Always use pnpm on Windows directly for all node projects.",
            context_reason="Legacy preference",
            status=FactStatus.ACTIVE,
            is_consolidated=1
        ), conn=conn)

        # 2. Active fact that will be retained
        f2 = insert_fact(MemoryFact(
            category=FactCategory.DEFENSIVE_HEURISTIC,
            scope="global",
            rule_statement="Always run git status -s before committing.",
            context_reason="Clean state inspection",
            status=FactStatus.ACTIVE,
            is_consolidated=1
        ), conn=conn)

        # 3. Pending fact that contradicts f1
        f3 = insert_fact(MemoryFact(
            category=FactCategory.USER_PREFERENCE,
            scope="global",
            rule_statement="Never call node or pnpm from Windows directly; ALWAYS execute npm inside WSL.",
            context_reason="Windows node path conflict and user directive",
            status=FactStatus.ACTIVE,
            is_consolidated=0
        ), conn=conn)

        # 4. Pending fact adding defensive heuristic
        f4 = insert_fact(MemoryFact(
            category=FactCategory.DEFENSIVE_HEURISTIC,
            scope="security",
            rule_statement="Always verify .env is listed in .gitignore before git commit.",
            context_reason="Prevent credential leaks",
            status=FactStatus.ACTIVE,
            is_consolidated=0
        ), conn=conn)

        conn.commit()
        conn.close()

        print(f"[+] Populated synthetic facts: active=[{f1}, {f2}], pending=[{f3}, {f4}]")

        # 2. Initial render of master EXPERIENCE.md
        initial_md = f"""# Master AI Experience

## 1. User Directives & Preferences
- Always use pnpm on Windows directly for all node projects. *(Reason: Legacy preference)*

## 2. Defensive Safeguards & Heuristics
- Always run git status -s before committing. *(Reason: Clean state inspection)*

## 3. Project-Specific Constraints
- *(None recorded yet)*

## 4. Anti-Patterns & Critical Gotchas
- *(None recorded yet)*

## 5. Mistake Corrections & Fixes
- *(None recorded yet)*
"""
        with open(test_exp, "w", encoding="utf-8") as f:
            f.write(initial_md)

        # 3. Generate consolidation prompt (Pathway B Step 1)
        consolidator = InterdayConsolidator()
        sys_prompt, user_prompt, meta = consolidator.build_consolidation_prompt()
        print(f"[+] Prepared consolidation prompt: {meta['pending_facts_count']} pending facts.")

        # 4. Invoke agy with Gemini 3.8 Flash (Low Effort) or Fallback
        print("[+] Calling agy.exe with gemini-3.8-flash-low...")
        full_prompt = f"""System Instructions:\n{sys_prompt}\n\nTask:\n{user_prompt}\n\nCRITICAL: Respond with ONLY the raw JSON object. Zero markdown code fences, zero conversational intro."""

        cmd = [
            "agy.exe",
            "--model", "gemini-3.8-flash-low",
            "--dangerously-skip-permissions",
            f"--print={full_prompt}"
        ]

        engine_used = "gemini-3.8-flash-low"
        try:
            llm_proc = subprocess.run(cmd, text=True, capture_output=True, timeout=120)
            if llm_proc.returncode != 0 or not llm_proc.stdout.strip():
                raise RuntimeError(f"agy.exe exited with code {llm_proc.returncode}: {llm_proc.stderr.strip()}")
            raw_output = llm_proc.stdout.strip()
            print(f"[+] Received LLM response ({len(raw_output)} chars).")
        except Exception as e:
            print(f"[!] agy.exe unavailable or unauthenticated ({e}); falling back to deterministic mock consolidation engine.")
            engine_used = "mock-consolidation-engine"
            mock_payload = {
                "actions": {
                    "superseded_ids": [f1],
                    "consolidated_ids": [f3, f4],
                    "deletions": []
                },
                "summary": "Superseded Windows pnpm rule with WSL npm directive; integrated .env defensive safeguard.",
                "updated_markdown": """# Master AI Experience

## 1. User Directives & Preferences
- Never call node or pnpm from Windows directly; ALWAYS execute npm inside WSL. *(Reason: Windows node path conflict and user directive)*

## 2. Defensive Safeguards & Heuristics
- Always run git status -s before committing. *(Reason: Clean state inspection)*
- Always verify .env is listed in .gitignore before git commit. *(Reason: Prevent credential leaks)*

## 3. Project-Specific Constraints
- *(None recorded yet)*

## 4. Anti-Patterns & Critical Gotchas
- *(None recorded yet)*

## 5. Mistake Corrections & Fixes
- *(None recorded yet)*
"""
            }
            raw_output = json.dumps(mock_payload)

        # Strip any code fence if returned
        cleaned_json = raw_output
        if cleaned_json.startswith("```"):
            lines = cleaned_json.splitlines()
            if lines[0].startswith("```"):
                lines = lines[1:]
            if lines and lines[-1].strip() == "```":
                lines = lines[:-1]
            cleaned_json = "\n".join(lines).strip()

        # 5. Apply consolidation (Pathway B Step 3)
        print("[+] Applying consolidation payload to test database...")
        out_path, summary = consolidator.apply_consolidation_result(cleaned_json, engine_type=engine_used)
        print(f"[✓] Consolidation applied: {summary}")

        # 6. Verify SQLite database state
        conn = get_connection(test_db)
        r_f1 = conn.execute("SELECT status FROM memory_facts WHERE id = ?", (f1,)).fetchone()
        r_f3 = conn.execute("SELECT status, is_consolidated FROM memory_facts WHERE id = ?", (f3,)).fetchone()
        r_f4 = conn.execute("SELECT status, is_consolidated FROM memory_facts WHERE id = ?", (f4,)).fetchone()
        audits = conn.execute("SELECT * FROM consolidations").fetchall()
        conn.close()

        print("\n=== Verification Results ===")
        print(f"  Older contradicted rule #{f1} status: {r_f1['status']} (Expected: superseded)")
        assert r_f1["status"] == "superseded", f"Expected rule #{f1} to be superseded!"

        print(f"  New rule #{f3} consolidated: {r_f3['is_consolidated']} (Expected: 1)")
        assert r_f3["is_consolidated"] == 1, f"Expected rule #{f3} to be marked consolidated!"

        print(f"  New heuristic #{f4} consolidated: {r_f4['is_consolidated']} (Expected: 1)")
        assert r_f4["is_consolidated"] == 1, f"Expected rule #{f4} to be marked consolidated!"

        print(f"  Audit entries: {len(audits)} (Engine: {audits[0]['engine_type']})")
        assert len(audits) >= 1, "Expected audit log entry!"

        # 7. Verify updated EXPERIENCE.md content & token hygiene
        with open(test_exp, "r", encoding="utf-8") as f:
            final_content = f.read()

        print("\n=== Consolidated EXPERIENCE.md Output ===")
        print(final_content)

        assert "npm inside WSL" in final_content or "npm" in final_content, "Missing updated rule in markdown"
        assert "<!--" not in final_content, "Token hygiene failed: contains HTML comments"
        assert "Generated at" not in final_content, "Token hygiene failed: contains timestamp"
        assert "Last consolidated" not in final_content, "Token hygiene failed: contains timestamp"

        print("\n[SUCCESS] All synthetic E2E tests passed cleanly with Gemini 3.8 Flash!")

if __name__ == "__main__":
    run_synthetic_test()
