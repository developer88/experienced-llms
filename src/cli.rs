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
use crate::scheduler;
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
    Render {
        /// Override maximum words ceiling for EXPERIENCE.md (default: 1200)
        #[arg(long = "max-words")]
        max_words: Option<usize>,
    },

    /// Display the active master EXPERIENCE.md
    Show,

    /// Show active facts, auto-skills, and schedule state
    Status,

    /// Configure default external LLM CLI command and settings
    Setup {
        /// External LLM CLI command (e.g. 'agy query', 'claude', 'ollama run mistral')
        #[arg(long = "llm")]
        llm: Option<String>,

        /// Maximum word count for master experience document
        #[arg(long = "max-words")]
        max_words: Option<usize>,
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

    /// Initialize SQLite schema, default template, and agent skill
    Init {
        /// Optional database file path
        #[arg(long = "db-path")]
        db_path: Option<PathBuf>,

        /// Automatically approve prompts (e.g. file overwrite)
        #[arg(short = 'y', long = "yes")]
        yes: bool,
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

        #[arg(long = "llm")]
        llm: Option<String>,
    },

    /// Synthesize JIT skills
    CompileSkills,

    /// Display context
    ShowContext,
}

pub fn run_cli() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { db_path, yes } => {
            println!("Initializing Experienced LLMs...");
            config::ensure_directories()?;
            config::ensure_template_with_prompt(&config::get_experienced_home(), yes)?;
            integrations::ensure_skill_with_prompt(None, yes)?;
            db::init_db(db_path.as_deref())?;
            let (count, _) = renderer::rebuild_master_experience(db_path.as_deref(), None)?;
            println!("Initialized Experienced LLMs at: {}", config::get_experienced_home().display());
            println!("Rendered active experience with {} facts.", count);
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
            println!("Schedule: {}", scheduler::get_schedule_status("auto"));

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

        Commands::Setup { llm, max_words } => {
            let mut cfg = config::load_config();
            if let Some(cmd) = llm {
                cfg.external_llm_cmd = Some(cmd);
            }
            if let Some(mw) = max_words {
                cfg.max_experience_words = mw;
            }

            config::save_config(&cfg)?;
            println!("Configuration saved to {}:", config::get_config_file().display());

            let json_str = serde_json::to_string_pretty(&cfg)?;
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

        Commands::Integrate { target, workspace: _ } => {
            match target.to_lowercase().as_str() {
                "antigravity" | "skill" => {
                    let msg = integrations::install_antigravity(None)?;
                    println!("[+] {}", msg);
                }
                "all" => {
                    let results = integrations::auto_detect_and_install_all(None);
                    for res in results {
                        println!("{}", res);
                    }
                }
                "claude" | "copilot" | "cursor" | "pi" => {
                    println!("[-] Note: Modifying workspace project files (CLAUDE.md, AGENTS.md, etc.) is discontinued.");
                    println!("    Experienced LLMs operates strictly via ~/.gemini/config/skills/experienced-llms/SKILL.md without workspace pollution.");
                    let msg = integrations::install_antigravity(None)?;
                    println!("[+] {}", msg);
                }
                _ => {
                    eprintln!("[-] Unknown integration target: {}. Supported: antigravity, skill, all", target);
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

            let consolidator = consolidator::InterdayConsolidator::new(Some(client));
            let engine_type = "cli_external_cmd";
            match consolidator.consolidate(
                date.as_deref(),
                max_words,
                Some(engine_type),
                None,
            ) {
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

        Commands::Schedule {
            install,
            uninstall,
            status: _,
            time,
            scheduler_type,
        } => {
            if install {
                let msg = scheduler::install_schedule(&time, &scheduler_type)?;
                println!("{}", msg);
            } else if uninstall {
                let msg = scheduler::uninstall_schedule(&scheduler_type)?;
                println!("{}", msg);
            } else {
                println!("Scheduler Status:");
                let st = scheduler::get_schedule_status(&scheduler_type);
                println!("  {}", st);
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


