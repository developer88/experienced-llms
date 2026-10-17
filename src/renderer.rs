use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::config;
use crate::db;
use crate::models::{FactCategory, MemoryFact};

/// Strips HTML comments and generation timestamps to enforce zero token waste.
pub fn clean_markdown_experience(raw_md: &str) -> String {
    // Strip HTML comments: <!-- ... -->
    let re_comment = Regex::new(r"(?s)<!--.*?-->").unwrap();
    let cleaned = re_comment.replace_all(raw_md, "");

    // Strip generation timestamps or metadata lines
    let re_timestamps = Regex::new(r"(?im)^.*(last consolidated|generated at|timestamp).*$\n?").unwrap();
    let cleaned = re_timestamps.replace_all(&cleaned, "");

    // Normalize multiple newlines
    let re_newlines = Regex::new(r"\n{3,}").unwrap();
    let cleaned = re_newlines.replace_all(&cleaned, "\n\n");

    format!("{}\n", cleaned.trim())
}

/// Renders clean, authoritative EXPERIENCE.md directly from active SQLite facts without calling LLM.
pub fn render_active_experience_markdown(facts: &[MemoryFact], max_words: Option<usize>) -> String {
    let categories = [
        (FactCategory::UserPreference, "## 1. User Directives & Preferences"),
        (FactCategory::DefensiveHeuristic, "## 2. Defensive Safeguards & Heuristics"),
        (FactCategory::TechnicalDecision, "## 3. Project-Specific Constraints"),
        (FactCategory::ProjectGotcha, "## 4. Anti-Patterns & Critical Gotchas"),
        (FactCategory::MistakeCorrection, "## 5. Mistake Corrections & Fixes"),
    ];

    let mut lines = vec![
        "# Master AI Experience".to_string(),
        "".to_string(),
    ];

    let mut seen_statements = HashSet::new();
    for (cat_enum, header) in categories {
        let matching: Vec<&MemoryFact> = facts.iter().filter(|f| f.category == cat_enum).collect();
        lines.push(header.to_string());
        let mut has_items = false;

        for f in matching {
            let statement = f.rule_statement.trim();
            let key = statement.to_lowercase();
            if !seen_statements.contains(&key) {
                seen_statements.insert(key);
                has_items = true;
                let scope_prefix = if !f.scope.is_empty() && f.scope != "global" {
                    format!("({}): ", f.scope)
                } else {
                    "".to_string()
                };
                let reason_suffix = match &f.context_reason {
                    Some(r) if !r.trim().is_empty() => format!(" *(Reason: {})*", r.trim()),
                    _ => "".to_string(),
                };
                lines.push(format!("- {}{}{}", scope_prefix, statement, reason_suffix));
            }
        }

        if !has_items {
            lines.push("- *(None recorded yet)*".to_string());
        }
        lines.push("".to_string());
    }

    let mut rendered = format!("{}\n", lines.join("\n").trim());

    if let Some(limit) = max_words {
        let words: Vec<&str> = rendered.split_whitespace().collect();
        if words.len() > limit {
            rendered = format!("{}\n", words[..limit].join(" ").trim());
        }
    }

    clean_markdown_experience(&rendered)
}

/// Query active facts from SQLite and rebuild the master EXPERIENCE.md file.
pub fn rebuild_master_experience(
    db_path: Option<&Path>,
    max_words: Option<usize>,
) -> Result<(usize, String), rusqlite::Error> {
    config::ensure_directories().map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(e),
        )
    })?;

    let conn = db::get_connection(db_path)?;
    let active_facts = db::get_active_facts(&conn, None)?;
    let rendered_md = render_active_experience_markdown(&active_facts, max_words);

    let master_file = config::get_master_experience_file();
    let _ = fs::write(master_file, &rendered_md);

    Ok((active_facts.len(), rendered_md))
}
