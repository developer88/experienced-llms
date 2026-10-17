use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "experienced-llms")]
#[command(version = "1.0.0-rc.3")]
#[command(about = "Autonomous memory & operational wisdom for AI coding agents", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize directories, SQLite database, and starter configuration
    Init {
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
    /// Show current status and database stats
    Status,
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Init { db_path }) => {
            println!("Initializing Experienced LLMs...");
            if let Err(e) = experienced_llms::config::ensure_directories() {
                eprintln!("Error creating directories: {}", e);
                process::exit(1);
            }
            match experienced_llms::db::init_db(db_path.as_deref()) {
                Ok(_) => {
                    println!("[✓] SQLite database and schema initialized successfully.");
                }
                Err(e) => {
                    eprintln!("Error initializing database: {}", e);
                    process::exit(1);
                }
            }
        }
        Some(Commands::Status) | None => {
            let home = experienced_llms::config::get_experienced_home();
            let db_path = experienced_llms::config::get_db_path();
            println!("Experienced LLMs (Rust Engine v1.0.0-rc.3)");
            println!("Home: {}", home.display());
            println!("Database: {}", db_path.display());
            if db_path.exists() {
                match experienced_llms::db::get_connection(Some(&db_path)) {
                    Ok(conn) => match experienced_llms::db::get_active_facts(&conn, None) {
                        Ok(facts) => println!("Active Facts: {}", facts.len()),
                        Err(e) => eprintln!("Error querying facts: {}", e),
                    },
                    Err(e) => eprintln!("Error opening database: {}", e),
                }
            } else {
                println!("Database not initialized. Run 'experienced-llms init'.");
            }
        }
    }
}
