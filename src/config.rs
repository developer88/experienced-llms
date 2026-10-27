use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};

pub const DEFAULT_MAX_EXPERIENCE_WORDS: usize = 1200;

pub const STARTER_EXPERIENCE_TEMPLATE: &str = r#"# Master AI Experience

## 1. User Directives & Preferences
- (None recorded yet)

## 2. Defensive Safeguards & Heuristics
- (None recorded yet)

## 3. Project-Specific Constraints
- (None recorded yet)

## 4. Anti-Patterns & Critical Gotchas
- (None recorded yet)

## 5. Mistake Corrections & Fixes
- (None recorded yet)
"#;

pub const DEFAULT_CONSOLIDATION_TEMPLATE: &str = r##"You are the master cognitive consolidation engine for Experienced LLMs.
Your job is to merge new pending intraday facts with active memory and the current EXPERIENCE.md into a single, authoritative, high-density EXPERIENCE.md document.

CANONICAL SECTIONS (Maintain these standard headers in this order):
# Master AI Experience
## 1. User Directives & Preferences
## 2. Defensive Safeguards & Heuristics
## 3. Anti-Patterns & Critical Gotchas
## 4. Mistake Corrections & Fixes

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

pub fn resolve_path(p: impl AsRef<Path>) -> PathBuf {
    let p_ref = p.as_ref();
    let p_str = p_ref.to_string_lossy();

    // Expand ~
    let expanded = if p_str.starts_with('~') {
        if let Some(home) = dirs::home_dir() {
            let without_tilde = p_str.trim_start_matches('~').trim_start_matches(['/', '\\']);
            home.join(without_tilde)
        } else {
            p_ref.to_path_buf()
        }
    } else {
        p_ref.to_path_buf()
    };

    // If running under Linux and path has Windows drive letter (e.g. C:\ or C:/)
    #[cfg(target_os = "linux")]
    {
        let s = expanded.to_string_lossy();
        if s.len() >= 3 && s.chars().nth(1) == Some(':') {
            let drive = s.chars().next().unwrap().to_ascii_lowercase();
            let separator = s.chars().nth(2).unwrap();
            if separator == '\\' || separator == '/' {
                let rest = &s[3..].replace('\\', "/");
                return PathBuf::from(format!("/mnt/{}/{}", drive, rest));
            }
        }
    }

    expanded
}

pub fn get_experienced_home() -> PathBuf {
    if let Ok(val) = env::var("EXPERIENCED_LLMS_HOME") {
        if !val.trim().is_empty() {
            return resolve_path(val);
        }
    }
    dirs::home_dir()
        .map(|h| h.join(".experienced-llms"))
        .unwrap_or_else(|| PathBuf::from(".experienced-llms"))
}

pub fn get_db_path() -> PathBuf {
    if let Ok(val) = env::var("EXPERIENCED_LLMS_DB") {
        if !val.trim().is_empty() {
            return resolve_path(val);
        }
    }
    get_experienced_home().join("memory.db")
}

pub fn get_config_file() -> PathBuf {
    get_experienced_home().join("config.json")
}

pub fn get_master_experience_file() -> PathBuf {
    get_experienced_home().join("EXPERIENCE.md")
}

pub fn get_template_file() -> PathBuf {
    get_experienced_home().join("template.md")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default)]
    pub gemini_api_key: String,
    #[serde(default = "default_gemini_model")]
    pub gemini_model: String,
    #[serde(default)]
    pub claude_api_key: String,
    #[serde(default = "default_claude_model")]
    pub claude_model: String,
    #[serde(default)]
    pub openai_api_key: String,
    #[serde(default = "default_openai_model")]
    pub openai_model: String,
    #[serde(default = "default_openai_base_url")]
    pub openai_base_url: String,
    #[serde(default = "default_ollama_base_url")]
    pub ollama_base_url: String,
    #[serde(default = "default_ollama_model")]
    pub ollama_model: String,
    #[serde(default = "default_max_experience_words")]
    pub max_experience_words: usize,
    #[serde(default)]
    pub external_llm_cmd: Option<String>,
    #[serde(default)]
    pub skill_dir: Option<String>,
}

fn default_provider() -> String {
    "gemini".to_string()
}

fn default_gemini_model() -> String {
    "gemini-1.5-flash".to_string()
}

fn default_claude_model() -> String {
    "claude-3-5-sonnet-latest".to_string()
}

fn default_openai_model() -> String {
    "gpt-4o-mini".to_string()
}

fn default_openai_base_url() -> String {
    "https://api.openai.com/v1".to_string()
}

fn default_ollama_base_url() -> String {
    "http://localhost:11434".to_string()
}

fn default_ollama_model() -> String {
    "qwen2.5-coder:7b".to_string()
}

fn default_max_experience_words() -> usize {
    DEFAULT_MAX_EXPERIENCE_WORDS
}

