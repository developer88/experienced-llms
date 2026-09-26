"""Universal Agent Integrations: Install and configure Experienced LLMs for Antigravity, Claude, Copilot, Cursor, Pi, and generic agents."""

import os
from pathlib import Path
from typing import Dict, List, Tuple, Optional

import src.config as config

UNIVERSAL_INSTRUCTION_BLOCK = """<!-- BEGIN EXPERIENCED-LLMS -->
## Autonomous Memory & Operational Wisdom
1. At the start of your session, inspect your accumulated master experience:
   Read: `~/.experienced-llms/EXPERIENCE.md`
2. Strictly obey all User Directives, Project Constraints, and Defensive Heuristics defined there.
3. Proactively record operational wisdom, defensive safeguards (e.g., verifying `.env` before git commits), hidden gotchas, or user directives:
   Run: `experienced-llms log "<rule statement>" --reason "<root cause or why>" --category [defensive_heuristic|technical_decision|project_gotcha|user_preference]`
   (If shell tool is unavailable, emit a `<session_learning>` block).
<!-- END EXPERIENCED-LLMS -->
"""

def install_antigravity(target_dir: Optional[Path] = None) -> Tuple[bool, str]:
    dest = target_dir or (Path.home() / ".gemini" / "config" / "skills" / "experienced-llms")
    dest.mkdir(parents=True, exist_ok=True)
    src_skill = config.BASE_DIR / "skill" / "SKILL.md"
    skill_file = dest / "SKILL.md"
    if src_skill.exists():
        with open(src_skill, "r", encoding="utf-8") as f_in:
            content = f_in.read()
    else:
        content = UNIVERSAL_INSTRUCTION_BLOCK
    with open(skill_file, "w", encoding="utf-8") as f_out:
        f_out.write(content)
    return True, f"Antigravity skill installed: {skill_file}"

def install_claude(workspace: Optional[Path] = None) -> Tuple[bool, str]:
    """Installs/updates CLAUDE.md in user home (~/.claude/CLAUDE.md) or workspace."""
    dest_dir = workspace or (Path.home() / ".claude")
    dest_dir.mkdir(parents=True, exist_ok=True)
    target = dest_dir / "CLAUDE.md"
    _append_or_replace_block(target, UNIVERSAL_INSTRUCTION_BLOCK)
    return True, f"Claude instructions installed: {target}"

def install_copilot(workspace: Optional[Path] = None) -> Tuple[bool, str]:
    """Installs .github/copilot-instructions.md in workspace or user profile."""
    base = workspace or Path.cwd()
    dest_dir = base / ".github"
    dest_dir.mkdir(parents=True, exist_ok=True)
    target = dest_dir / "copilot-instructions.md"
    _append_or_replace_block(target, UNIVERSAL_INSTRUCTION_BLOCK)
    return True, f"GitHub Copilot instructions installed: {target}"

def install_cursor(workspace: Optional[Path] = None) -> Tuple[bool, str]:
    """Installs/updates .cursorrules in workspace or home."""
    base = workspace or Path.cwd()
    target = base / ".cursorrules"
    _append_or_replace_block(target, UNIVERSAL_INSTRUCTION_BLOCK)
    return True, f"Cursor rules installed: {target}"

def install_pi_generic(workspace: Optional[Path] = None) -> Tuple[bool, str]:
    """Installs universal AGENTS.md in workspace or user home."""
    base = workspace or Path.cwd()
    target = base / "AGENTS.md"
    _append_or_replace_block(target, UNIVERSAL_INSTRUCTION_BLOCK)
    return True, f"Universal AGENTS.md (for Pi and generic agents) installed: {target}"

def auto_detect_and_install_all(workspace: Optional[Path] = None) -> List[str]:
    """Detects available agent frameworks and configures all of them."""
    results = []

    # 1. Antigravity
    gemini_home = Path.home() / ".gemini"
    if gemini_home.exists():
        ok, msg = install_antigravity()
        results.append(f"[+] {msg}")

    # 2. Claude (Claude Code / Desktop)
    claude_home = Path.home() / ".claude"
    if claude_home.exists():
        ok, msg = install_claude()
        results.append(f"[+] {msg}")
    else:
        # Create global Claude instructions
        ok, msg = install_claude()
        results.append(f"[+] {msg}")

    # 3. GitHub Copilot
    cwd = workspace or Path.cwd()
    if (cwd / ".github").exists() or (cwd / ".git").exists():
        ok, msg = install_copilot(cwd)
        results.append(f"[+] {msg}")

    # 4. Universal AGENTS.md (Pi and open agent standard)
    ok, msg = install_pi_generic(cwd)
    results.append(f"[+] {msg}")

    return results

def _append_or_replace_block(file_path: Path, block: str):
    file_path.parent.mkdir(parents=True, exist_ok=True)
    if not file_path.exists():
        with open(file_path, "w", encoding="utf-8") as f:
            f.write(block.strip() + "\n")
        return

    with open(file_path, "r", encoding="utf-8") as f:
        existing = f.read()

    start_marker = "<!-- BEGIN EXPERIENCED-LLMS -->"
    end_marker = "<!-- END EXPERIENCED-LLMS -->"

    if start_marker in existing and end_marker in existing:
        before = existing.split(start_marker)[0]
        after = existing.split(end_marker)[1]
        new_content = before.rstrip() + "\n\n" + block.strip() + "\n" + after.lstrip()
    else:
        new_content = existing.rstrip() + "\n\n" + block.strip() + "\n"

    with open(file_path, "w", encoding="utf-8") as f:
        f.write(new_content)
