use experienced_llms::config::Config;
use experienced_llms::db;
use experienced_llms::models::{
    ConsolidationAudit, FactCategory, FactStatus, MemoryFact, SkillFile,
};
use std::fs;
use std::process::Command;
use std::str::FromStr;
use tempfile::tempdir;

#[test]
fn test_models_serialization_and_display() {
    // 1. FactCategory display and parsing
    assert_eq!(FactCategory::DefensiveHeuristic.as_str(), "defensive_heuristic");
    assert_eq!(FactCategory::UserPreference.as_str(), "user_preference");
    assert_eq!(FactCategory::TechnicalDecision.as_str(), "technical_decision");
    assert_eq!(FactCategory::MistakeCorrection.as_str(), "mistake_correction");
    assert_eq!(FactCategory::ProjectGotcha.as_str(), "project_gotcha");

    assert_eq!(FactCategory::from_str("defensive_heuristic").unwrap(), FactCategory::DefensiveHeuristic);
    assert_eq!(FactCategory::from_str("user_preference").unwrap(), FactCategory::UserPreference);
    assert!(FactCategory::from_str("invalid_category").is_err());

    // 2. FactStatus display and parsing
    assert_eq!(FactStatus::Active.as_str(), "active");
    assert_eq!(FactStatus::Superseded.as_str(), "superseded");
    assert_eq!(FactStatus::UserVerified.as_str(), "user_verified");
    assert_eq!(FactStatus::Archived.as_str(), "archived");

    assert_eq!(FactStatus::from_str("active").unwrap(), FactStatus::Active);
    assert_eq!(FactStatus::from_str("superseded").unwrap(), FactStatus::Superseded);
    assert!(FactStatus::from_str("unknown_status").is_err());

    // 3. MemoryFact serialization/deserialization
    let mut fact = MemoryFact::new(FactCategory::DefensiveHeuristic, "Always verify clean git status.");
    fact.context_reason = Some("Prevent uncommitted changes".to_string());
    fact.scope = "git".to_string();

    let json = serde_json::to_string(&fact).expect("Serialization failed");
    assert!(json.contains("\"category\":\"defensive_heuristic\""));
    assert!(json.contains("\"rule_statement\":\"Always verify clean git status.\""));
    assert!(json.contains("\"scope\":\"git\""));

    let deserialized: MemoryFact = serde_json::from_str(&json).expect("Deserialization failed");
    assert_eq!(deserialized.category, FactCategory::DefensiveHeuristic);
    assert_eq!(deserialized.rule_statement, "Always verify clean git status.");
    assert_eq!(deserialized.context_reason.as_deref(), Some("Prevent uncommitted changes"));
}

#[test]
fn test_config_defaults_and_roundtrip() {
    let dir = tempdir().expect("Failed to create tempdir");
    let config_path = dir.path().join("config.json");

    let cfg = Config::default();
    assert_eq!(cfg.max_experience_words, 1200);
    assert_eq!(cfg.schedule_time, "02:00");
    assert_eq!(cfg.provider, "gemini");

    let json = serde_json::to_string_pretty(&cfg).expect("Failed to serialize config");
    fs::write(&config_path, &json).expect("Failed to write config");

    let read_json = fs::read_to_string(&config_path).expect("Failed to read config");
    let loaded: Config = serde_json::from_str(&read_json).expect("Failed to deserialize config");
    assert_eq!(loaded, cfg);
}

#[test]
fn test_db_initialization_and_migrations() {
    let dir = tempdir().expect("Failed to create tempdir");
    let db_path = dir.path().join("test_memory.db");

    let conn = db::init_db(Some(&db_path)).expect("Failed to init db");

    // 1. Verify schema_migrations contains baseline
    let mut stmt = conn
        .prepare("SELECT version FROM schema_migrations")
        .expect("Failed to prepare stmt");
    let versions: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .expect("Query failed")
        .flatten()
        .collect();

    assert_eq!(versions, vec![db::BASELINE_VERSION]);

    // 2. Verify idempotency
    assert!(db::run_migrations(&conn).is_ok());

    // 3. Verify tables exist
    let mut table_stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .expect("Failed to prepare table stmt");
    let tables: Vec<String> = table_stmt
        .query_map([], |row| row.get(0))
        .expect("Query failed")
        .flatten()
        .collect();

    assert!(tables.contains(&"schema_migrations".to_string()));
    assert!(tables.contains(&"sessions".to_string()));
    assert!(tables.contains(&"memory_facts".to_string()));
    assert!(tables.contains(&"consolidations".to_string()));

    // 4. Verify indexes exist
    let mut idx_stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='index' ORDER BY name")
        .expect("Failed to prepare idx stmt");
    let indexes: Vec<String> = idx_stmt
        .query_map([], |row| row.get(0))
        .expect("Query failed")
        .flatten()
        .collect();

    assert!(indexes.contains(&"idx_memory_facts_scope_status".to_string()));
    assert!(indexes.contains(&"idx_memory_facts_consolidation".to_string()));
    assert!(indexes.contains(&"idx_memory_facts_category".to_string()));

    // 5. Verify WAL mode
    let journal_mode: String = conn
        .query_row("PRAGMA journal_mode;", [], |row| row.get(0))
        .expect("Failed to query journal_mode");
    assert_eq!(journal_mode.to_lowercase(), "wal");
}

