use rusqlite::{params, Connection, Result};
use std::fs;
use std::path::Path;

use crate::config;
use crate::models::{ConsolidationAudit, MemoryFact, ParseEnumError, SkillFile};

pub const BASELINE_VERSION: &str = "001_1_0_0rc3_baseline";
pub const BASELINE_SCHEMA_SQL: &str = include_str!("../schema.sql");

/// Open a SQLite connection with standard WAL mode, busy timeout, and foreign keys.
pub fn get_connection(db_path: Option<&Path>) -> Result<Connection> {
    let target = match db_path {
        Some(p) => p.to_path_buf(),
        None => config::get_db_path(),
    };

    if let Some(parent) = target.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let conn = Connection::open(&target)?;
    conn.busy_timeout(std::time::Duration::from_millis(5000))?;
    conn.execute("PRAGMA foreign_keys = ON;", [])?;
    let _ = conn.execute("PRAGMA journal_mode = WAL;", []);
    Ok(conn)
}

/// Initialize the database, execute baseline schema, and record migrations.
pub fn init_db(db_path: Option<&Path>) -> Result<Connection> {
    let _ = config::ensure_directories();
    let conn = get_connection(db_path)?;
    run_migrations(&conn)?;
    Ok(conn)
}

/// Run embedded schema migrations sequentially.
pub fn run_migrations(conn: &Connection) -> Result<()> {
    // 1. Ensure schema_migrations table exists
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version TEXT PRIMARY KEY,
            applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );",
        [],
    )?;

    // 2. Check if baseline applied
    let mut stmt = conn.prepare("SELECT 1 FROM schema_migrations WHERE version = ?")?;
    let baseline_applied = stmt.exists([BASELINE_VERSION])?;

    if !baseline_applied {
        // Execute baseline SQL script
        conn.execute_batch(BASELINE_SCHEMA_SQL)?;
        conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES (?)",
            [BASELINE_VERSION],
        )?;
    } else {
        // Backward compatibility: ensure is_consolidated column exists if table predated it
        let mut pragma_stmt = conn.prepare("PRAGMA table_info(memory_facts);")?;
        let rows = pragma_stmt.query_map([], |row| row.get::<_, String>(1))?;
        let mut has_is_consolidated = false;
        let mut has_memory_facts = false;
        for col_name in rows.flatten() {
            has_memory_facts = true;
            if col_name == "is_consolidated" {
                has_is_consolidated = true;
                break;
            }
        }
        if has_memory_facts && !has_is_consolidated {
            conn.execute(
                "ALTER TABLE memory_facts ADD COLUMN is_consolidated INTEGER DEFAULT 0;",
                [],
            )?;
        }
    }

    Ok(())
}

/// Insert or update a session record.
pub fn insert_session(
    conn: &Connection,
    session_id: &str,
    source: &str,
    raw_path: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO sessions (id, source, raw_path)
         VALUES (?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET raw_path=excluded.raw_path;",
        params![session_id, source, raw_path],
    )?;
    Ok(())
}

