import tempfile
import unittest
from pathlib import Path

from src.models import FactCategory, FactStatus, MemoryFact
from src.db import (
    BASELINE_VERSION,
    get_connection,
    init_db,
    insert_fact,
    get_pending_facts,
    get_active_facts,
    mark_facts_consolidated,
    record_consolidation_audit,
)
import src.config as config

class TestDatabaseSchemaAndHelpers(unittest.TestCase):
    def setUp(self):
        self.test_dir = tempfile.TemporaryDirectory()
        self.test_path = Path(self.test_dir.name)
        self.db_path = self.test_path / "memory.db"
        config.DB_PATH = self.db_path
        init_db(self.db_path)

    def tearDown(self):
        self.test_dir.cleanup()

    def test_database_initialization_has_canonical_tables(self):
        conn = get_connection(self.db_path)
        tables = conn.execute(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'"
        ).fetchall()
        table_names = set(r["name"] for r in tables)
        conn.close()

        expected_tables = {"schema_migrations", "sessions", "memory_facts", "consolidations"}
        self.assertEqual(table_names, expected_tables)

    def test_schema_migrations_baseline_recorded(self):
        conn = get_connection(self.db_path)
        rows = conn.execute("SELECT version, applied_at FROM schema_migrations").fetchall()
        conn.close()

        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]["version"], BASELINE_VERSION)
        self.assertIsNotNone(rows[0]["applied_at"])

    def test_wal_mode_and_foreign_keys(self):
        conn = get_connection(self.db_path)
        journal_mode = conn.execute("PRAGMA journal_mode").fetchone()[0]
        foreign_keys = conn.execute("PRAGMA foreign_keys").fetchone()[0]
        conn.close()

        self.assertEqual(journal_mode.lower(), "wal")
        self.assertEqual(foreign_keys, 1)

    def test_insert_fact_default_is_consolidated_zero(self):
        conn = get_connection(self.db_path)
        fact_id = insert_fact(
            MemoryFact(
                category=FactCategory.DEFENSIVE_HEURISTIC,
                rule_statement="Always verify .env is in .gitignore before commit",
                context_reason="Prevent credential leak",
                scope="global"
            ),
            conn=conn
        )
        row = conn.execute("SELECT is_consolidated, status FROM memory_facts WHERE id = ?", (fact_id,)).fetchone()
        conn.close()

        self.assertEqual(row["is_consolidated"], 0)
        self.assertEqual(row["status"], "active")

    def test_get_pending_and_active_facts(self):
        conn = get_connection(self.db_path)
        id1 = insert_fact(
            MemoryFact(
                category=FactCategory.DEFENSIVE_HEURISTIC,
                rule_statement="Rule 1: pending and active",
                is_consolidated=0,
                status=FactStatus.ACTIVE,
                scope="global"
            ),
            conn=conn
        )
        id2 = insert_fact(
            MemoryFact(
                category=FactCategory.USER_PREFERENCE,
                rule_statement="Rule 2: already consolidated and active",
                is_consolidated=1,
                status=FactStatus.ACTIVE,
                scope="global"
            ),
            conn=conn
        )
        id3 = insert_fact(
            MemoryFact(
                category=FactCategory.MISTAKE_CORRECTION,
                rule_statement="Rule 3: superseded rule",
                is_consolidated=0,
                status=FactStatus.SUPERSEDED,
                scope="global"
            ),
            conn=conn
        )
        id4 = insert_fact(
            MemoryFact(
                category=FactCategory.PROJECT_GOTCHA,
                rule_statement="Rule 4: project specific pending",
                is_consolidated=0,
                status=FactStatus.ACTIVE,
                scope="my-project"
            ),
            conn=conn
        )
        conn.close()

        # get_pending_facts: only is_consolidated = 0 AND active
        pending_all = get_pending_facts()
        pending_ids = [f.id for f in pending_all]
        self.assertIn(id1, pending_ids)
        self.assertNotIn(id2, pending_ids) # consolidated
        self.assertNotIn(id3, pending_ids) # superseded
        self.assertIn(id4, pending_ids)

        # get_pending_facts with scope filtering
        pending_scoped = get_pending_facts(scope="my-project")
        pending_scoped_ids = [f.id for f in pending_scoped]
        self.assertIn(id1, pending_scoped_ids) # global
        self.assertIn(id4, pending_scoped_ids) # my-project

        pending_other = get_pending_facts(scope="other-project")
        pending_other_ids = [f.id for f in pending_other]
        self.assertIn(id1, pending_other_ids) # global
        self.assertNotIn(id4, pending_other_ids) # my-project excluded

        # get_active_facts: all active regardless of is_consolidated
        active_all = get_active_facts()
        active_ids = [f.id for f in active_all]
        self.assertIn(id1, active_ids)
        self.assertIn(id2, active_ids)
        self.assertNotIn(id3, active_ids)
        self.assertIn(id4, active_ids)

    def test_mark_facts_consolidated(self):
        conn = get_connection(self.db_path)
        id1 = insert_fact(
            MemoryFact(
                category=FactCategory.TECHNICAL_DECISION,
                rule_statement="Direct SQLite queries over ORM",
                is_consolidated=0,
                status=FactStatus.ACTIVE
            ),
            conn=conn
        )
        id2 = insert_fact(
            MemoryFact(
                category=FactCategory.TECHNICAL_DECISION,
                rule_statement="Use WAL mode for concurrency",
                is_consolidated=0,
                status=FactStatus.ACTIVE
            ),
            conn=conn
        )
        conn.close()

        self.assertEqual(len(get_pending_facts()), 2)

        # Mark id1 as consolidated
        updated_count = mark_facts_consolidated([id1])
        self.assertEqual(updated_count, 1)

        pending_remaining = get_pending_facts()
        self.assertEqual(len(pending_remaining), 1)
        self.assertEqual(pending_remaining[0].id, id2)

        # Empty list should return 0 safely
        self.assertEqual(mark_facts_consolidated([]), 0)

    def test_record_consolidation_audit(self):
        audit_id = record_consolidation_audit(
            engine_type="cli_external_cmd",
            facts_processed=4,
            facts_superseded=1,
            summary="Deduplicated 3 rules, superseded 1 older directive"
        )
        self.assertIsNotNone(audit_id)

        conn = get_connection(self.db_path)
        row = conn.execute("SELECT * FROM consolidations WHERE id = ?", (audit_id,)).fetchone()
        conn.close()

        self.assertEqual(row["engine_type"], "cli_external_cmd")
        self.assertEqual(row["facts_processed"], 4)
        self.assertEqual(row["facts_superseded"], 1)
        self.assertEqual(row["summary_of_changes"], "Deduplicated 3 rules, superseded 1 older directive")
        self.assertIsNotNone(row["consolidated_at"])

if __name__ == "__main__":
    unittest.main()
