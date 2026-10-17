use std::fs;
use std::path::Path;

use crate::config;

pub const DEFAULT_CONSOLIDATION_TEMPLATE: &str = r##"You are the master cognitive consolidation engine for Experienced LLMs.
Your job is to merge new pending intraday facts with active memory and the current EXPERIENCE.md into a single, authoritative, high-density EXPERIENCE.md document.

CANONICAL SECTIONS (Maintain these exact 5 headers in this order):
# Master AI Experience
## 1. User Directives & Preferences
## 2. Defensive Safeguards & Heuristics
## 3. Project-Specific Constraints
## 4. Anti-Patterns & Critical Gotchas
## 5. Mistake Corrections & Fixes

CRITICAL CONSOLIDATION RULES:
1. Semantic Deduplication: Merge redundant, overlapping, or paraphrased points into sharp, concise directives.
2. Contradiction Resolution: If a pending rule invalidates, corrects, or replaces an older active rule, mark the older rule's ID in superseded_fact_ids.
3. Maximum Word Count: Keep the entire output lean and operational (under {max_words} words).
4. Zero Token Waste: Absolutely ZERO HTML comments (no <!-- ... -->), zero generation timestamps, zero conversational preamble or markdown code fence wrappers in the JSON string.

YOU MUST RETURN A VALID JSON OBJECT MATCHING THIS SCHEMA:
{
  "updated_experience_markdown": "# Master AI Experience\n\n## 1. User Directives & Preferences\n...",
  "processed_pending_ids": [list of pending fact IDs integrated],
  "superseded_fact_ids": [list of older active fact IDs superseded],
  "summary_of_changes": "1-2 sentences summarizing changes made"
}
"##;

pub fn get_consolidation_template(
    custom_path: Option<&Path>,
    home_dir: Option<&Path>,
) -> String {
    if let Some(cp) = custom_path {
        if cp.exists() {
            if let Ok(content) = fs::read_to_string(cp) {
                return content;
            }
        }
    }

    let home = home_dir
        .map(|p| p.to_path_buf())
        .unwrap_or_else(config::get_experienced_home);
    let user_custom = home.join("consolidation_prompt.txt");
    if user_custom.exists() {
        if let Ok(content) = fs::read_to_string(&user_custom) {
            return content;
        }
    }

    DEFAULT_CONSOLIDATION_TEMPLATE.to_string()
}