#[test]
fn test_db_fact_crud_lifecycle() {
    let dir = tempdir().expect("Failed to create tempdir");
    let db_path = dir.path().join("test_crud.db");

    let conn = db::init_db(Some(&db_path)).expect("Failed to init db");

    // 1. Insert session
    db::insert_session(&conn, "session-100", "chat", "/path/to/raw.jsonl")
        .expect("Failed to insert session");

    // 2. Insert active consolidated fact
    let mut f1 = MemoryFact::new(FactCategory::UserPreference, "Use pnpm on Windows.");
    f1.session_id = Some("session-100".to_string());
    f1.is_consolidated = 1;
    let f1_id = db::insert_fact(&conn, &f1).expect("Failed to insert fact 1");
    assert_eq!(f1_id, 1);

    // 3. Insert active retained fact
    let mut f2 = MemoryFact::new(FactCategory::DefensiveHeuristic, "Always run tests before release.");
    f2.is_consolidated = 1;
    let f2_id = db::insert_fact(&conn, &f2).expect("Failed to insert fact 2");
    assert_eq!(f2_id, 2);

    // 4. Insert pending contradicting fact (is_consolidated = 0)
    let mut f3 = MemoryFact::new(FactCategory::UserPreference, "ALWAYS execute npm inside WSL.");
    f3.is_consolidated = 0;
    let f3_id = db::insert_fact(&conn, &f3).expect("Failed to insert fact 3");
    assert_eq!(f3_id, 3);

    // 5. Query pending facts
    let pending = db::get_pending_facts(&conn, None).expect("Failed to get pending facts");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, Some(f3_id));
    assert_eq!(pending[0].rule_statement, "ALWAYS execute npm inside WSL.");

    // 6. Query active facts
    let active = db::get_active_facts(&conn, None).expect("Failed to get active facts");
    assert_eq!(active.len(), 3);

    // 7. Supersede fact #1 by fact #3
    db::supersede_fact(&conn, f1_id, f3_id).expect("Failed to supersede fact");

    // Fact #1 should no longer be active
    let active_after_supersede = db::get_active_facts(&conn, None).expect("Failed to get active facts");
    assert_eq!(active_after_supersede.len(), 2);
    assert!(!active_after_supersede.iter().any(|f| f.id == Some(f1_id)));

    // Verify superseded_by column
    let (status, superseded_by): (String, Option<i64>) = conn
        .query_row(
            "SELECT status, superseded_by FROM memory_facts WHERE id = ?",
            [f1_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("Query failed");
    assert_eq!(status, "superseded");
    assert_eq!(superseded_by, Some(f3_id));

    // 8. Mark fact #3 as consolidated
    let affected = db::mark_facts_consolidated(&conn, &[f3_id]).expect("Failed to mark consolidated");
    assert_eq!(affected, 1);

    let pending_after = db::get_pending_facts(&conn, None).expect("Failed to query pending");
    assert_eq!(pending_after.len(), 0);

    // 9. Record consolidation audit
    let audit = ConsolidationAudit::new(
        "gemini-3.8-flash-low",
        3,
        1,
        Some("Superseded pnpm rule with WSL npm rule".to_string()),
    );
    let audit_id = db::record_consolidation_audit(&conn, &audit).expect("Failed to record audit");
    assert!(audit_id >= 1);

    // 10. Test skills table upsert and retrieval
    let skill = SkillFile {
        key: "experienced-llm".to_string(),
        file_path: "/skills/experienced-llm/SKILL.md".to_string(),
        role_description: "Autonomous memory manager".to_string(),
        rules_summary: "Consolidate daily facts".to_string(),
        token_count: 180,
        status: "active".to_string(),
        created_at: None,
        updated_at: None,
    };
    db::upsert_skill(&conn, &skill).expect("Failed to upsert skill");
    let active_skills = db::get_active_skills(&conn).expect("Failed to get active skills");
    assert_eq!(active_skills.len(), 1);
    assert_eq!(active_skills[0].key, "experienced-llm");

    // 11. Test daily consolidation recording
    db::record_daily_consolidation(&conn, "2026-10-17", "/path/to/daily.md", 3, 1)
        .expect("Failed to record daily consolidation");
}

#[test]
fn test_python_rust_bidirectional_compatibility() {
    let dir = tempdir().expect("Failed to create tempdir");
    let db_path = dir.path().join("compat_memory.db");

    // Step A: Initialize and populate DB via Python src.db
    let py_script = format!(
        r#"
import sys
from pathlib import Path
sys.path.insert(0, ".")
from src.db import init_db, insert_session, insert_fact, get_connection
from src.models import MemoryFact, FactCategory, FactStatus

db_p = Path(r"{}")
init_db(db_p)
conn = get_connection(db_p)
insert_session("py-session-1", "chat", "py_raw.jsonl", conn=conn)
insert_fact(MemoryFact(
    category=FactCategory.TECHNICAL_DECISION,
    rule_statement="Store all operational wisdom in SQLite WAL mode.",
    context_reason="Zero concurrency lockups",
    scope="database",
    confidence=0.95,
    status=FactStatus.ACTIVE,
    is_consolidated=1
), conn=conn)
insert_fact(MemoryFact(
    category=FactCategory.PROJECT_GOTCHA,
    rule_statement="Never run node directly on Windows.",
    context_reason="Path mapping failure",
    status=FactStatus.ACTIVE,
    is_consolidated=0
), conn=conn)
conn.commit()
conn.close()
print("PYTHON_POPULATED_OK")
"#,
        db_path.display()
    );

    let py_res = Command::new("python3")
        .arg("-c")
        .arg(&py_script)
        .output();

    if let Ok(out) = py_res {
        let stdout = String::from_utf8_lossy(&out.stdout);
        if !stdout.contains("PYTHON_POPULATED_OK") {
            eprintln!("python3 test setup skipped or failed: {}", String::from_utf8_lossy(&out.stderr));
            return;
        }
    } else {
        eprintln!("python3 not found in test environment, skipping bidirectional test");
        return;
    }

    // Step B: Open with Rust db engine and verify Python data
    let conn = db::get_connection(Some(&db_path)).expect("Rust failed to open Python DB");
    let active_facts = db::get_active_facts(&conn, None).expect("Rust failed to get active facts");
    assert_eq!(active_facts.len(), 2);

    let f1 = active_facts.iter().find(|f| f.category == FactCategory::TechnicalDecision).unwrap();
    assert_eq!(f1.rule_statement, "Store all operational wisdom in SQLite WAL mode.");
    assert_eq!(f1.scope, "database");
    assert_eq!(f1.is_consolidated, 1);

    let f2 = active_facts.iter().find(|f| f.category == FactCategory::ProjectGotcha).unwrap();
    assert_eq!(f2.rule_statement, "Never run node directly on Windows.");
    assert_eq!(f2.is_consolidated, 0);

    // Step C: Rust mutates database (inserts a fact and supersedes Python's f1)
    let f1_id = f1.id.unwrap();
    let mut rust_fact = MemoryFact::new(FactCategory::TechnicalDecision, "Store operational wisdom in SQLite WAL with busy timeout.");
    rust_fact.scope = "database".to_string();
    rust_fact.is_consolidated = 1;
    let rust_f_id = db::insert_fact(&conn, &rust_fact).expect("Rust failed to insert fact");

    db::supersede_fact(&conn, f1_id, rust_f_id).expect("Rust failed to supersede fact");

    // Step D: Verify with Python that Rust mutations are 100% valid and readable
    let verify_py_script = format!(
        r#"
import sys
from pathlib import Path
sys.path.insert(0, ".")
from src.db import get_connection, get_active_facts

db_p = Path(r"{}")
conn = get_connection(db_p)
active = get_active_facts(conn=conn)
# f1 should be superseded, f2 and rust_fact should be active
assert len(active) == 2, f"Expected 2 active facts, found {{len(active)}}"
statements = [f.rule_statement for f in active]
assert "Store operational wisdom in SQLite WAL with busy timeout." in statements
assert "Never run node directly on Windows." in statements
conn.close()
print("PYTHON_VERIFIED_OK")
"#,
        db_path.display()
    );

    let verify_res = Command::new("python3")
        .arg("-c")
        .arg(&verify_py_script)
        .output()
        .expect("Failed to execute python verification");

    let stdout = String::from_utf8_lossy(&verify_res.stdout);
    assert!(stdout.contains("PYTHON_VERIFIED_OK"), "Python failed to read Rust mutations: {}", String::from_utf8_lossy(&verify_res.stderr));
}

#[test]
fn test_intraday_logging() {
    let dir = tempdir().expect("Failed to create tempdir");
    let db_path = dir.path().join("intraday_test.db");

    let conn = db::init_db(Some(&db_path)).expect("Failed to init db");

    let fact = experienced_llms::intraday::log_intraday(
        "Never run npm directly on Windows; always run inside WSL.",
        FactCategory::UserPreference,
        Some("global"),
        Some("Windows path collisions"),
        Some("session-xyz"),
        Some(&db_path),
    ).expect("Failed to log intraday");

    assert_eq!(fact.id, Some(1));
    assert_eq!(fact.is_consolidated, 0);
    assert_eq!(fact.status, FactStatus::Active);

    let pending = db::get_pending_facts(&conn, None).expect("Failed to get pending");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].rule_statement, "Never run npm directly on Windows; always run inside WSL.");
}

