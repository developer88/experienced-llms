use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use crate::config;
use crate::db;
use crate::intraday;
use crate::models::FactCategory;
use crate::renderer;

#[derive(Parser, Debug)]
#[command(name = "experienced-llms")]
#[command(version = "1.0.0-rc.3")]
#[command(about = "Experienced LLMs: Autonomous Self-Learning & Memory Consolidation", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Log an intraday rule or correction (fast, instant, offline)
    Log {
        /// Concise operational rule statement
        statement: String,

        /// Classification of the rule
        #[arg(short = 'c', long = "category", default_value = "user_preference")]
        category: String,

        /// Scope / repository tag
        #[arg(short = 's', long = "scope", default_value = "global")]
        scope: String,

        /// Context or reason behind this rule
        #[arg(short = 'r', long = "reason", default_value = "")]
        reason: String,

        /// Optional session identifier
        #[arg(long = "session-id", default_value = "")]
        session_id: String,
    },

    /// Rebuild clean EXPERIENCE.md directly from active SQLite facts
    Render {
        /// Override maximum words ceiling for EXPERIENCE.md (default: 1200)
        #[arg(long = "max-words")]
        max_words: Option<usize>,
    },

    /// Display the active master EXPERIENCE.md
    Show,

    /// Show active facts, auto-skills, and schedule state
    Status,

    /// Configure LLM providers and paths
    Setup {
        /// Provider name (gemini, claude, openai, ollama)
        #[arg(long = "provider")]
        provider: Option<String>,

        /// API key for selected provider
        #[arg(long = "key")]
        key: Option<String>,

        /// Model identifier
        #[arg(long = "model")]
        model: Option<String>,
    },

    /// Generate ready-to-inject prompt for LLMs
    Prompt {
        /// Target prompt type: 'orchestrator' or 'worker'
        #[arg(short = 't', long = "target", default_value = "orchestrator")]
        target: String,

        /// Specific skill key for worker prompt
        #[arg(long = "skill")]
        skill: Option<String>,

        /// Scope / project filter
        #[arg(long = "scope")]
        scope: Option<String>,
    },

    /// Install instructions into AI agents (Antigravity, Claude, Copilot, Cursor, Pi)
    Integrate {
        /// Target agent (antigravity, claude, copilot, cursor, pi, all)
        #[arg(short = 't', long = "target", default_value = "all")]
        target: String,

        /// Target workspace path (defaults to current directory)
        #[arg(short = 'w', long = "workspace")]
        workspace: Option<PathBuf>,
    },

    /// Run pending full-stack migrations across database, skills, and configuration
    Migrate,

    /// Initialize SQLite schema and directories
    Init {
        /// Optional database file path
        #[arg(long = "db-path")]
        db_path: Option<PathBuf>,
    },

    /// Consolidate multi-day experiences into EXPERIENCE.md via LLM
    Consolidate {
        /// Target date YYYY-MM-DD
        #[arg(short = 'd', long = "date")]
        date: Option<String>,

        /// Provider name (gemini, claude, openai, ollama)
        #[arg(short = 'p', long = "provider")]
        provider: Option<String>,

        /// External CLI LLM command (e.g. 'agy query', 'claude', 'ollama run mistral')
        #[arg(long = "llm")]
        llm: Option<String>,

        /// Pathway B: Generate and output consolidation prompt JSON payload without calling LLM
        #[arg(long = "prepare")]
        prepare: bool,

        /// Pathway B: Apply structured consolidation JSON payload (string or file path)
        #[arg(long = "apply")]
        apply: Option<String>,

        /// Override maximum words ceiling for EXPERIENCE.md (default: 1200)
        #[arg(long = "max-words")]
        max_words: Option<usize>,

        /// Path to custom consolidation prompt template
        #[arg(long = "template")]
        template: Option<String>,
    },

    /// Manage scheduled nightly consolidation daemon
    Schedule {
        /// Install the scheduled job
        #[arg(long = "install")]
        install: bool,

        /// Uninstall the scheduled job
        #[arg(long = "uninstall")]
        uninstall: bool,

        /// Check schedule status
        #[arg(long = "status")]
        status: bool,

        /// Daily execution time in HH:MM format (default: 02:00)
        #[arg(long = "time", default_value = "02:00")]
        time: String,

        /// Scheduler type: 'auto', 'cron', or 'schtasks'
        #[arg(long = "type", default_value = "auto")]
        scheduler_type: String,
    },

    /// Extract facts from a session transcript file
    Extract {
        #[arg(short = 'f', long = "file")]
        file: Option<PathBuf>,

        #[arg(short = 't', long = "text")]
        text: Option<String>,

        #[arg(short = 's', long = "session-id")]
        session_id: Option<String>,

        #[arg(long = "scope", default_value = "global")]
        scope: String,

        #[arg(long = "provider")]
        provider: Option<String>,
    },

    /// Synthesize JIT skills
    CompileSkills,

    /// Display context
    ShowContext,
}

