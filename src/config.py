import os
import sys
import re
from pathlib import Path

BASE_DIR = Path(__file__).resolve().parent.parent

def resolve_path(p: str | Path) -> Path:
    """Resolve Windows path to WSL path if running inside Linux."""
    p_str = str(p)
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

# Database
DEFAULT_DB = BASE_DIR / "data" / "memory.db"
DB_PATH = resolve_path(os.getenv("EXPERIENCED_LLMS_DB", str(DEFAULT_DB)))

# Storage roots for Markdown memories
RAW_OBSIDIAN = os.getenv(
    "OBSIDIAN_MEMORY_DIR",
    r"C:\Users\andrey\Google Drive Streaming\My Drive\obsidian\personal_gdrive\Project implementations\LLMs experience\memory"
)
DEFAULT_OBSIDIAN_DIR = resolve_path(RAW_OBSIDIAN)

# If Obsidian directory is accessible, use it, otherwise fall back to local workspace memory dir
MEMORY_ROOT = DEFAULT_OBSIDIAN_DIR if safe_exists(DEFAULT_OBSIDIAN_DIR.parent) else (BASE_DIR / "memory")
SESSIONS_DIR = MEMORY_ROOT / "sessions"
DAILY_DIR = MEMORY_ROOT / "daily"
SKILLS_DIR = BASE_DIR / "skills"
CORE_PROFILE_FILE = MEMORY_ROOT / "core_profile.md"

# LLM Providers
OLLAMA_BASE_URL = os.getenv("OLLAMA_BASE_URL", "http://localhost:11434")
OLLAMA_MODEL = os.getenv("OLLAMA_MODEL", "qwen2.5-coder:7b")

GEMINI_API_KEY = os.getenv("GEMINI_API_KEY", "")
GEMINI_MODEL = os.getenv("GEMINI_MODEL", "gemini-1.5-flash") # or gemini-2.0-flash / 3.8

# Token and size bounds
CORE_PROFILE_MAX_WORDS = 150 # approx 150-200 tokens
JIT_SKILL_MAX_WORDS = 220    # approx 200-300 tokens

def ensure_directories():
    """Ensure that all required directory trees exist."""
    DB_PATH.parent.mkdir(parents=True, exist_ok=True)
    SESSIONS_DIR.mkdir(parents=True, exist_ok=True)
    DAILY_DIR.mkdir(parents=True, exist_ok=True)
    SKILLS_DIR.mkdir(parents=True, exist_ok=True)
