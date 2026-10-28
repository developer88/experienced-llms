use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config;
use crate::db;
use crate::llm_client;

pub const LOCK_EXPIRATION_SECS: i64 = 20 * 60; // 20 minutes

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockMetadata {
    pub locked_at: String,
    pub expires_at_unix: i64,
    pub pid: u32,
}

pub struct ConsolidationLock {
    lock_path: PathBuf,
    acquired: bool,
}

impl ConsolidationLock {
    /// Attempts to acquire the 20-minute consolidation lock in `home_dir`.
    /// Returns:
    /// - `Ok(Some(lock))` if the lock was acquired.
    /// - `Ok(None)` if another process holds an unexpired lock.
    /// - `Err(...)` on unexpected I/O failure.
    pub fn try_acquire(home_dir: &Path) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        fs::create_dir_all(home_dir)?;
        let lock_path = home_dir.join("consolidation.lock");
        let now_unix = Utc::now().timestamp();

        if lock_path.exists() {
            let mut is_active = false;
            if let Ok(content) = fs::read_to_string(&lock_path) {
                if let Ok(meta) = serde_json::from_str::<LockMetadata>(&content) {
                    if now_unix < meta.expires_at_unix {
                        is_active = true;
                    }
                } else if let Ok(file_meta) = fs::metadata(&lock_path) {
                    if let Ok(modified) = file_meta.modified() {
                        if let Ok(elapsed) = modified.elapsed() {
                            if elapsed.as_secs() < LOCK_EXPIRATION_SECS as u64 {
                                is_active = true;
                            }
                        }
                    }
                }
            }
            if is_active {
                return Ok(None);
            }
            // Lock is expired (> 20 mins) or stale; remove it safely
            let _ = fs::remove_file(&lock_path);
        }

        let now = Local::now();
        let meta = LockMetadata {
            locked_at: now.to_rfc3339(),
            expires_at_unix: now_unix + LOCK_EXPIRATION_SECS,
            pid: std::process::id(),
        };

        let json = serde_json::to_string_pretty(&meta)?;
        fs::write(&lock_path, json)?;

        Ok(Some(Self {
            lock_path,
            acquired: true,
        }))
    }

    /// Releases the lock upon successful consolidation.
    pub fn release(mut self) {
        if self.acquired {
            let _ = fs::remove_file(&self.lock_path);
            self.acquired = false;
        }
    }

    /// Retains the lock file upon consolidation failure so that future calls
    /// within 20 minutes skip auto-consolidation and safely retry after 20 minutes.
    pub fn retain_on_failure(mut self) {
        self.acquired = false;
    }

    pub fn is_acquired(&self) -> bool {
        self.acquired
    }
}

impl Drop for ConsolidationLock {
    fn drop(&mut self) {
        if self.acquired {
            let _ = fs::remove_file(&self.lock_path);
        }
    }
}

/// Retrieve the active lock metadata if an unexpired lock exists in `home_dir`.
pub fn get_lock_status(home_dir: &Path) -> Option<LockMetadata> {
    let lock_path = home_dir.join("consolidation.lock");
    if !lock_path.exists() {
        return None;
    }
    if let Ok(content) = fs::read_to_string(&lock_path) {
        if let Ok(meta) = serde_json::from_str::<LockMetadata>(&content) {
            let now_unix = Utc::now().timestamp();
            if now_unix < meta.expires_at_unix {
                return Some(meta);
            }
        }
    }
    None
}

/// Parses a date string from the `consolidations` table into a NaiveDate.
pub fn parse_consolidation_date(raw: &str) -> Option<NaiveDate> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    // 1. Try RFC3339 format (e.g. 2026-10-26T21:55:00+01:00)
    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        return Some(dt.with_timezone(&Local).date_naive());
    }
    // 2. Try SQLite default CURRENT_TIMESTAMP format (YYYY-MM-DD HH:MM:SS)
    if let Ok(ndt) = NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S") {
        return Some(ndt.date());
    }
    // 3. Fallback to 10-character date prefix (YYYY-MM-DD)
    if trimmed.len() >= 10 {
        if let Ok(date) = NaiveDate::parse_from_str(&trimmed[..10], "%Y-%m-%d") {
            return Some(date);
        }
    }
    None
}