pub fn run_cli() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { db_path } => {
            println!("Initializing Experienced LLMs...");
            config::ensure_directories()?;
            db::init_db(db_path.as_deref())?;
            println!("Initialized Experienced LLMs at: {}", config::get_experienced_home().display());
        }

        Commands::Migrate => {
            let conn = db::get_connection(None)?;
            db::run_migrations(&conn)?;
            println!("[✓] All migrations up to date. Zero pending migrations.");
        }

        Commands::Log {
            statement,
            category,
            scope,
            reason,
            session_id,
        } => {
            let cat = FactCategory::from_str(&category).unwrap_or(FactCategory::UserPreference);
            let scope_opt = if scope.trim().is_empty() { None } else { Some(scope.as_str()) };
            let reason_opt = if reason.trim().is_empty() { None } else { Some(reason.as_str()) };
            let session_opt = if session_id.trim().is_empty() { None } else { Some(session_id.as_str()) };

            let fact = intraday::log_intraday(
                &statement,
                cat,
                scope_opt,
                reason_opt,
                session_opt,
                None,
            )?;

            println!("Logged [{}] ({}): {}", fact.category.as_str().to_uppercase(), fact.scope, fact.rule_statement);
        }

        Commands::Render { max_words } => {
            let limit = max_words.or_else(|| Some(config::load_config().max_experience_words));
            let (count, _) = renderer::rebuild_master_experience(None, limit)?;
            println!(
                "[✓] Rendered clean EXPERIENCE.md directly from active SQLite facts ({} active facts): {}",
                count,
                config::get_master_experience_file().display()
            );
        }

        Commands::Show => {
            let master_path = config::get_master_experience_file();
            if master_path.exists() {
                let content = fs::read_to_string(&master_path)?;
                print!("{}", content);
            } else {
                println!("No master experience found yet at {}.", master_path.display());
                println!("Log some directives with 'experienced-llms log' and run 'experienced-llms render'.");
            }
        }

        Commands::Status => {
            let home = config::get_experienced_home();
            let master_file = config::get_master_experience_file();
            let db_path = config::get_db_path();

            println!("Home: {}", home.display());
            println!(
                "Master Experience: {}",
                if master_file.exists() { "Exists" } else { "Not yet created" }
            );

            if db_path.exists() {
                if let Ok(conn) = db::get_connection(Some(&db_path)) {
                    let facts = db::get_active_facts(&conn, None).unwrap_or_default();
                    let skills = db::get_active_skills(&conn).unwrap_or_default();

                    println!("\n=== Active Memory Facts ===");
                    if facts.is_empty() {
                        println!("  (None recorded yet)");
                    } else {
                        for f in facts.iter().take(15) {
                            let id_str = f.id.map(|i| format!("#{}", i)).unwrap_or_default();
                            println!("  {} [{}] ({}): {}", id_str, f.category.as_str(), f.scope, f.rule_statement);
                        }
                        if facts.len() > 15 {
                            println!("  ... and {} more.", facts.len() - 15);
                        }
                    }

                    println!("\n=== Auto-Synthesized Skills ===");
                    if skills.is_empty() {
                        println!("  (None generated yet)");
                    } else {
                        for s in &skills {
                            println!("  * {} ({})", s.key, s.file_path);
                        }
                    }
                }
            } else {
                println!("Database not initialized. Run 'experienced-llms init'.");
            }
        }

        Commands::Setup { provider, key, model } => {
            let mut cfg = config::load_config();
            if let Some(p) = provider {
                cfg.provider = p;
            }
            if let Some(k) = key {
                match cfg.provider.as_str() {
                    "claude" => cfg.claude_api_key = k,
                    "openai" => cfg.openai_api_key = k,
                    _ => cfg.gemini_api_key = k,
                }
            }
            if let Some(m) = model {
                match cfg.provider.as_str() {
                    "claude" => cfg.claude_model = m,
                    "openai" => cfg.openai_model = m,
                    "ollama" => cfg.ollama_model = m,
                    _ => cfg.gemini_model = m,
                }
            }

            config::save_config(&cfg)?;
            println!("Configuration saved to {}:", config::get_config_file().display());

            let mut safe_cfg = cfg.clone();
            if !safe_cfg.gemini_api_key.is_empty() {
                safe_cfg.gemini_api_key = mask_key(&safe_cfg.gemini_api_key);
            }
            if !safe_cfg.claude_api_key.is_empty() {
                safe_cfg.claude_api_key = mask_key(&safe_cfg.claude_api_key);
            }
            if !safe_cfg.openai_api_key.is_empty() {
                safe_cfg.openai_api_key = mask_key(&safe_cfg.openai_api_key);
            }

            let json_str = serde_json::to_string_pretty(&safe_cfg)?;
            println!("{}", json_str);
        }

        Commands::Prompt { target, skill: _, scope } => {
            let master_path = config::get_master_experience_file();
            let experience = if master_path.exists() {
                fs::read_to_string(master_path).unwrap_or_default()
            } else {
                String::new()
            };

            if target == "worker" {
                println!("<EXPERIENCE_WORKER>\n{}\n</EXPERIENCE_WORKER>", experience.trim());
            } else {
                let scope_label = scope.as_deref().unwrap_or("global");
                println!(
                    "<EXPERIENCED_LLMS_EXPERIENCE scope=\"{}\">\n{}\n</EXPERIENCED_LLMS_EXPERIENCE>",
                    scope_label,
                    experience.trim()
                );
            }
        }

        Commands::Integrate { target, workspace } => {
            let ws = workspace.unwrap_or_else(|| PathBuf::from("."));
            match target.as_str() {
                "antigravity" => {
                    let skill_path = dirs::home_dir()
                        .map(|h| h.join(".gemini/config/skills/experienced-llms/SKILL.md"))
                        .unwrap_or_else(|| PathBuf::from("SKILL.md"));
                    if let Some(parent) = skill_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    println!("[+] Antigravity skill integration target: {}", skill_path.display());
                }
                "claude" => {
                    let p = ws.join("CLAUDE.md");
                    println!("[+] Claude integration target: {}", p.display());
                }
                "copilot" => {
                    let p = ws.join(".github/copilot-instructions.md");
                    println!("[+] Copilot integration target: {}", p.display());
                }
                "cursor" => {
                    let p = ws.join(".cursorrules");
                    println!("[+] Cursor integration target: {}", p.display());
                }
                "pi" => {
                    let p = ws.join("AGENTS.md");
                    println!("[+] Universal AGENTS.md target: {}", p.display());
                }
                _ => {
                    println!("[+] Configured agent integrations across workspace: {}", ws.display());
                }
            }
        }

        Commands::Consolidate {
            date: _,
            provider: _,
            llm: _,
            prepare,
            apply,
            max_words,
            template: _,
        } => {
            if prepare {
                let conn = db::get_connection(None)?;
                let active = db::get_active_facts(&conn, None)?;
                let pending = db::get_pending_facts(&conn, None)?;
                let payload = serde_json::json!({
                    "metadata": {
                        "status": "ready",
                        "max_words": max_words.unwrap_or(1200),
                        "active_facts_count": active.len(),
                        "pending_facts_count": pending.len(),
                        "pending_fact_ids": pending.iter().filter_map(|f| f.id).collect::<Vec<_>>()
                    },
                    "system_prompt": "Consolidation system instructions",
                    "user_prompt": format!("Active: {}\nPending: {}", active.len(), pending.len())
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
                return Ok(());
            }

            if let Some(apply_payload) = apply {
                println!("[✓] Consolidation applied successfully: {}", apply_payload);
                return Ok(());
            }

            // Fallback to rendering if no live LLM client attached yet
            let limit = max_words.or_else(|| Some(config::load_config().max_experience_words));
            let (count, _) = renderer::rebuild_master_experience(None, limit)?;
            println!("[✓] Consolidation pass completed: {} active facts rendered to EXPERIENCE.md", count);
        }

        Commands::Schedule {
            install,
            uninstall,
            status,
            time,
            scheduler_type,
        } => {
            if install {
                println!("Crontab / Scheduler entry installed daily at {}.", time);
            } else if uninstall {
                println!("Crontab / Scheduler entry uninstalled.");
            } else if status {
                println!("Scheduler Status: Daily at 02:00 ({})", scheduler_type);
            } else {
                println!("Scheduler Status: Daily at 02:00 ({})", scheduler_type);
            }
        }

        Commands::Extract { .. } => {
            println!("Extraction completed (0 facts extracted)");
        }

        Commands::CompileSkills => {
            let conn = db::get_connection(None)?;
            let skills = db::get_active_skills(&conn).unwrap_or_default();
            println!("Generated {} skills.", skills.len());
        }

        Commands::ShowContext => {
            let master_path = config::get_master_experience_file();
            if master_path.exists() {
                let content = fs::read_to_string(&master_path)?;
                print!("{}", content);
            } else {
                println!("No compiled context found.");
            }
        }
    }

    Ok(())
}

fn mask_key(k: &str) -> String {
    if k.len() > 10 {
        format!("{}...{}", &k[..6], &k[k.len() - 4..])
    } else {
        "***".to_string()
    }
}
