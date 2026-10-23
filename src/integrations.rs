use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

pub const EMBEDDED_SKILL_MD: &str = include_str!("../skills/experienced-llm/SKILL.md");

/// Returns the canonical global agent skill directory: ~/.gemini/config/skills/experienced-llms
pub fn get_default_skill_dir() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".gemini").join("config").join("skills").join("experienced-llms"))
        .unwrap_or_else(|| PathBuf::from(".gemini/config/skills/experienced-llms"))
}

/// Creates a timestamped backup copy: <filename>.bak_<YYYYMMDD_HHMMSS>
pub fn backup_file(path: &Path) -> io::Result<PathBuf> {
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let backup_name = format!("{}.bak_{}", file_name, timestamp);
    let backup_path = path.with_file_name(backup_name);
    fs::copy(path, &backup_path)?;
    Ok(backup_path)
}

/// Prompts user to overwrite or skip when a file already exists.
/// Returns true if user approves overwrite, false if skipped.
/// In non-interactive environments (CI, pipes), defaults to false unless auto_yes is true.
pub fn prompt_overwrite_or_skip(file_path: &Path, auto_yes: bool) -> bool {
    if auto_yes {
        return true;
    }
    if !io::stdin().is_terminal() {
        return false;
    }
    print!(
        "File exists: {}. Overwrite? [o]verwrite / [s]kip (default: skip): ",
        file_path.display()
    );
    let _ = io::stdout().flush();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        let trimmed = input.trim().to_lowercase();
        trimmed == "o" || trimmed == "overwrite" || trimmed == "y" || trimmed == "yes"
    } else {
        false
    }
}

/// Ensures the agent skill file exists in the target directory (default: ~/.gemini/config/skills/experienced-llms/SKILL.md).
/// If it exists and differs, prompts the user to overwrite or skip.
/// If overwritten, automatically creates a timestamped backup first.
pub fn ensure_skill_with_prompt(
    target_dir: Option<&Path>,
    auto_yes: bool,
) -> io::Result<Option<PathBuf>> {
    let dest_dir = target_dir
        .map(|p| p.to_path_buf())
        .unwrap_or_else(get_default_skill_dir);

    fs::create_dir_all(&dest_dir)?;
    let skill_file = dest_dir.join("SKILL.md");

    if skill_file.exists() {
        let existing = fs::read_to_string(&skill_file)?;
        if existing.trim() == EMBEDDED_SKILL_MD.trim() {
            println!("[✓] Agent skill is already up to date: {}", skill_file.display());
            return Ok(None);
        }

        if prompt_overwrite_or_skip(&skill_file, auto_yes) {
            let bak = backup_file(&skill_file)?;
            println!("[+] Backed up existing SKILL.md to: {}", bak.display());
            fs::write(&skill_file, EMBEDDED_SKILL_MD)?;
            println!("[+] Updated agent skill: {}", skill_file.display());
            Ok(Some(bak))
        } else {
            println!("[*] Skipped SKILL.md (kept existing custom skill).");
            Ok(None)
        }
    } else {
        fs::write(&skill_file, EMBEDDED_SKILL_MD)?;
        println!("[+] Installed agent skill: {}", skill_file.display());
        Ok(None)
    }
}

/// Install/sync canonical Antigravity skill using the embedded SKILL.md asset
pub fn install_antigravity(target_dir: Option<&Path>) -> Result<String, Box<dyn std::error::Error>> {
    ensure_skill_with_prompt(target_dir, true)?;
    let dest_dir = target_dir
        .map(|p| p.to_path_buf())
        .unwrap_or_else(get_default_skill_dir);
    Ok(format!("Antigravity skill installed: {}", dest_dir.join("SKILL.md").display()))
}

/// Installs canonical agent skill. Modifying workspace project files (CLAUDE.md, AGENTS.md,
/// .cursorrules, copilot-instructions.md) has been completely discontinued.
pub fn auto_detect_and_install_all(_workspace: Option<&Path>) -> Vec<String> {
    let mut results = Vec::new();
    if let Ok(_) = ensure_skill_with_prompt(None, true) {
        let skill_file = get_default_skill_dir().join("SKILL.md");
        results.push(format!("[+] Installed agent skill: {}", skill_file.display()));
    }
    results
}
