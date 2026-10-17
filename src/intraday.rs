use chrono::Local;
use rusqlite::Result;
use std::path::Path;

use crate::config;
use crate::db;
use crate::models::{FactCategory, FactStatus, MemoryFact};

/// Record an operational rule or mistake correction directly into SQLite memory_facts
/// with is_consolidated = 0.
/// 100% deterministic, instant, free, and operates completely offline.
pub fn log_intraday(
    rule_statement: &str,
    category: FactCategory,
    scope: Option<&str>,
    context_reason: Option<&str>,
    session_id: Option<&str>,
    db_path: Option<&Path>,
) -> Result<MemoryFact> {
    config::ensure_directories().map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(e),
        )
    })?;

    let now = Local::now();
    let date_str = now.format("%Y-%m-%d").to_string();

    let actual_session_id = session_id
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("intraday_{}", date_str));

    let actual_scope = scope.unwrap_or("global").to_string();

    let conn = db::get_connection(db_path)?;
    db::insert_session(&conn, &actual_session_id, "intraday", "")?;

    let mut fact = MemoryFact::new(category, rule_statement);
    fact.session_id = Some(actual_session_id);
    fact.scope = actual_scope;
    fact.context_reason = context_reason.filter(|s| !s.trim().is_empty()).map(|s| s.to_string());
    fact.status = FactStatus::Active;
    fact.is_consolidated = 0;

    let row_id = db::insert_fact(&conn, &fact)?;
    fact.id = Some(row_id);

    Ok(fact)
}
