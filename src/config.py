import json
import os
import sys
import re
from pathlib import Path
from typing import Dict, Any

BASE_DIR = Path(__file__).resolve().parent.parent

def resolve_path(p: str | Path) -> Path:
    """Resolve Windows path to WSL path if running inside Linux, and expand user home."""
    p_str = str(p)
    if p_str.startswith("~"):
        p_str = str(Path(p_str).expanduser())
    if sys.platform.startswith("linux") and re.match(r"^[a-zA-Z]:[/\\]", p_str):
        drive = p_str[0].lower()
        rest = p_str[2:].replace("\\", "/")
        return Path(f"/mnt/{drive}{rest}")
    return Path(p_str)

def safe_exists(p: Path) -> bool:
    try:
        return p.exists()
    except (OSError, IOError):
        return False

# Global Home for Experienced LLMs
DEFAULT_HOME = Path.home() / ".experienced-llms"
EXPERIENCED_HOME = resolve_path(os.getenv("EXPERIENCED_LLMS_HOME", str(DEFAULT_HOME)))

CONFIG_FILE = EXPERIENCED_HOME / "config.json"
MASTER_EXPERIENCE_FILE = EXPERIENCED_HOME / "EXPERIENCE.md"
RAW_DAYS_DIR = EXPERIENCED_HOME / "raw_days"
SKILLS_DIR = EXPERIENCED_HOME / "skills"
DB_PATH = resolve_path(os.getenv("EXPERIENCED_LLMS_DB", str(EXPERIENCED_HOME / "memory.db")))

# Legacy workspace paths for backward compatibility and tests
MEMORY_ROOT = EXPERIENCED_HOME
SESSIONS_DIR = EXPERIENCED_HOME / "sessions"
DAILY_DIR = EXPERIENCED_HOME / "daily"
CORE_PROFILE_FILE = EXPERIENCED_HOME / "core_profile.md"

# Token bounds
CORE_PROFILE_MAX_WORDS = 150
JIT_SKILL_MAX_WORDS = 220

DEFAULT_CONFIG: Dict[str, Any] = {
    "provider": "gemini",
    "gemini_api_key": os.getenv("GEMINI_API_KEY", ""),
    "gemini_model": os.getenv("GEMINI_MODEL", "gemini-1.5-flash"),
    "claude_api_key": os.getenv("ANTHROPIC_API_KEY", ""),
    "claude_model": os.getenv("CLAUDE_MODEL", "claude-3-5-sonnet-latest"),
    "openai_api_key": os.getenv("OPENAI_API_KEY", ""),
    "openai_model": os.getenv("OPENAI_MODEL", "gpt-4o-mini"),
    "openai_base_url": os.getenv("OPENAI_BASE_URL", "https://api.openai.com/v1"),
    "ollama_base_url": os.getenv("OLLAMA_BASE_URL", "http://localhost:11434"),
    "ollama_model": os.getenv("OLLAMA_MODEL", "qwen2.5-coder:7b"),
    "schedule_time": "02:00"
}

def load_config() -> Dict[str, Any]:
    """Load configuration from config.json, merged with defaults."""
    cfg = DEFAULT_CONFIG.copy()
    if CONFIG_FILE.exists():
        try:
            with open(CONFIG_FILE, "r", encoding="utf-8") as f:
                saved = json.load(f)
                cfg.update(saved)
        except Exception:
            pass
    # Environment variable overrides
    if os.getenv("GEMINI_API_KEY"):
        cfg["gemini_api_key"] = os.getenv("GEMINI_API_KEY")
    if os.getenv("ANTHROPIC_API_KEY"):
        cfg["claude_api_key"] = os.getenv("ANTHROPIC_API_KEY")
    if os.getenv("OPENAI_API_KEY"):
        cfg["openai_api_key"] = os.getenv("OPENAI_API_KEY")
    return cfg

def save_config(cfg: Dict[str, Any]):
    """Save configuration to config.json."""
    ensure_directories()
    with open(CONFIG_FILE, "w", encoding="utf-8") as f:
        json.dump(cfg, f, indent=2)

STARTER_EXPERIENCE_TEMPLATE = """# Master AI Experience

## 1. User Directives & Preferences
- (None recorded yet)

## 2. Defensive Safeguards & Heuristics
- (None recorded yet)

## 3. Project-Specific Constraints
- (None recorded yet)

## 4. Anti-Patterns & Critical Gotchas
- (None recorded yet)
"""

def ensure_directories():
    """Ensure that all required directory trees exist and starter experience file is initialized."""
    EXPERIENCED_HOME.mkdir(parents=True, exist_ok=True)
    RAW_DAYS_DIR.mkdir(parents=True, exist_ok=True)
    SKILLS_DIR.mkdir(parents=True, exist_ok=True)
    SESSIONS_DIR.mkdir(parents=True, exist_ok=True)
    DAILY_DIR.mkdir(parents=True, exist_ok=True)
    DB_PATH.parent.mkdir(parents=True, exist_ok=True)
    if not MASTER_EXPERIENCE_FILE.exists():
        try:
            with open(MASTER_EXPERIENCE_FILE, "w", encoding="utf-8") as f:
                f.write(STARTER_EXPERIENCE_TEMPLATE)
        except Exception:
            pass
