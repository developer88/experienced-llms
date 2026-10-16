import tempfile
import unittest
import sqlite3
from pathlib import Path

import src.config as config
from src.db import get_connection, BASELINE_VERSION
from src.migration_runner import (
    get_applied_migrations,
    get_available_migrations,
    get_pending_migrations,
    apply_migration,
    run_migrations,
)

class TestMigrationRunner(unittest.TestCase):
    def setUp(self):
        self.test_dir = tempfile.TemporaryDirectory()
        self.test_path = Path(self.test_dir.name)
        self.db_path = self.test_path / "memory.db"
        self.migrations_dir = self.test_path / "migrations"
        self.migrations_dir.mkdir(parents=True, exist_ok=True)
        
        self.orig_db_path = config.DB_PATH
        config.DB_PATH = self.db_path

    def tearDown(self):
        config.DB_PATH = self.orig_db_path
        self.test_dir.cleanup()

    def test_fresh_install_initializes_db_and_records_baseline(self):
        self.assertFalse(self.db_path.exists())
        is_fresh, applied = run_migrations(migrations_dir=self.migrations_dir)

        self.assertTrue(is_fresh)
        self.assertEqual(applied, [])
        self.assertTrue(self.db_path.exists())

        conn = get_connection(self.db_path)
        applied_in_db = get_applied_migrations(conn)
        conn.close()

        self.assertEqual(applied_in_db, [BASELINE_VERSION])

    def test_get_available_migrations_sorting(self):
        (self.migrations_dir / "003_third.py").write_text("def migrate(ctx): pass\n", encoding="utf-8")
        (self.migrations_dir / "002_second.sql").write_text("SELECT 1;\n", encoding="utf-8")
        (self.migrations_dir / "README.md").write_text("Not a migration\n", encoding="utf-8")
        (self.migrations_dir / "ignore_me.txt").write_text("Ignore\n", encoding="utf-8")

        available = get_available_migrations(self.migrations_dir)
        versions = [v for v, p in available]
        self.assertEqual(versions, ["002_second", "003_third"])

    def test_get_pending_migrations_filters_applied(self):
        run_migrations(migrations_dir=self.migrations_dir)
        conn = get_connection(self.db_path)

        (self.migrations_dir / "002_add_field.py").write_text("def migrate(ctx): pass\n", encoding="utf-8")
        pending = get_pending_migrations(conn, self.migrations_dir)
        self.assertEqual(len(pending), 1)
        self.assertEqual(pending[0][0], "002_add_field")

        # Manually record 002_add_field
        conn.execute("INSERT INTO schema_migrations (version) VALUES (?)", ("002_add_field",))
        conn.commit()

        pending_after = get_pending_migrations(conn, self.migrations_dir)
        self.assertEqual(pending_after, [])
        conn.close()

    def test_apply_python_migration_full_stack(self):
        run_migrations(migrations_dir=self.migrations_dir)
        conn = get_connection(self.db_path)

        migration_script = self.migrations_dir / "002_test_python.py"
        migration_script.write_text(
            """
def migrate(context):
    conn = context["conn"]
    conn.execute("CREATE TABLE test_full_stack (id INTEGER PRIMARY KEY, note TEXT)")
    conn.commit()
""",
            encoding="utf-8"
        )

        success = apply_migration("002_test_python", migration_script, conn)
        self.assertTrue(success)

        # Verify table created
        table = conn.execute(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='test_full_stack'"
        ).fetchone()
        self.assertIsNotNone(table)

        # Verify migration recorded
        applied = get_applied_migrations(conn)
        self.assertIn("002_test_python", applied)
        conn.close()

    def test_apply_sql_migration(self):
        run_migrations(migrations_dir=self.migrations_dir)
        conn = get_connection(self.db_path)

        migration_script = self.migrations_dir / "003_test_sql.sql"
        migration_script.write_text(
            "CREATE TABLE test_sql_table (id INTEGER PRIMARY KEY, title TEXT);\n",
            encoding="utf-8"
        )

        success = apply_migration("003_test_sql", migration_script, conn)
        self.assertTrue(success)

        table = conn.execute(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='test_sql_table'"
        ).fetchone()
        self.assertIsNotNone(table)

        applied = get_applied_migrations(conn)
        self.assertIn("003_test_sql", applied)
        conn.close()

    def test_run_migrations_end_to_end_sequential(self):
        # Create fresh install
        is_fresh, applied = run_migrations(migrations_dir=self.migrations_dir)
        self.assertTrue(is_fresh)
        self.assertEqual(applied, [])

        # Add 2 migrations
        (self.migrations_dir / "002_step_one.sql").write_text(
            "CREATE TABLE step_one (id INTEGER PRIMARY KEY);\n",
            encoding="utf-8"
        )
        (self.migrations_dir / "003_step_two.sql").write_text(
            "CREATE TABLE step_two (id INTEGER PRIMARY KEY);\n",
            encoding="utf-8"
        )

        conn = get_connection(self.db_path)
        is_fresh_2, applied_2 = run_migrations(conn=conn, migrations_dir=self.migrations_dir)
        conn.close()

        self.assertFalse(is_fresh_2)
        self.assertEqual(applied_2, ["002_step_one", "003_step_two"])

        # Running again is idempotent
        conn = get_connection(self.db_path)
        is_fresh_3, applied_3 = run_migrations(conn=conn, migrations_dir=self.migrations_dir)
        self.assertFalse(is_fresh_3)
        self.assertEqual(applied_3, [])
        conn.close()

if __name__ == "__main__":
    unittest.main()