/// Insert a memory fact into the database, returning its new row id.
pub fn insert_fact(conn: &Connection, fact: &MemoryFact) -> Result<i64> {
    conn.execute(
        "INSERT INTO memory_facts (
            session_id, category, scope, rule_statement,
            context_reason, confidence, status, is_consolidated, superseded_by
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?);",
        params![
            fact.session_id,
            fact.category.as_str(),
            fact.scope,
            fact.rule_statement,
            fact.context_reason,
            fact.confidence,
            fact.status.as_str(),
            fact.is_consolidated,
            fact.superseded_by,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Mark an older fact as superseded by a newer fact.
pub fn supersede_fact(conn: &Connection, old_fact_id: i64, new_fact_id: i64) -> Result<()> {
    conn.execute(
        "UPDATE memory_facts
         SET status = 'superseded', superseded_by = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?;",
        params![new_fact_id, old_fact_id],
    )?;
    Ok(())
}

/// Fetch pending (unconsolidated, active/user_verified) facts.
pub fn get_pending_facts(conn: &Connection, scope: Option<&str>) -> Result<Vec<MemoryFact>> {
    let mut query = "SELECT id, session_id, category, scope, rule_statement, context_reason, \
                     confidence, status, is_consolidated, superseded_by, created_at, updated_at \
                     FROM memory_facts \
                     WHERE is_consolidated = 0 AND status IN ('active', 'user_verified')"
        .to_string();

    let mut facts = Vec::new();
    if let Some(s) = scope {
        query.push_str(" AND (scope = ?1 OR scope = 'global') ORDER BY id ASC;");
        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map(params![s], row_to_fact)?;
        for r in rows {
            facts.push(r?);
        }
    } else {
        query.push_str(" ORDER BY id ASC;");
        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map([], row_to_fact)?;
        for r in rows {
            facts.push(r?);
        }
    }

    Ok(facts)
}

/// Fetch all active facts (active/user_verified).
pub fn get_active_facts(conn: &Connection, scope: Option<&str>) -> Result<Vec<MemoryFact>> {
    let mut query = "SELECT id, session_id, category, scope, rule_statement, context_reason, \
                     confidence, status, is_consolidated, superseded_by, created_at, updated_at \
                     FROM memory_facts \
                     WHERE status IN ('active', 'user_verified')"
        .to_string();

    let mut facts = Vec::new();
    if let Some(s) = scope {
        query.push_str(" AND (scope = ?1 OR scope = 'global') ORDER BY id ASC;");
        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map(params![s], row_to_fact)?;
        for r in rows {
            facts.push(r?);
        }
    } else {
        query.push_str(" ORDER BY id ASC;");
        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map([], row_to_fact)?;
        for r in rows {
            facts.push(r?);
        }
    }

    Ok(facts)
}

/// Mark an array of fact IDs as consolidated.
pub fn mark_facts_consolidated(conn: &Connection, fact_ids: &[i64]) -> Result<usize> {
    if fact_ids.is_empty() {
        return Ok(0);
    }

    let placeholders = fact_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "UPDATE memory_facts \
         SET is_consolidated = 1, updated_at = CURRENT_TIMESTAMP \
         WHERE id IN ({});",
        placeholders
    );

    let params_from_iter = rusqlite::params_from_iter(fact_ids.iter());
    let affected = conn.execute(&sql, params_from_iter)?;
    Ok(affected)
}

/// Record a consolidation audit entry.
pub fn record_consolidation_audit(
    conn: &Connection,
    audit: &ConsolidationAudit,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO consolidations (engine_type, facts_processed, facts_superseded, summary_of_changes)
         VALUES (?, ?, ?, ?);",
        params![
            audit.engine_type,
            audit.facts_processed,
            audit.facts_superseded,
            audit.summary_of_changes,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Fetch facts by creation date (YYYY-MM-DD).
pub fn get_facts_by_date(conn: &Connection, date_str: &str) -> Result<Vec<MemoryFact>> {
    let sql = "SELECT id, session_id, category, scope, rule_statement, context_reason, \
               confidence, status, is_consolidated, superseded_by, created_at, updated_at \
               FROM memory_facts \
               WHERE strftime('%Y-%m-%d', created_at, 'localtime') = ?1 \
                  OR strftime('%Y-%m-%d', created_at) = ?1 \
               ORDER BY id ASC;";

    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![date_str], row_to_fact)?;
    let mut facts = Vec::new();
    for r in rows {
        facts.push(r?);
    }
    Ok(facts)
}

/// Upsert a skill definition into the skills table.
pub fn upsert_skill(conn: &Connection, skill: &SkillFile) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS skills (
            key TEXT PRIMARY KEY,
            file_path TEXT NOT NULL,
            role_description TEXT NOT NULL,
            rules_summary TEXT NOT NULL,
            token_count INTEGER DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'active',
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );",
        [],
    )?;

    conn.execute(
        "INSERT INTO skills (key, file_path, role_description, rules_summary, token_count, status)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(key) DO UPDATE SET
            file_path=excluded.file_path,
            role_description=excluded.role_description,
            rules_summary=excluded.rules_summary,
            token_count=excluded.token_count,
            status=excluded.status,
            updated_at=CURRENT_TIMESTAMP;",
        params![
            skill.key,
            skill.file_path,
            skill.role_description,
            skill.rules_summary,
            skill.token_count,
            skill.status,
        ],
    )?;
    Ok(())
}

/// Get all active skills.
pub fn get_active_skills(conn: &Connection) -> Result<Vec<SkillFile>> {
    let mut table_check = conn.prepare("SELECT 1 FROM sqlite_master WHERE type='table' AND name='skills';")?;
    if !table_check.exists([])? {
        return Ok(Vec::new());
    }

    let mut stmt = conn.prepare(
        "SELECT key, file_path, role_description, rules_summary, token_count, status, created_at, updated_at \
         FROM skills WHERE status = 'active' ORDER BY key ASC;",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(SkillFile {
            key: row.get(0)?,
            file_path: row.get(1)?,
            role_description: row.get(2)?,
            rules_summary: row.get(3)?,
            token_count: row.get(4)?,
            status: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })?;

    let mut skills = Vec::new();
    for r in rows {
        skills.push(r?);
    }
    Ok(skills)
}

/// Record daily consolidation summary.
pub fn record_daily_consolidation(
    conn: &Connection,
    date_str: &str,
    file_path: &str,
    facts_count: i64,
    skills_updated: i64,
) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS daily_consolidations (
            date TEXT PRIMARY KEY,
            file_path TEXT NOT NULL,
            facts_count INTEGER DEFAULT 0,
            skills_updated INTEGER DEFAULT 0,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );",
        [],
    )?;

    conn.execute(
        "INSERT INTO daily_consolidations (date, file_path, facts_count, skills_updated)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(date) DO UPDATE SET
            file_path=excluded.file_path,
            facts_count=excluded.facts_count,
            skills_updated=excluded.skills_updated;",
        params![date_str, file_path, facts_count, skills_updated],
    )?;
    Ok(())
}

fn row_to_fact(row: &rusqlite::Row) -> Result<MemoryFact> {
    let category_str: String = row.get("category")?;
    let status_str: String = row.get("status")?;

    let category = category_str.parse().map_err(|e: ParseEnumError| {
        rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(e))
    })?;

    let status = status_str.parse().map_err(|e: ParseEnumError| {
        rusqlite::Error::FromSqlConversionFailure(7, rusqlite::types::Type::Text, Box::new(e))
    })?;

    Ok(MemoryFact {
        id: row.get("id")?,
        session_id: row.get("session_id")?,
        category,
        scope: row.get("scope")?,
        rule_statement: row.get("rule_statement")?,
        context_reason: row.get("context_reason")?,
        confidence: row.get("confidence")?,
        status,
        is_consolidated: row.get::<_, Option<i64>>("is_consolidated")?.unwrap_or(0),
        superseded_by: row.get("superseded_by")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}