impl Default for Config {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            gemini_api_key: env::var("GEMINI_API_KEY").unwrap_or_default(),
            gemini_model: env::var("GEMINI_MODEL").unwrap_or_else(|_| default_gemini_model()),
            claude_api_key: env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
            claude_model: env::var("CLAUDE_MODEL").unwrap_or_else(|_| default_claude_model()),
            openai_api_key: env::var("OPENAI_API_KEY").unwrap_or_default(),
            openai_model: env::var("OPENAI_MODEL").unwrap_or_else(|_| default_openai_model()),
            openai_base_url: env::var("OPENAI_BASE_URL").unwrap_or_else(|_| default_openai_base_url()),
            ollama_base_url: env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| default_ollama_base_url()),
            ollama_model: env::var("OLLAMA_MODEL").unwrap_or_else(|_| default_ollama_model()),
            max_experience_words: default_max_experience_words(),
            external_llm_cmd: env::var("EXPERIENCED_LLMS_EXTERNAL_CMD").ok().filter(|s| !s.trim().is_empty()),
            skill_dir: env::var("EXPERIENCED_LLMS_SKILL_DIR").ok().filter(|s| !s.trim().is_empty()),
        }
    }
}

pub fn get_configured_skill_dir() -> Option<PathBuf> {
    if let Ok(val) = env::var("EXPERIENCED_LLMS_SKILL_DIR") {
        if !val.trim().is_empty() {
            return Some(resolve_path(val));
        }
    }
    let cfg = load_config();
    cfg.skill_dir.as_ref().map(resolve_path)
}

pub fn save_configured_skill_dir(dir: &Path) -> std::io::Result<()> {
    let mut cfg = load_config();
    cfg.skill_dir = Some(dir.to_string_lossy().to_string());
    save_config(&cfg)
}

pub fn load_config() -> Config {
    let mut config = Config::default();
    let config_path = get_config_file();
    if config_path.exists() {
        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Ok(saved) = serde_json::from_str::<Config>(&content) {
                config = saved;
            }
        }
    }

    // Apply environment variable overrides if present
    if let Ok(key) = env::var("GEMINI_API_KEY") {
        if !key.trim().is_empty() {
            config.gemini_api_key = key;
        }
    }
    if let Ok(key) = env::var("ANTHROPIC_API_KEY") {
        if !key.trim().is_empty() {
            config.claude_api_key = key;
        }
    }
    if let Ok(key) = env::var("OPENAI_API_KEY") {
        if !key.trim().is_empty() {
            config.openai_api_key = key;
        }
    }
    config
}

pub fn save_config(config: &Config) -> std::io::Result<()> {
    ensure_directories()?;
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    fs::write(get_config_file(), content)?;
    // Clear any stale failure lock when configuration is updated
    let lock_path = get_experienced_home().join("consolidation.lock");
    let _ = fs::remove_file(lock_path);
    Ok(())
}

pub fn command_exists(cmd: &str) -> bool {
    #[cfg(windows)]
    let check = std::process::Command::new("where.exe").arg(cmd).output();
    #[cfg(not(windows))]
    let check = std::process::Command::new("which").arg(cmd).output();
    check.map(|o| o.status.success()).unwrap_or(false)
}

pub fn detect_available_llm_cli() -> Option<(&'static str, &'static str)> {
    if command_exists("agy") {
        Some(("agy", "agy query"))
    } else if command_exists("claude") {
        Some(("claude", "claude -p"))
    } else if command_exists("ollama") {
        Some(("ollama", "ollama run mistral"))
    } else {
        None
    }
}

