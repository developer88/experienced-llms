from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
from typing import Optional, List

class FactCategory(str, Enum):
    USER_PREFERENCE = "user_preference"
    TECHNICAL_DECISION = "technical_decision"
    MISTAKE_CORRECTION = "mistake_correction"
    PROJECT_GOTCHA = "project_gotcha"

class FactStatus(str, Enum):
    ACTIVE = "active"
    SUPERSEDED = "superseded"
    USER_VERIFIED = "user_verified"
    ARCHIVED = "archived"

@dataclass
class MemoryFact:
    category: FactCategory
    rule_statement: str
    context_reason: Optional[str] = None
    scope: str = "global"
    confidence: float = 1.0
    status: FactStatus = FactStatus.ACTIVE
    superseded_by: Optional[int] = None
    session_id: Optional[str] = None
    id: Optional[int] = None
    created_at: Optional[datetime] = None
    updated_at: Optional[datetime] = None

@dataclass
class Skill:
    key: str
    file_path: str
    role_description: str
    rules_summary: str
    token_count: int = 0
    status: str = "active"
    created_at: Optional[datetime] = None
    updated_at: Optional[datetime] = None

@dataclass
class SessionExtractResult:
    session_id: str
    facts: List[MemoryFact] = field(default_factory=list)
    raw_markdown: str = ""
