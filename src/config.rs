use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_MAX_EXPERIENCE_WORDS: usize = 1200;
pub const DEFAULT_SCHEDULE_TIME: &str = "02:00";

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
    #[serde(default = "default_schedule_time")]
    pub schedule_time: String,
    #[serde(default = "default_max_experience_words")]
    pub max_experience_words: usize,
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

fn default_schedule_time() -> String {
    DEFAULT_SCHEDULE_TIME.to_string()
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
            schedule_time: default_schedule_time(),
            max_experience_words: default_max_experience_words(),
        }
    }
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
    fs::write(get_config_file(), content)
}

pub fn ensure_directories() -> std::io::Result<()> {
    let home = get_experienced_home();
    fs::create_dir_all(&home)?;

    let db_path = get_db_path();
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let master_exp = get_master_experience_file();
    if !master_exp.exists() {
        let _ = fs::write(master_exp, STARTER_EXPERIENCE_TEMPLATE);
    }
    Ok(())
}