pub fn ensure_llm_cmd_with_prompt(auto_yes: bool, cli_llm: Option<&str>) -> std::io::Result<Option<String>> {
    let mut cfg = load_config();

    if let Some(cmd) = cli_llm {
        let trimmed = cmd.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
            cfg.external_llm_cmd = None;
            save_config(&cfg)?;
            println!("[✓] Cleared LLM CLI command (set to null).");
            return Ok(None);
        } else {
            cfg.external_llm_cmd = Some(trimmed.to_string());
            save_config(&cfg)?;
            println!("[✓] Configured LLM CLI command: '{}'", trimmed);
            return Ok(Some(trimmed.to_string()));
        }
    }

    if auto_yes || !std::io::stdin().is_terminal() {
        return Ok(cfg.external_llm_cmd);
    }

    if let Some(ref current) = cfg.external_llm_cmd {
        println!("[?] Current LLM CLI command is: '{}'.", current);
        print!("    Keep this command? [Y/n] (or enter replacement command): ");
        std::io::stdout().flush()?;
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("y") || trimmed.eq_ignore_ascii_case("yes") {
            println!("[✓] Kept current LLM CLI command: '{}'", current);
            Ok(Some(current.clone()))
        } else if trimmed.eq_ignore_ascii_case("n") || trimmed.eq_ignore_ascii_case("no") {
            print!("    Enter new LLM CLI command (or press Enter for none): ");
            std::io::stdout().flush()?;
            let mut new_line = String::new();
            std::io::stdin().read_line(&mut new_line)?;
            let new_trimmed = new_line.trim();
            if new_trimmed.is_empty() || new_trimmed.eq_ignore_ascii_case("none") {
                cfg.external_llm_cmd = None;
                save_config(&cfg)?;
                println!("[*] Cleared LLM CLI command (set to null).");
                Ok(None)
            } else {
                cfg.external_llm_cmd = Some(new_trimmed.to_string());
                save_config(&cfg)?;
                println!("[✓] Configured LLM CLI command: '{}'", new_trimmed);
                Ok(Some(new_trimmed.to_string()))
            }
        } else {
            cfg.external_llm_cmd = Some(trimmed.to_string());
            save_config(&cfg)?;
            println!("[✓] Configured LLM CLI command: '{}'", trimmed);
            Ok(Some(trimmed.to_string()))
        }
    } else {
        if let Some((name, suggested)) = detect_available_llm_cli() {
            println!("[+] Detected '{}' in PATH.", name);
            print!("    Use '{}' for memory consolidation? [Y/n] (or enter custom command): ", suggested);
            std::io::stdout().flush()?;
            let mut line = String::new();
            std::io::stdin().read_line(&mut line)?;
            let trimmed = line.trim();

            if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("y") || trimmed.eq_ignore_ascii_case("yes") {
                cfg.external_llm_cmd = Some(suggested.to_string());
                save_config(&cfg)?;
                println!("[✓] Configured LLM CLI command: '{}'", suggested);
                Ok(Some(suggested.to_string()))
            } else if trimmed.eq_ignore_ascii_case("n") || trimmed.eq_ignore_ascii_case("no") {
                println!("[*] Skipped LLM CLI command (set to null). Note: consolidation will fail until configured.");
                Ok(None)
            } else {
                cfg.external_llm_cmd = Some(trimmed.to_string());
                save_config(&cfg)?;
                println!("[✓] Configured LLM CLI command: '{}'", trimmed);
                Ok(Some(trimmed.to_string()))
            }
        } else {
            println!("[?] No LLM CLI command configured for memory consolidation.");
            print!("    Enter command (e.g. 'agy query', 'claude -p', 'ollama run mistral') or Enter for none: ");
            std::io::stdout().flush()?;
            let mut line = String::new();
            std::io::stdin().read_line(&mut line)?;
            let trimmed = line.trim();

            if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
                println!("[*] No LLM command set (null). Note: consolidation will fail until configured.");
                Ok(None)
            } else {
                cfg.external_llm_cmd = Some(trimmed.to_string());
                save_config(&cfg)?;
                println!("[✓] Configured LLM CLI command: '{}'", trimmed);
                Ok(Some(trimmed.to_string()))
            }
        }
    }
}

pub fn ensure_template_with_prompt(home: &Path, auto_yes: bool) -> std::io::Result<Option<PathBuf>> {
    fs::create_dir_all(home)?;
    let template_file = home.join("template.md");
    if template_file.exists() {
        let existing = fs::read_to_string(&template_file)?;
        if existing.trim() == DEFAULT_CONSOLIDATION_TEMPLATE.trim() {
            println!("[✓] Consolidation template is up to date: {}", template_file.display());
            return Ok(None);
        }

        if crate::integrations::prompt_overwrite_or_skip(&template_file, auto_yes) {
            let bak = crate::integrations::backup_file(&template_file)?;
            println!("[+] Backed up existing template.md to: {}", bak.display());
            fs::write(&template_file, DEFAULT_CONSOLIDATION_TEMPLATE)?;
            println!("[+] Updated template.md to default version.");
            Ok(Some(bak))
        } else {
            println!("[*] Skipped template.md (kept existing custom template).");
            Ok(None)
        }
    } else {
        fs::write(&template_file, DEFAULT_CONSOLIDATION_TEMPLATE)?;
        println!("[+] Created default consolidation template: {}", template_file.display());
        Ok(None)
    }
}

pub fn ensure_template_in(home: &Path) -> std::io::Result<()> {
    let _ = ensure_template_with_prompt(home, false)?;
    Ok(())
}

pub fn ensure_directories() -> std::io::Result<()> {
    ensure_directories_in(&get_experienced_home())
}

pub fn ensure_directories_in(home: &Path) -> std::io::Result<()> {
    fs::create_dir_all(home)?;

    let db_path = home.join("memory.db");
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let master_exp = home.join("EXPERIENCE.md");
    if !master_exp.exists() {
        let _ = fs::write(master_exp, STARTER_EXPERIENCE_TEMPLATE);
    }
    Ok(())
}
