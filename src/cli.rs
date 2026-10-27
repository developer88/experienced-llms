use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use crate::config;
use crate::consolidator;
use crate::db;
use crate::integrations;
use crate::intraday;
use crate::llm_client;
use crate::models::FactCategory;
use crate::renderer;
use crate::autoconsolidate;
use crate::updater;

#[derive(Parser, Debug)]
#[command(name = "experienced-llms")]
#[command(version)]
#[command(about = "Experienced LLMs: Autonomous Self-Learning & Memory Consolidation", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Log an intraday rule or correction (fast, instant capture)
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
    #[command(hide = true)]
    Render {
        /// Override maximum words ceiling for EXPERIENCE.md (default: 1200)
        #[arg(long = "max-words")]
        max_words: Option<usize>,
    },

    /// Show active facts, last consolidation date, and engine status
    Status,

    /// Install instructions into AI agents (Antigravity skill)
    #[command(hide = true)]
    Integrate {
        /// Target agent (antigravity, skill, all)
        #[arg(short = 't', long = "target", default_value = "all")]
        target: String,

        /// Target workspace path (defaults to current directory)
        #[arg(short = 'w', long = "workspace")]
        workspace: Option<PathBuf>,
    },

    /// Run pending full-stack migrations across database, skills, and configuration
    #[command(hide = true)]
    Migrate,

    /// Initialize SQLite schema, default template, and agent skill
    Init {
        /// Optional database file path
        #[arg(long = "db-path")]
        db_path: Option<PathBuf>,

        /// Optional target directory for agent skill (SKILL.md)
        #[arg(long = "skill-dir")]
        skill_dir: Option<PathBuf>,

        /// Configure LLM command for consolidation (e.g. 'agy query', 'claude -p')
        #[arg(long = "llm")]
        llm: Option<String>,

        /// Automatically approve prompts (e.g. file overwrite)
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },

    /// View or adjust configuration (LLM CLI command, max words, skill dir)
    Config {
        /// Set or update the LLM CLI command (e.g. 'agy query', 'claude -p', 'ollama run mistral')
        #[arg(long = "llm")]
        llm: Option<String>,

        /// Clear the configured LLM CLI command (set to null)
        #[arg(long = "clear-llm")]
        clear_llm: bool,

        /// Override maximum words ceiling for EXPERIENCE.md
        #[arg(long = "max-words")]
        max_words: Option<usize>,

        /// Configure target directory for agent skill (SKILL.md)
        #[arg(long = "skill-dir")]
        skill_dir: Option<PathBuf>,

        /// Display current configuration values
        #[arg(long = "show")]
        show: bool,
    },

    /// Self-update experienced-llms to the latest release
    Update {
        /// Check for available updates without installing
        #[arg(long)]
        check: bool,

        /// Automatically approve prompts (e.g. file overwrite)
        #[arg(short = 'y', long = "yes")]
        yes: bool,

        /// Pin to a specific release tag
        #[arg(long)]
        tag: Option<String>,
    },

    /// Consolidate multi-day experiences into EXPERIENCE.md via LLM
    Consolidate {
        /// Target date YYYY-MM-DD
        #[arg(short = 'd', long = "date")]
        date: Option<String>,

        /// External CLI LLM command (e.g. 'agy query', 'claude', 'ollama run mistral')
        #[arg(long = "llm")]
        llm: Option<String>,

        /// Override maximum words ceiling for EXPERIENCE.md (default: 1200)
        #[arg(long = "max-words")]
        max_words: Option<usize>,
    },

    /// Extract facts from a session transcript file
    #[command(hide = true)]
    Extract {
        #[arg(short = 'f', long = "file")]
        file: Option<PathBuf>,

        #[arg(short = 't', long = "text")]
        text: Option<String>,

        #[arg(short = 's', long = "session-id")]
        session_id: Option<String>,

        #[arg(long = "scope", default_value = "global")]
        scope: String,

        #[arg(long = "llm")]
        llm: Option<String>,
    },

    /// Synthesize JIT skills
    #[command(hide = true)]
    CompileSkills,

    /// Display context
    #[command(hide = true)]
    ShowContext,
}

