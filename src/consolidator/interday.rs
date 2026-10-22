use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use crate::config;
use crate::db;
use crate::llm_client::LLMClient;
use crate::renderer;
use crate::consolidator::prompt_template;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationMetadata {
    pub status: String,
    pub date: String,
    pub max_words: usize,
    pub active_facts_count: usize,
    pub pending_facts_count: usize,
    pub pending_fact_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationPreparePayload {
    pub metadata: ConsolidationMetadata,
    pub system_prompt: String,
    pub user_prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationResultPayload {
    pub updated_experience_markdown: String,
    #[serde(default)]
    pub processed_pending_ids: Vec<i64>,
    #[serde(default)]
    pub superseded_fact_ids: Vec<i64>,
    #[serde(default)]
    pub superseded_facts: Vec<serde_json::Value>,
    #[serde(default = "default_summary")]
    pub summary_of_changes: String,
}

fn default_summary() -> String {
    "Consolidation applied.".to_string()
}

pub fn strip_markdown_fences(input: &str) -> &str {
    let mut s = input.trim();
    if s.starts_with("```") {
        if let Some(first_nl) = s.find('\n') {
            s = &s[first_nl + 1..];
        } else {
            s = s.trim_start_matches('`');
        }
        if s.ends_with("```") {
            s = &s[..s.len() - 3];
        }
    }
    s.trim()
}

pub struct InterdayConsolidator {
    pub llm_client: Option<Box<dyn LLMClient>>,
    pub home_dir: Option<PathBuf>,
}

impl InterdayConsolidator {
    pub fn new(llm_client: Option<Box<dyn LLMClient>>) -> Self {
        Self {
            llm_client,
            home_dir: None,
        }
    }

    pub fn with_home(llm_client: Option<Box<dyn LLMClient>>, home_dir: Option<PathBuf>) -> Self {
        Self {
            llm_client,
            home_dir,
        }
    }

    pub fn master_experience_file(&self) -> PathBuf {
        self.home_dir
            .as_ref()
            .map(|h| h.join("EXPERIENCE.md"))
            .unwrap_or_else(config::get_master_experience_file)
    }

    pub fn build_consolidation_prompt(
        &self,
        conn: &rusqlite::Connection,
        target_date: Option<&str>,
        max_words: Option<usize>,
        template_path: Option<&Path>,
    ) -> Result<(Option<String>, Option<String>, ConsolidationMetadata), Box<dyn std::error::Error>> {
        let cfg = config::load_config();
        let limit_words = max_words.unwrap_or(cfg.max_experience_words);

        let master_file = self.master_experience_file();
        let master_content = if master_file.exists() {
            fs::read_to_string(&master_file).unwrap_or_else(|_| "# Master AI Experience\n\n*(Initial empty state)*".to_string())
        } else {
            "# Master AI Experience\n\n*(Initial empty state)*".to_string()
        };

        let active_facts = db::get_active_facts(conn, None)?;
        let pending_facts = db::get_pending_facts(conn, None)?;

        let now_str = target_date
            .map(|s| s.to_string())
            .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d").to_string());

        let pending_ids: Vec<i64> = pending_facts.iter().filter_map(|f| f.id).collect();

        if active_facts.is_empty() && pending_facts.is_empty() {
            let meta = ConsolidationMetadata {
                status: "noop".to_string(),
                date: now_str,
                max_words: limit_words,
                active_facts_count: 0,
                pending_facts_count: 0,
                pending_fact_ids: Vec::new(),
            };
            return Ok((None, None, meta));
        }

        let active_summary = if active_facts.is_empty() {
            "- *(None active)*".to_string()
        } else {
            active_facts
                .iter()
                .map(|f| {
                    let reason = match &f.context_reason {
                        Some(r) if !r.trim().is_empty() => format!(" (Reason: {})", r.trim()),
                        _ => String::new(),
                    };
                    format!(
                        "- [ID:{}] [{}] ({}): {}{}",
                        f.id.unwrap_or(0),
                        f.category.as_str(),
                        f.scope,
                        f.rule_statement,
                        reason
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        let pending_summary = if pending_facts.is_empty() {
            "- *(None pending)*".to_string()
        } else {
            pending_facts
                .iter()
                .map(|f| {
                    let reason = match &f.context_reason {
                        Some(r) if !r.trim().is_empty() => format!(" (Reason: {})", r.trim()),
                        _ => String::new(),
                    };
                    format!(
                        "- [ID:{}] [{}] ({}): {}{}",
                        f.id.unwrap_or(0),
                        f.category.as_str(),
                        f.scope,
                        f.rule_statement,
                        reason
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        let template = prompt_template::get_consolidation_template(template_path, self.home_dir.as_deref())?;
        let system_prompt = template.replace("{max_words}", &limit_words.to_string());

        let user_prompt = format!(
            "=== CURRENT MASTER EXPERIENCE.MD ===\n{}\n\n=== ACTIVE CONSOLIDATED FACTS ({}) ===\n{}\n\n=== PENDING NEW INTRADAY FACTS TO CONSOLIDATE ({}) ===\n{}",
            master_content.trim(),
            active_facts.len(),
            active_summary,
            pending_facts.len(),
            pending_summary
        );

        let metadata = ConsolidationMetadata {
            status: "ready".to_string(),
            date: now_str,
            max_words: limit_words,
            active_facts_count: active_facts.len(),
            pending_facts_count: pending_facts.len(),
            pending_fact_ids: pending_ids,
        };

        Ok((Some(system_prompt), Some(user_prompt), metadata))
    }

    pub fn apply_consolidation_result(
        &self,
        conn: &mut rusqlite::Connection,
        consolidation_data: &str,
        engine_type: &str,
    ) -> Result<(PathBuf, String), Box<dyn std::error::Error>> {
        let content = if Path::new(consolidation_data).exists() {
            fs::read_to_string(consolidation_data)?
        } else {
            consolidation_data.to_string()
        };

        let cleaned_json = strip_markdown_fences(&content);
        let payload: ConsolidationResultPayload = serde_json::from_str(cleaned_json)
            .map_err(|e| format!("Failed to parse consolidation JSON payload: {}", e))?;

        if payload.updated_experience_markdown.trim().is_empty() {
            return Err("Consolidation data did not contain valid 'updated_experience_markdown'.".into());
        }

        // Clean markdown to guarantee zero token waste
        let cleaned_md = renderer::clean_markdown_experience(&payload.updated_experience_markdown);

        // Write EXPERIENCE.md
        let master_path = self.master_experience_file();
        if self.home_dir.is_none() {
            config::ensure_directories()?;
        } else if let Some(parent) = master_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(&master_path, &cleaned_md)?;

        // Execute SQLite transaction
        let tx = conn.transaction()?;

        // 1. Supersede explicitly identified active facts
        for sid in &payload.superseded_fact_ids {
            tx.execute(
                "UPDATE memory_facts SET status = 'superseded', updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                rusqlite::params![sid],
            )?;
        }

        // 2. Support legacy superseded_facts with older_rule matching
        for item in &payload.superseded_facts {
            if let Some(older_rule) = item.get("older_rule").and_then(|r| r.as_str()) {
                let pattern = format!("%{}%", older_rule.trim().to_lowercase());
                tx.execute(
                    "UPDATE memory_facts SET status = 'superseded', updated_at = CURRENT_TIMESTAMP WHERE LOWER(rule_statement) LIKE ? AND status = 'active'",
                    rusqlite::params![pattern],
                )?;
            }
        }

        // 3. Mark pending facts consolidated
        let mut processed_ids = payload.processed_pending_ids.clone();
        if processed_ids.is_empty() {
            let mut stmt = tx.prepare("SELECT id FROM memory_facts WHERE is_consolidated = 0")?;
            let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
            for r in rows {
                processed_ids.push(r?);
            }
        }

        if !processed_ids.is_empty() {
            let placeholders = processed_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let sql = format!(
                "UPDATE memory_facts SET is_consolidated = 1, updated_at = CURRENT_TIMESTAMP WHERE id IN ({})",
                placeholders
            );
            let params_iter = rusqlite::params_from_iter(processed_ids.iter());
            tx.execute(&sql, params_iter)?;
        }

        // 4. Record audit in consolidations table
        let facts_superseded = payload.superseded_fact_ids.len() + payload.superseded_facts.len();
        tx.execute(
            "INSERT INTO consolidations (engine_type, facts_processed, facts_superseded, summary_of_changes) VALUES (?, ?, ?, ?)",
            rusqlite::params![
                engine_type,
                processed_ids.len() as i64,
                facts_superseded as i64,
                payload.summary_of_changes,
            ],
        )?;

        tx.commit()?;

        Ok((master_path, payload.summary_of_changes))
    }

    pub fn consolidate(
        &self,
        target_date: Option<&str>,
        max_words: Option<usize>,
        template_path: Option<&Path>,
        engine_type_override: Option<&str>,
        db_path: Option<&Path>,
    ) -> Result<(PathBuf, String), Box<dyn std::error::Error>> {
        let client = self
            .llm_client
            .as_ref()
            .ok_or("Consolidation requires an LLM client. Pass --llm '<cmd>' or configure default via 'experienced-llms setup --llm \"<cmd>\"'.")?;

        if self.home_dir.is_none() {
            config::ensure_directories()?;
        }
        let mut conn = db::get_connection(db_path)?;

        let (sys_prompt_opt, user_prompt_opt, _meta) = self.build_consolidation_prompt(
            &conn,
            target_date,
            max_words,
            template_path,
        )?;

        let sys_prompt = match sys_prompt_opt {
            Some(s) => s,
            None => {
                return Ok((
                    self.master_experience_file(),
                    "No active or pending facts to consolidate.".to_string(),
                ));
            }
        };
        let user_prompt = user_prompt_opt.unwrap_or_default();

        let raw_response = client.generate(&sys_prompt, &user_prompt, true)?;
        if raw_response.trim().is_empty() {
            return Err("LLM returned empty consolidation response. Aborting without mutating state.".into());
        }

        let engine_type = engine_type_override.unwrap_or("llm");
        self.apply_consolidation_result(&mut conn, &raw_response, engine_type)
    }
}
