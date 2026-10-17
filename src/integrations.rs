use std::fs;
use std::path::{Path, PathBuf};

pub const UNIVERSAL_INSTRUCTION_BLOCK: &str = r#"<!-- BEGIN EXPERIENCED-LLMS -->
## Autonomous Memory & Operational Wisdom
1. At the start of your session, inspect your accumulated master experience:
   Read: `~/.experienced-llms/EXPERIENCE.md`
2. Strictly obey all User Directives, Project Constraints, and Defensive Heuristics defined there.
3. Proactively record operational wisdom, defensive safeguards (e.g., verifying `.env` before git commits), hidden gotchas, or user directives:
   Run: `experienced-llms log "<rule statement>" --reason "<root cause or why>" --category [defensive_heuristic|technical_decision|project_gotcha|user_preference]`
<!-- END EXPERIENCED-LLMS -->
"#;

pub fn append_or_replace_block(file_path: &Path, block: &str) -> std::io::Result<()> {
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)?;
    }

    if !file_path.exists() {
        fs::write(file_path, format!("{}\n", block.trim()))?;
        return Ok(());
    }

    let existing = fs::read_to_string(file_path)?;
    let start_marker = "<!-- BEGIN EXPERIENCED-LLMS -->";
    let end_marker = "<!-- END EXPERIENCED-LLMS -->";

    let new_content = if let (Some(start_idx), Some(end_idx)) = (existing.find(start_marker), existing.find(end_marker)) {
        if start_idx <= end_idx {
            let before = &existing[..start_idx];
            let after = &existing[end_idx + end_marker.len()..];
            format!("{}\n\n{}\n{}", before.trim_end(), block.trim(), after.trim_start())
        } else {
            format!("{}\n\n{}\n", existing.trim_end(), block.trim())
        }
    } else {
        format!("{}\n\n{}\n", existing.trim_end(), block.trim())
    };

    fs::write(file_path, new_content)?;
    Ok(())
}

pub fn install_antigravity(target_dir: Option<&Path>) -> Result<String, Box<dyn std::error::Error>> {
    let dest_dir = target_dir
        .map(|p| p.to_path_buf())
        .or_else(|| {
            dirs::home_dir().map(|h| h.join(".gemini").join("config").join("skills").join("experienced-llms"))
        })
        .unwrap_or_else(|| PathBuf::from(".gemini/config/skills/experienced-llms"));

    fs::create_dir_all(&dest_dir)?;
    let skill_file = dest_dir.join("SKILL.md");

    // Check if repository skills/experienced-llm/SKILL.md exists
    let src_skill = Path::new("skills").join("experienced-llm").join("SKILL.md");
    let content = if src_skill.exists() {
        fs::read_to_string(&src_skill)?
    } else {
        UNIVERSAL_INSTRUCTION_BLOCK.to_string()
    };

    fs::write(&skill_file, content)?;
    Ok(format!("Antigravity skill installed: {}", skill_file.display()))
}

pub fn install_claude(workspace: Option<&Path>) -> Result<String, Box<dyn std::error::Error>> {
    let target = if let Some(ws) = workspace {
        ws.join("CLAUDE.md")
    } else if let Some(home) = dirs::home_dir() {
        home.join(".claude").join("CLAUDE.md")
    } else {
        PathBuf::from("CLAUDE.md")
    };

    append_or_replace_block(&target, UNIVERSAL_INSTRUCTION_BLOCK)?;
    Ok(format!("Claude instructions installed: {}", target.display()))
}

pub fn install_copilot(workspace: Option<&Path>) -> Result<String, Box<dyn std::error::Error>> {
    let base = workspace.unwrap_or_else(|| Path::new("."));
    let target = base.join(".github").join("copilot-instructions.md");
    append_or_replace_block(&target, UNIVERSAL_INSTRUCTION_BLOCK)?;
    Ok(format!("GitHub Copilot instructions installed: {}", target.display()))
}

pub fn install_cursor(workspace: Option<&Path>) -> Result<String, Box<dyn std::error::Error>> {
    let base = workspace.unwrap_or_else(|| Path::new("."));
    let target = base.join(".cursorrules");
    append_or_replace_block(&target, UNIVERSAL_INSTRUCTION_BLOCK)?;
    Ok(format!("Cursor rules installed: {}", target.display()))
}

pub fn install_pi(workspace: Option<&Path>) -> Result<String, Box<dyn std::error::Error>> {
    let base = workspace.unwrap_or_else(|| Path::new("."));
    let target = base.join("AGENTS.md");
    append_or_replace_block(&target, UNIVERSAL_INSTRUCTION_BLOCK)?;
    Ok(format!("Universal AGENTS.md (for Pi and generic agents) installed: {}", target.display()))
}

pub fn auto_detect_and_install_all(workspace: Option<&Path>) -> Vec<String> {
    let mut results = Vec::new();

    // 1. Antigravity
    if let Some(home) = dirs::home_dir() {
        let gemini_home = home.join(".gemini");
        if gemini_home.exists() {
            if let Ok(msg) = install_antigravity(None) {
                results.push(format!("[+] {}", msg));
            }
        }
    }

    // 2. Claude
    if let Ok(msg) = install_claude(workspace) {
        results.push(format!("[+] {}", msg));
    }

    // 3. GitHub Copilot
    let cwd = workspace.unwrap_or_else(|| Path::new("."));
    if cwd.join(".github").exists() || cwd.join(".git").exists() {
        if let Ok(msg) = install_copilot(Some(cwd)) {
            results.push(format!("[+] {}", msg));
        }
    }

    // 4. Universal AGENTS.md (Pi and open agent standard)
    if let Ok(msg) = install_pi(Some(cwd)) {
        results.push(format!("[+] {}", msg));
    }

    results
}