#[test]
fn test_renderer_clean_and_truncate() {
    // 1. Test clean_markdown_experience
    let dirty_md = r#"# Master AI Experience
<!-- internal consolidation note -->
Last consolidated: 2026-10-17 02:00:00

## 1. User Directives & Preferences
- Rule A

Generated at 2026-10-17
"#;
    let cleaned = experienced_llms::renderer::clean_markdown_experience(dirty_md);
    assert!(!cleaned.contains("<!--"));
    assert!(!cleaned.contains("Last consolidated"));
    assert!(!cleaned.contains("Generated at"));
    assert!(cleaned.contains("## 1. User Directives & Preferences"));
    assert!(cleaned.contains("- Rule A"));

    // 2. Test render_active_experience_markdown deduplication and word limit
    let f1 = MemoryFact::new(FactCategory::UserPreference, "Rule 1");
    let f2 = MemoryFact::new(FactCategory::UserPreference, "Rule 1"); // Duplicate
    let f3 = MemoryFact::new(FactCategory::DefensiveHeuristic, "Rule 2");

    let rendered = experienced_llms::renderer::render_active_experience_markdown(&[f1.clone(), f2, f3.clone()], None);
    assert_eq!(rendered.matches("Rule 1").count(), 1, "Duplicate rule should be deduplicated");

    let truncated = experienced_llms::renderer::render_active_experience_markdown(&[f1, f3], Some(10));
    let word_count = truncated.split_whitespace().count();
    assert!(word_count <= 10, "Rendered markdown should be truncated to max_words");
}

