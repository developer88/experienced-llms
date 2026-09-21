-- Schema for Experienced LLMs Memory Engine

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    source TEXT DEFAULT 'chat',
    raw_path TEXT
);

CREATE TABLE IF NOT EXISTS memory_facts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT REFERENCES sessions(id) ON DELETE SET NULL,
    category TEXT NOT NULL CHECK(category IN ('user_preference', 'technical_decision', 'mistake_correction', 'project_gotcha')),
    scope TEXT DEFAULT 'global',
    rule_statement TEXT NOT NULL,
    context_reason TEXT,
    confidence REAL DEFAULT 1.0,
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'superseded', 'user_verified', 'archived')),
    superseded_by INTEGER REFERENCES memory_facts(id) ON DELETE SET NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_memory_facts_scope_status ON memory_facts(scope, status);
CREATE INDEX IF NOT EXISTS idx_memory_facts_category ON memory_facts(category);

CREATE TABLE IF NOT EXISTS skills (
    key TEXT PRIMARY KEY,
    file_path TEXT NOT NULL,
    role_description TEXT NOT NULL,
    rules_summary TEXT NOT NULL,
    token_count INTEGER DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'draft', 'archived')),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS daily_consolidations (
    date TEXT PRIMARY KEY, -- YYYY-MM-DD
    file_path TEXT NOT NULL,
    facts_count INTEGER DEFAULT 0,
    skills_updated INTEGER DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
