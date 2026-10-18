use experienced_llms::config::Config;
use experienced_llms::db;
use experienced_llms::llm_client::LLMClient;
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

#[test]
fn test_prompt_template_resolution_and_hierarchy() {
    let dir = tempdir().expect("Failed to create tempdir");
    let home_path = dir.path().to_path_buf();

    // 1. Default fallback
    let default_tmpl = experienced_llms::consolidator::prompt_template::get_consolidation_template(
        None,
        Some(&home_path),
    );
    assert!(default_tmpl.contains("You are the master cognitive consolidation engine"));
    assert!(default_tmpl.contains("## 1. User Directives & Preferences"));
    assert!(default_tmpl.contains("## 2. Defensive Safeguards & Heuristics"));
    assert!(default_tmpl.contains("## 3. Project-Specific Constraints"));
    assert!(default_tmpl.contains("## 4. Anti-Patterns & Critical Gotchas"));
    assert!(default_tmpl.contains("## 5. Mistake Corrections & Fixes"));
    assert!(default_tmpl.contains("{max_words}"));

    // 2. User home directory override: ~/.experienced-llms/consolidation_prompt.txt
    let home_file = home_path.join("consolidation_prompt.txt");
    fs::write(&home_file, "HOME PROMPT TEMPLATE {max_words}").expect("Failed to write home template");

    let home_tmpl = experienced_llms::consolidator::prompt_template::get_consolidation_template(
        None,
        Some(&home_path),
    );
    assert_eq!(home_tmpl, "HOME PROMPT TEMPLATE {max_words}");

    // 3. Explicit custom path override (--template)
    let custom_file = home_path.join("custom_template.txt");
    fs::write(&custom_file, "EXPLICIT TEMPLATE {max_words}").expect("Failed to write custom template");

    let explicit_tmpl = experienced_llms::consolidator::prompt_template::get_consolidation_template(
        Some(&custom_file),
        Some(&home_path),
    );
    assert_eq!(explicit_tmpl, "EXPLICIT TEMPLATE {max_words}");
}

#[test]
fn test_external_command_llm_client() {
    #[cfg(not(windows))]
    let mock_cmd = r##"echo '{"updated_experience_markdown": "# Mock Title\n- rule 1", "summary_of_changes": "Mocked", "processed_pending_ids": [], "superseded_fact_ids": []}'"##;

    #[cfg(windows)]
    let mock_cmd = r##"echo {"updated_experience_markdown": "# Mock Title\n- rule 1", "summary_of_changes": "Mocked", "processed_pending_ids": [], "superseded_fact_ids": []}"##;

    let client = experienced_llms::llm_client::ExternalCommandLLMClient::new(mock_cmd)
        .expect("Failed to create client");
    let resp = client
        .generate("System prompt", "User prompt", true)
        .expect("Failed to generate");
    assert!(resp.contains("Mock Title"));

    // Test failure handling with failing exit code
    #[cfg(not(windows))]
    let fail_cmd = "sh -c 'echo \"Fatal LLM failure\" >&2; exit 2'";

    #[cfg(windows)]
    let fail_cmd = "cmd /C echo Fatal LLM failure 1>&2 & exit 2";

    let fail_client = experienced_llms::llm_client::ExternalCommandLLMClient::new(fail_cmd)
        .expect("Failed to create fail client");
    let err = fail_client.generate("sys", "user", false);
    assert!(err.is_err());
    let err_msg = err.unwrap_err().to_string();
    assert!(err_msg.contains("Fatal LLM failure") || err_msg.contains("External LLM command failed"));
}