/// Queries the SQLite database for the date of the most recent consolidation.
pub fn get_last_consolidation_date(
    conn: &rusqlite::Connection,
) -> Result<Option<NaiveDate>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT consolidated_at FROM consolidations ORDER BY id DESC LIMIT 1;"
    )?;
    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        let raw_ts: String = row.get(0)?;
        return Ok(parse_consolidation_date(&raw_ts));
    }
    Ok(None)
}

/// Returns true if consolidation is needed (last consolidation was on a previous day or never).
pub fn should_autoconsolidate(
    conn: &rusqlite::Connection,
    current_date: NaiveDate,
) -> Result<bool, rusqlite::Error> {
    match get_last_consolidation_date(conn)? {
        Some(last_date) => Ok(last_date < current_date),
        None => Ok(true), // Never consolidated before
    }
}

/// Checks if auto-consolidation is needed and runs it if conditions are met.
/// Returns:
/// - `Ok(true)` if auto-consolidation ran successfully.
/// - `Ok(false)` if skipped (not needed, locked by another process, or no LLM configured).
/// - `Err(...)` if consolidation was attempted and failed (lock is retained for 20 minutes).
pub fn check_and_run_autoconsolidation(
    home_dir_opt: Option<&Path>,
    db_path_opt: Option<&Path>,
) -> Result<bool, Box<dyn std::error::Error>> {
    let home = home_dir_opt
        .map(|p| p.to_path_buf())
        .unwrap_or_else(config::get_experienced_home);
    let db_path = db_path_opt
        .map(|p| p.to_path_buf())
        .unwrap_or_else(config::get_db_path);

    if !db_path.exists() {
        return Ok(false);
    }

    let conn = db::get_connection(Some(&db_path))?;
    let today = Local::now().date_naive();

    if !should_autoconsolidate(&conn, today)? {
        return Ok(false);
    }

    // Attempt to acquire the 20-minute concurrency lock
    let lock = match ConsolidationLock::try_acquire(&home)? {
        Some(l) => l,
        None => {
            println!("[*] Another consolidation is in progress or lock active. Skipping auto-consolidation.");
            return Ok(false);
        }
    };

    // Check if an LLM is configured - fails hard if missing
    let client = match llm_client::get_llm_client(None) {
        Ok(c) => c,
        Err(_) => {
            lock.retain_on_failure();
            return Err("No LLM CLI command is configured. Auto-consolidation requires an LLM CLI to consolidate operational memory into EXPERIENCE.md.\n    To configure an LLM command, run:\n      experienced-llms config --llm \"<cmd>\"\n    Examples:\n      experienced-llms config --llm \"agy\"\n      experienced-llms config --llm \"claude -p\"\n      experienced-llms config --llm \"ollama run mistral\"\n    Or set the EXPERIENCED_LLMS_EXTERNAL_CMD environment variable.".into());
        }
    };

    println!("[*] Previous consolidation was on a previous day. Running auto-consolidation...");

    let consolidator = crate::consolidator::interday::InterdayConsolidator::with_home(
        Some(client),
        Some(home.clone()),
    );

    match consolidator.consolidate(None, None, Some("autoconsolidation"), Some(&db_path)) {
        Ok((_path, summary)) => {
            println!("[✓] Auto-consolidation complete: {}", summary);
            lock.release();
            Ok(true)
        }
        Err(err) => {
            lock.retain_on_failure();
            Err(format!("Auto-consolidation execution failed: {}.\n    Check your LLM configuration with 'experienced-llms config' or test with 'experienced-llms consolidate --llm \"<cmd>\"'.", err).into())
        }
    }
}