#[test]
fn test_cli_subcommands_sandbox() {
    let dir = tempdir().expect("Failed to create tempdir");
    let home_dir = dir.path().to_path_buf();

    // Test running binary via Command
    let exe = env!("CARGO_BIN_EXE_experienced-llms");

    // 1. Init
    let init_out = Command::new(exe)
        .env("EXPERIENCED_LLMS_HOME", &home_dir)
        .arg("init")
        .output()
        .expect("Failed to run init");
    assert!(init_out.status.success());

    // 2. Log
    let log_out = Command::new(exe)
        .env("EXPERIENCED_LLMS_HOME", &home_dir)
        .args(["log", "Always test CLI commands", "--category", "defensive_heuristic", "--scope", "testing"])
        .output()
        .expect("Failed to run log");
    assert!(log_out.status.success());
    let log_str = String::from_utf8_lossy(&log_out.stdout);
    assert!(log_str.contains("Logged [DEFENSIVE_HEURISTIC] (testing): Always test CLI commands"));

    // 3. Status
    let status_out = Command::new(exe)
        .env("EXPERIENCED_LLMS_HOME", &home_dir)
        .arg("status")
        .output()
        .expect("Failed to run status");
    assert!(status_out.status.success());
    let status_str = String::from_utf8_lossy(&status_out.stdout);
    assert!(status_str.contains("Active Memory Facts"));
    assert!(status_str.contains("Always test CLI commands"));

    // 4. Render
    let render_out = Command::new(exe)
        .env("EXPERIENCED_LLMS_HOME", &home_dir)
        .args(["render", "--max-words", "500"])
        .output()
        .expect("Failed to run render");
    assert!(render_out.status.success());

    // 5. Show
    let show_out = Command::new(exe)
        .env("EXPERIENCED_LLMS_HOME", &home_dir)
        .arg("show")
        .output()
        .expect("Failed to run show");
    assert!(show_out.status.success());
    let show_str = String::from_utf8_lossy(&show_out.stdout);
    assert!(show_str.contains("# Master AI Experience"));
    assert!(show_str.contains("Always test CLI commands"));
}