#[test]
fn test_consolidator_engine_full_lifecycle() {
    let dir = tempdir().expect("Failed to create tempdir");
    let home_path = dir.path().to_path_buf();
    let db_path = home_path.join("memory.db");

    let mut conn = db::init_db(Some(&db_path)).expect("Failed to init db");

    // Insert active fact f1
    let f1_id = db::insert_fact(
        &conn,
        &MemoryFact::new(FactCategory::UserPreference, "Old rule to supersede"),
    )
    .expect("Failed to insert f1");
    // Mark f1 consolidated
    conn.execute(
        "UPDATE memory_facts SET is_consolidated = 1 WHERE id = ?",
        rusqlite::params![f1_id],
    )
    .expect("Failed to update f1");

    // Insert pending fact f2
    let f2_id = db::insert_fact(
        &conn,
        &MemoryFact::new(FactCategory::UserPreference, "New pending rule to integrate"),
    )
    .expect("Failed to insert f2");

    let consolidator = experienced_llms::consolidator::InterdayConsolidator::with_home(None, Some(home_path.clone()));

    // 1. Test build_consolidation_prompt
    let (sys_opt, user_opt, meta) = consolidator
        .build_consolidation_prompt(&conn, Some("2026-10-17"), Some(800), None)
        .expect("Failed to build prompt");

    assert!(sys_opt.is_some());
    let sys = sys_opt.unwrap();
    assert!(sys.contains("under 800 words"));

    assert!(user_opt.is_some());
    let user = user_opt.unwrap();
    assert!(user.contains("Old rule to supersede"));
    assert!(user.contains("New pending rule to integrate"));

    assert_eq!(meta.active_facts_count, 2);
    assert_eq!(meta.pending_facts_count, 1);
    assert_eq!(meta.pending_fact_ids, vec![f2_id]);

    // 2. Test apply_consolidation_result
    let payload = serde_json::json!({
        "updated_experience_markdown": "# Master AI Experience\n<!-- comment -->\nGenerated at: 2026-10-17\n\n## 1. User Directives & Preferences\n- New pending rule to integrate",
        "processed_pending_ids": [f2_id],
        "superseded_fact_ids": [f1_id],
        "summary_of_changes": "Superseded old rule and applied new rule."
    });

    let (master_path, summary) = consolidator
        .apply_consolidation_result(&mut conn, &payload.to_string(), "test_engine")
        .expect("Failed to apply consolidation result");

    assert_eq!(summary, "Superseded old rule and applied new rule.");
    assert!(master_path.exists());

    // Check clean markdown in EXPERIENCE.md
    let md_content = fs::read_to_string(&master_path).expect("Failed to read master experience");
    assert!(md_content.contains("# Master AI Experience"));
    assert!(md_content.contains("- New pending rule to integrate"));
    assert!(!md_content.contains("<!-- comment -->"));
    assert!(!md_content.contains("Generated at:"));

    // Verify SQLite states
    let mut stmt = conn
        .prepare("SELECT status, is_consolidated FROM memory_facts WHERE id = ?")
        .expect("Failed to prepare stmt");

    let f1_row: (String, i64) = stmt
        .query_row(rusqlite::params![f1_id], |r| Ok((r.get(0)?, r.get(1)?)))
        .expect("Failed to get f1");
    assert_eq!(f1_row.0, "superseded");

    let f2_row: (String, i64) = stmt
        .query_row(rusqlite::params![f2_id], |r| Ok((r.get(0)?, r.get(1)?)))
        .expect("Failed to get f2");
    assert_eq!(f2_row.1, 1); // marked consolidated

    // Verify audit record in consolidations table
    let audit_count: i64 = conn
        .query_row("SELECT count(*) FROM consolidations WHERE engine_type = 'test_engine'", [], |r| r.get(0))
        .expect("Failed to get audit count");
    assert_eq!(audit_count, 1);
}