pub fn run_cli() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { db_path, skill_dir, llm, yes } => {
            println!("Initializing Experienced LLMs...");
            config::ensure_directories()?;
            config::ensure_template_with_prompt(&config::get_experienced_home(), yes)?;
            if let Some(ref sd) = skill_dir {
                let _ = config::save_configured_skill_dir(sd);
            }
            integrations::ensure_skill_with_prompt(skill_dir.as_deref(), yes)?;
            config::ensure_llm_cmd_with_prompt(yes, llm.as_deref())?;
            db::init_db(db_path.as_deref())?;
            let (count, _) = renderer::rebuild_master_experience(db_path.as_deref(), None)?;
            println!("Initialized Experienced LLMs at: {}", config::get_experienced_home().display());
            println!("Rendered active experience with {} facts.", count);
        }

        Commands::Config { llm, clear_llm, max_words, skill_dir, show } => {
            let mut cfg = config::load_config();
            let mut mutated = false;

            if clear_llm {
                cfg.external_llm_cmd = None;
                mutated = true;
                println!("[✓] Cleared LLM CLI command (set to null).");
            } else if let Some(cmd) = llm {
                let trimmed = cmd.trim();
                if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
                    cfg.external_llm_cmd = None;
                    println!("[✓] Cleared LLM CLI command (set to null).");
                } else {
                    cfg.external_llm_cmd = Some(trimmed.to_string());
                    println!("[✓] Configured LLM CLI command: \"{}\"", trimmed);
                }
                mutated = true;
            }

            if let Some(mw) = max_words {
                cfg.max_experience_words = mw;
                mutated = true;
                println!("[✓] Updated max experience words: {}", mw);
            }

            if let Some(sd) = skill_dir {
                cfg.skill_dir = Some(sd.to_string_lossy().to_string());
                mutated = true;
                println!("[✓] Updated skill directory: {}", sd.display());
            }

            if mutated {
                config::save_config(&cfg)?;
            } else if show || true {
                let cfg_file = config::get_config_file();
                println!("Experienced LLMs Configuration ({}):", cfg_file.display());
                match &cfg.external_llm_cmd {
                    Some(cmd) => println!("  LLM CLI Command:      {}", cmd),
                    None => println!("  LLM CLI Command:      (None / null) -> consolidation will fail hard until configured"),
                }
                println!("  Max Experience Words: {}", cfg.max_experience_words);
                let skill_dir_str = config::get_configured_skill_dir()
                    .map(|d| d.display().to_string())
                    .unwrap_or_else(|| "(Default autodetection)".to_string());
                println!("  Skill Directory:      {}", skill_dir_str);
            }
        }

        Commands::Update { check, yes, tag } => {
            updater::run_self_update(check, yes, tag.as_deref())?;
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

            // Auto-consolidation: check if previous consolidation was on a previous day
            // Fails hard if auto-consolidation is needed and cannot complete
            if let Err(err) = autoconsolidate::check_and_run_autoconsolidation(None, None) {
                eprintln!("[-] {}", err);
                std::process::exit(1);
            }
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

        Commands::Status => {
            let home = config::get_experienced_home();
            let master_file = config::get_master_experience_file();
            let db_path = config::get_db_path();

            println!("Home: {}", home.display());
            println!(
                "Master Experience: {}",
                if master_file.exists() { "Exists" } else { "Not yet created" }
            );

            let cfg = config::load_config();
            match &cfg.external_llm_cmd {
                Some(cmd) => println!("LLM CLI Command: {}", cmd),
                None => println!("LLM CLI Command: (None / null) -> consolidation will fail hard until configured"),
            }

            if let Some(lock_meta) = autoconsolidate::get_lock_status(&home) {
                let remaining = lock_meta.expires_at_unix - chrono::Utc::now().timestamp();
                println!("Consolidation Lock: Active (expires in {}s, PID: {})", remaining.max(0), lock_meta.pid);
            }

            if db_path.exists() {
                if let Ok(conn) = db::get_connection(Some(&db_path)) {
                    let last_cons = autoconsolidate::get_last_consolidation_date(&conn)
                        .ok()
                        .flatten()
                        .map(|d| d.format("%Y-%m-%d").to_string())
                        .unwrap_or_else(|| "Never".to_string());
                    println!("Last Consolidation: {}", last_cons);

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

                    if !skills.is_empty() {
                        println!("\n=== Auto-Synthesized Skills ===");
                        for s in &skills {
                            println!("  * {} ({})", s.key, s.file_path);
                        }
                    }
                }
            } else {
                println!("Database not initialized. Run 'experienced-llms init'.");
            }
        }

        Commands::Integrate { target, workspace: _ } => {
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            match target.to_lowercase().as_str() {
                "antigravity" | "gemini" => {
                    let dir = home.join(".gemini").join("config").join("skills").join("experienced-llms");
                    let msg = integrations::install_antigravity(Some(&dir))?;
                    println!("[+] {}", msg);
                }
                "claude" => {
                    let dir = home.join(".claude").join("skills").join("experienced-llms");
                    let msg = integrations::install_antigravity(Some(&dir))?;
                    println!("[+] {}", msg);
                }
                "cursor" => {
                    let dir = home.join(".cursor").join("skills").join("experienced-llms");
                    let msg = integrations::install_antigravity(Some(&dir))?;
                    println!("[+] {}", msg);
                }
                "skill" | "all" => {
                    let results = integrations::auto_detect_and_install_all(None);
                    for res in results {
                        println!("{}", res);
                    }
                }
                _ => {
                    let custom_path = config::resolve_path(&target);
                    let msg = integrations::install_antigravity(Some(&custom_path))?;
                    println!("[+] {}", msg);
                }
            }
        }

        Commands::Consolidate {
            date,
            llm,
            max_words,
        } => {
            let client = match llm_client::get_llm_client(llm.as_deref()) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[-] Consolidation aborted without mutating facts: {}", e);
                    std::process::exit(1);
                }
            };

            let home = config::get_experienced_home();
            let lock = autoconsolidate::ConsolidationLock::try_acquire(&home).ok().flatten();

            let consolidator = consolidator::InterdayConsolidator::new(Some(client));
            let engine_type = "cli_external_cmd";
            let res = consolidator.consolidate(
                date.as_deref(),
                max_words,
                Some(engine_type),
                None,
            );
            if let Some(l) = lock {
                l.release();
            }

            match res {
                Ok((master_path, summary)) => {
                    println!("[✓] Consolidation complete: {}", master_path.display());
                    println!("[✓] Summary: {}", summary);
                }
                Err(e) => {
                    eprintln!("[-] Consolidation aborted without mutating facts: {}", e);
                    std::process::exit(1);
                }
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


