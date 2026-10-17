use std::process;

fn main() {
    if let Err(e) = experienced_llms::cli::run_cli() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
