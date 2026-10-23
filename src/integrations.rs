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

#[derive(Debug, Clone)]
pub struct DetectedAgentSkill {
    pub name: &'static str,
    pub path: PathBuf,
}

/// Automatically detects installed AI agent environments on the host machine.
pub fn detect_agent_skill_dirs() -> Vec<DetectedAgentSkill> {
    let mut detected = Vec::new();
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return detected,
    };

    // 1. Google Antigravity / Gemini: ~/.gemini
    let gemini_base = home.join(".gemini");
    if gemini_base.is_dir() {
        detected.push(DetectedAgentSkill {
            name: "Google Antigravity / Gemini",
            path: gemini_base.join("config").join("skills").join("experienced-llms"),
        });
    }

    // 2. Claude Code: ~/.claude
    let claude_base = home.join(".claude");
    if claude_base.is_dir() {
        detected.push(DetectedAgentSkill {
            name: "Claude Code",
            path: claude_base.join("skills").join("experienced-llms"),
        });
    }

    // 3. Cursor: ~/.cursor
    let cursor_base = home.join(".cursor");
    if cursor_base.is_dir() {
        detected.push(DetectedAgentSkill {
            name: "Cursor",
            path: cursor_base.join("skills").join("experienced-llms"),
        });
    }

    // 4. Windsurf / Codeium: ~/.codeium
    let codeium_base = home.join(".codeium");
    if codeium_base.is_dir() {
        detected.push(DetectedAgentSkill {
            name: "Windsurf",
            path: codeium_base.join("windsurf").join("skills").join("experienced-llms"),
        });
    }

    detected
}

/// Prompts the user to choose or confirm the agent skill installation directory.
/// Automatically detects existing agent folders and asks user for confirmation.
pub fn prompt_user_for_skill_dir() -> io::Result<Option<PathBuf>> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let detected = detect_agent_skill_dirs();

    // Case 1: Exactly one agent folder detected -> ask user if they want to install there
    if detected.len() == 1 {
        let agent = &detected[0];
        println!("\n[+] Detected {} agent environment:", agent.name);
        println!("    Target directory: {}", agent.path.display());
        print!("Install agent skill here? [Y/n/custom/skip] (default: Y): ");
        let _ = io::stdout().flush();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            let trimmed = input.trim().to_lowercase();
            if trimmed.is_empty() || trimmed == "y" || trimmed == "yes" {
                return Ok(Some(agent.path.clone()));
            } else if trimmed == "s" || trimmed == "skip" {
                println!("[*] Skipped agent skill installation.");
                return Ok(None);
            } else if trimmed.contains('/') || trimmed.contains('\\') {
                return Ok(Some(crate::config::resolve_path(input.trim())));
            }
            // If "n", "no", "custom", fall through to full options menu
        } else {
            return Ok(Some(agent.path.clone()));
        }
    }

    // Case 2: Multiple detected, or none detected, or user declined single detected
    let default_options = [
        ("Google Antigravity / Gemini", home.join(".gemini").join("config").join("skills").join("experienced-llms")),
        ("Claude Code", home.join(".claude").join("skills").join("experienced-llms")),
        ("Cursor", home.join(".cursor").join("skills").join("experienced-llms")),
    ];

    println!("\nWhere should the agent skill (SKILL.md) be installed?");
    for (i, (name, path)) in default_options.iter().enumerate() {
        let is_detected = detected.iter().any(|d| d.path == *path);
        let tag = if is_detected { " [DETECTED]" } else { "" };
        println!("  [{}] {} ({}){}", i + 1, name, path.display(), tag);
    }
    println!("  [4] Custom directory path");
    println!("  [s] Skip skill installation");

    let default_idx = if !detected.is_empty() {
        default_options.iter().position(|(_, p)| *p == detected[0].path).map(|i| i + 1).unwrap_or(1)
    } else {
        1
    };

    print!("Select an option [1-4, s] (default: {}): ", default_idx);
    let _ = io::stdout().flush();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        let choice = input.trim();
        match choice {
            "" => Ok(Some(default_options[default_idx - 1].1.clone())),
            "1" => Ok(Some(default_options[0].1.clone())),
            "2" => Ok(Some(default_options[1].1.clone())),
            "3" => Ok(Some(default_options[2].1.clone())),
            "4" | "c" | "custom" => {
                print!("Enter custom directory path: ");
                let _ = io::stdout().flush();
                let mut custom = String::new();
                if io::stdin().read_line(&mut custom).is_ok() && !custom.trim().is_empty() {
                    let resolved = crate::config::resolve_path(custom.trim());
                    Ok(Some(resolved))
                } else {
                    println!("[*] No path entered, defaulting to: {}", default_options[default_idx - 1].1.display());
                    Ok(Some(default_options[default_idx - 1].1.clone()))
                }
            }
            "s" | "S" | "skip" | "none" => {
                println!("[*] Skipped agent skill installation.");
                Ok(None)
            }
            other => {
                if other.contains('/') || other.contains('\\') {
                    let resolved = crate::config::resolve_path(other);
                    Ok(Some(resolved))
                } else {
                    println!("[*] Unrecognized choice '{}', defaulting to: {}", other, default_options[default_idx - 1].1.display());
                    Ok(Some(default_options[default_idx - 1].1.clone()))
                }
            }
        }
    } else {
        Ok(Some(default_options[default_idx - 1].1.clone()))
    }
}

/// Ensures the agent skill file exists in the target directory.
/// If target_dir is None, checks configured skill_dir, environment variable, or prompts user interactively.
/// If it exists and differs, prompts the user to overwrite or skip.
/// If overwritten, automatically creates a timestamped backup first.
pub fn ensure_skill_with_prompt(
    target_dir: Option<&Path>,
    auto_yes: bool,
) -> io::Result<Option<PathBuf>> {
    let dest_dir = if let Some(p) = target_dir {
        Some(p.to_path_buf())
    } else if let Some(configured) = crate::config::get_configured_skill_dir() {
        Some(configured)
    } else if auto_yes || !io::stdin().is_terminal() {
        Some(get_default_skill_dir())
    } else {
        let chosen = prompt_user_for_skill_dir()?;
        if let Some(ref d) = chosen {
            let _ = crate::config::save_configured_skill_dir(d);
        }
        chosen
    };

    let dest_dir = match dest_dir {
        Some(d) => d,
        None => return Ok(None),
    };

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

/// Install/sync canonical agent skill using the embedded SKILL.md asset
pub fn install_antigravity(target_dir: Option<&Path>) -> Result<String, Box<dyn std::error::Error>> {
    ensure_skill_with_prompt(target_dir, true)?;
    let dest_dir = target_dir
        .map(|p| p.to_path_buf())
        .or_else(crate::config::get_configured_skill_dir)
        .unwrap_or_else(get_default_skill_dir);
    Ok(format!("Agent skill installed: {}", dest_dir.join("SKILL.md").display()))
}

/// Installs canonical agent skill. Modifying workspace project files (CLAUDE.md, AGENTS.md,
/// .cursorrules, copilot-instructions.md) has been completely discontinued.
pub fn auto_detect_and_install_all(_workspace: Option<&Path>) -> Vec<String> {
    let mut results = Vec::new();
    if let Ok(_) = ensure_skill_with_prompt(None, true) {
        let skill_dir = crate::config::get_configured_skill_dir().unwrap_or_else(get_default_skill_dir);
        let skill_file = skill_dir.join("SKILL.md");
        results.push(format!("[+] Installed agent skill: {}", skill_file.display()));
    }
    results
}