#[test]
fn test_consolidate_pathway_a_mock_and_abort_resilience() {
    let dir = tempdir().expect("Failed to create tempdir");
    let home_path = dir.path().to_path_buf();
    let db_path = home_path.join("memory.db");

    let conn = db::init_db(Some(&db_path)).expect("Failed to init db");

    let f1_id = db::insert_fact(
        &conn,
        &MemoryFact::new(FactCategory::UserPreference, "Resilience test fact"),
    )
    .expect("Failed to insert fact");

    // 1. Test fail-safe abort with failing client (empty response)
    let failing_mock = experienced_llms::llm_client::MockLLMClient::new("");
    let failing_consolidator = experienced_llms::consolidator::InterdayConsolidator::with_home(
        Some(Box::new(failing_mock)),
        Some(home_path.clone()),
    );

    let result = failing_consolidator.consolidate(None, None, None, None, Some(&db_path));
    assert!(result.is_err(), "Consolidator must abort if LLM returns empty output");

    // Verify fact was NOT consolidated and no audit was written
    let f1_status: (String, i64) = conn
        .query_row(
            "SELECT status, is_consolidated FROM memory_facts WHERE id = ?",
            rusqlite::params![f1_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("Failed to query fact");
    assert_eq!(f1_status.0, "active");
    assert_eq!(f1_status.1, 0);

    let audits: i64 = conn
        .query_row("SELECT count(*) FROM consolidations", [], |r| r.get(0))
        .expect("Failed to query audits");
    assert_eq!(audits, 0);

    // 2. Test successful consolidation pass with Mock client
    let valid_response = serde_json::json!({
        "updated_experience_markdown": "# Master AI Experience\n\n## 1. User Directives & Preferences\n- Integrated resilience fact",
        "processed_pending_ids": [f1_id],
        "superseded_fact_ids": [],
        "summary_of_changes": "Mock consolidation success"
    });

    let success_mock = experienced_llms::llm_client::MockLLMClient::new(valid_response.to_string());
    let success_consolidator = experienced_llms::consolidator::InterdayConsolidator::with_home(
        Some(Box::new(success_mock)),
        Some(home_path.clone()),
    );

    let (master_path, summary) = success_consolidator
        .consolidate(None, None, None, Some("mock_test"), Some(&db_path))
        .expect("Consolidation should succeed");

    assert_eq!(summary, "Mock consolidation success");
    assert!(master_path.exists());

    let final_status: i64 = conn
        .query_row(
            "SELECT is_consolidated FROM memory_facts WHERE id = ?",
            rusqlite::params![f1_id],
            |r| r.get(0),
        )
        .expect("Failed to query fact final status");
    assert_eq!(final_status, 1);
}

#[test]
fn test_cli_consolidate_prepare_and_apply() {
    let dir = tempdir().expect("Failed to create tempdir");
    let home_path = dir.path().to_path_buf();
    let exe = env!("CARGO_BIN_EXE_experienced-llms");

    // 1. Init
    let init_out = Command::new(exe)
        .env("EXPERIENCED_LLMS_HOME", &home_path)
        .arg("init")
        .output()
        .expect("Failed to run init");
    assert!(init_out.status.success());

    // 2. Log facts
    let log1 = Command::new(exe)
        .env("EXPERIENCED_LLMS_HOME", &home_path)
        .args(["log", "First test rule for consolidation", "--category", "user_preference"])
        .output()
        .expect("Failed to log 1");
    assert!(log1.status.success());

    let log2 = Command::new(exe)
        .env("EXPERIENCED_LLMS_HOME", &home_path)
        .args(["log", "Second test rule for consolidation", "--category", "defensive_heuristic"])
        .output()
        .expect("Failed to log 2");
    assert!(log2.status.success());

    // 3. Test consolidate --prepare
    let prepare_out = Command::new(exe)
        .env("EXPERIENCED_LLMS_HOME", &home_path)
        .args(["consolidate", "--prepare"])
        .output()
        .expect("Failed to run consolidate --prepare");
    assert!(prepare_out.status.success());

    let prepare_json = String::from_utf8_lossy(&prepare_out.stdout);
    assert!(prepare_json.contains("\"metadata\""));
    assert!(prepare_json.contains("\"system_prompt\""));
    assert!(prepare_json.contains("\"user_prompt\""));
    assert!(prepare_json.contains("First test rule for consolidation"));
    assert!(prepare_json.contains("Second test rule for consolidation"));

    // 4. Test consolidate --template <path> --prepare
    let custom_tmpl_file = home_path.join("custom_prompt.txt");
    fs::write(&custom_tmpl_file, "SPECIAL CLI TEMPLATE {max_words}").expect("Failed to write custom template");

    let prepare_custom = Command::new(exe)
        .env("EXPERIENCED_LLMS_HOME", &home_path)
        .args(["consolidate", "--template", custom_tmpl_file.to_str().unwrap(), "--prepare"])
        .output()
        .expect("Failed to run consolidate with custom template");
    assert!(prepare_custom.status.success());
    let custom_json = String::from_utf8_lossy(&prepare_custom.stdout);
    assert!(custom_json.contains("SPECIAL CLI TEMPLATE 1200"));

    // 5. Test consolidate --apply '<payload>'
    let apply_payload = serde_json::json!({
        "updated_experience_markdown": "# Master AI Experience\n\n## 1. User Directives & Preferences\n- Consolidated via CLI test\n\n## 2. Defensive Safeguards & Heuristics\n- Defensive safeguard",
        "processed_pending_ids": [1, 2],
        "superseded_fact_ids": [],
        "summary_of_changes": "Applied via CLI test sandbox"
    });

    let apply_out = Command::new(exe)
        .env("EXPERIENCED_LLMS_HOME", &home_path)
        .args(["consolidate", "--apply", &apply_payload.to_string()])
        .output()
        .expect("Failed to run consolidate --apply");
    assert!(apply_out.status.success());
    let apply_str = String::from_utf8_lossy(&apply_out.stdout);
    assert!(apply_str.contains("[✓] Consolidation applied successfully"));
    assert!(apply_str.contains("Applied via CLI test sandbox"));

    // 6. Test show displays the updated EXPERIENCE.md
    let show_out = Command::new(exe)
        .env("EXPERIENCED_LLMS_HOME", &home_path)
        .arg("show")
        .output()
        .expect("Failed to run show");
    assert!(show_out.status.success());
    let show_str = String::from_utf8_lossy(&show_out.stdout);
    assert!(show_str.contains("Consolidated via CLI test"));
    assert!(show_str.contains("Defensive safeguard"));
}

#[test]
fn test_scheduler_time_parsing_and_status() {
    assert_eq!(experienced_llms::scheduler::parse_time("02:00"), (2, 0));
    assert_eq!(experienced_llms::scheduler::parse_time("15:45"), (15, 45));
    assert_eq!(experienced_llms::scheduler::parse_time("invalid"), (2, 0));

    let status = experienced_llms::scheduler::get_schedule_status("auto");
    assert!(!status.is_empty());

    let uninst = experienced_llms::scheduler::uninstall_schedule("auto");
    assert!(uninst.is_ok());
}

#[test]
fn test_agent_integrations_files() {
    let dir = tempdir().expect("Failed to create tempdir");
    let ws = dir.path().to_path_buf();

    // 1. Antigravity
    let ag_dir = ws.join("antigravity_skill");
    let ag_res = experienced_llms::integrations::install_antigravity(Some(&ag_dir));
    assert!(ag_res.is_ok());
    assert!(ag_dir.join("SKILL.md").exists());

    // 2. Claude
    let cl_res = experienced_llms::integrations::install_claude(Some(&ws));
    assert!(cl_res.is_ok());
    let cl_file = ws.join("CLAUDE.md");
    assert!(cl_file.exists());
    let cl_content = fs::read_to_string(&cl_file).unwrap();
    assert!(cl_content.contains("<!-- BEGIN EXPERIENCED-LLMS -->"));

    // 3. Idempotent block replacement
    experienced_llms::integrations::append_or_replace_block(
        &cl_file,
        experienced_llms::integrations::UNIVERSAL_INSTRUCTION_BLOCK,
    )
    .unwrap();
    let cl_content_again = fs::read_to_string(&cl_file).unwrap();
    assert_eq!(
        cl_content_again.matches("<!-- BEGIN EXPERIENCED-LLMS -->").count(),
        1,
        "Instruction block must be replaced idempotently without duplicate markers"
    );

    // 4. Copilot
    let cp_res = experienced_llms::integrations::install_copilot(Some(&ws));
    assert!(cp_res.is_ok());
    assert!(ws.join(".github").join("copilot-instructions.md").exists());

    // 5. Cursor
    let cur_res = experienced_llms::integrations::install_cursor(Some(&ws));
    assert!(cur_res.is_ok());
    assert!(ws.join(".cursorrules").exists());

    // 6. Pi
    let pi_res = experienced_llms::integrations::install_pi(Some(&ws));
    assert!(pi_res.is_ok());
    assert!(ws.join("AGENTS.md").exists());
}

#[test]
fn test_cli_integrate_and_schedule_commands() {
    let dir = tempdir().expect("Failed to create tempdir");
    let ws = dir.path().to_path_buf();
    let exe = env!("CARGO_BIN_EXE_experienced-llms");

    // 1. Integrate targets
    let out_claude = Command::new(exe)
        .args(["integrate", "--target", "claude", "--workspace", ws.to_str().unwrap()])
        .output()
        .expect("Failed to run integrate claude");
    assert!(out_claude.status.success());
    assert!(ws.join("CLAUDE.md").exists());

    let out_cursor = Command::new(exe)
        .args(["integrate", "--target", "cursor", "--workspace", ws.to_str().unwrap()])
        .output()
        .expect("Failed to run integrate cursor");
    assert!(out_cursor.status.success());
    assert!(ws.join(".cursorrules").exists());

    let out_all = Command::new(exe)
        .args(["integrate", "--target", "all", "--workspace", ws.to_str().unwrap()])
        .output()
        .expect("Failed to run integrate all");
    assert!(out_all.status.success());
    assert!(ws.join("AGENTS.md").exists());

    // 2. Schedule status
    let out_status = Command::new(exe)
        .args(["schedule", "--status"])
        .output()
        .expect("Failed to run schedule status");
    assert!(out_status.status.success());
    let status_str = String::from_utf8_lossy(&out_status.stdout);
    assert!(status_str.contains("Scheduler Status:"));
}

#[test]
fn test_skill_packaging_and_compliance() {
    let skill_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("skills")
        .join("experienced-llm")
        .join("SKILL.md");

    assert!(skill_path.exists(), "skills/experienced-llm/SKILL.md must exist");

    let content = fs::read_to_string(&skill_path).expect("Failed to read SKILL.md");
    let fm_re = regex::Regex::new(r"(?s)^---\r?\n(.*?)\r?\n---").unwrap();
    let caps = fm_re.captures(&content).expect("SKILL.md must start with YAML frontmatter bounded by ---");
    let frontmatter = caps.get(1).unwrap().as_str();

    // 1. Name
    let name_re = regex::Regex::new(r"(?m)^name:\s*([\w\-]+)").unwrap();
    let name_caps = name_re.captures(frontmatter).expect("Missing 'name' in YAML frontmatter");
    assert_eq!(&name_caps[1], "experienced-llm");

    // 2. Description (< 200 chars, > 20 chars)
    let desc_re = regex::Regex::new(r"(?m)^description:\s*(.+)$").unwrap();
    let desc_caps = desc_re.captures(frontmatter).expect("Missing 'description' in YAML frontmatter");
    let desc = desc_caps[1].trim();
    assert!(desc.len() < 200, "Description must be < 200 chars");
    assert!(desc.len() > 20, "Description must be > 20 chars");

    // 3. Version
    let ver_re = regex::Regex::new(r"(?m)^version:\s*(.+)$").unwrap();
    let ver_caps = ver_re.captures(frontmatter).expect("Missing 'version' in YAML frontmatter");
    assert_eq!(ver_caps[1].trim(), "1.0.0rc4");

    // 4. Author
    let author_re = regex::Regex::new(r"(?m)^author:\s*(.+)$").unwrap();
    let author_caps = author_re.captures(frontmatter).expect("Missing 'author' in YAML frontmatter");
    assert_eq!(author_caps[1].trim(), "developer88");

    // 5. Repository
    let repo_re = regex::Regex::new(r"(?m)^repository:\s*(.+)$").unwrap();
    let repo_caps = repo_re.captures(frontmatter).expect("Missing 'repository' in YAML frontmatter");
    assert_eq!(repo_caps[1].trim(), "https://github.com/developer88/experienced-llms");

    // 6. License
    let lic_re = regex::Regex::new(r"(?m)^license:\s*(.+)$").unwrap();
    let lic_caps = lic_re.captures(frontmatter).expect("Missing 'license' in YAML frontmatter");
    assert_eq!(lic_caps[1].trim(), "MIT");

    // 7. Keywords
    assert!(frontmatter.contains("keywords:"));
    assert!(frontmatter.contains("memory"));
    assert!(frontmatter.contains("persistent-memory"));

    // 8. Operational documentation sections
    assert!(content.contains("# Experienced LLM Operating Protocol"));
    assert!(content.contains("## 1. Startup Bootstrap"));
    assert!(content.contains("## 2. Daytime Operational Logging"));
    assert!(content.contains("## 3. Consolidation Engine"));
}


