-- Schema for Experienced LLMs Memory Engine (1.0.0rc3 Clean Slate)

-- 1. Full-Stack Migration Ledger
CREATE TABLE IF NOT EXISTS schema_migrations (
    version TEXT PRIMARY KEY,
    applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 2. Sessions Ledger
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    source TEXT DEFAULT 'chat',
    raw_path TEXT
);

-- 3. Master Memory Facts Ledger
CREATE TABLE IF NOT EXISTS memory_facts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT REFERENCES sessions(id) ON DELETE SET NULL,
    category TEXT NOT NULL CHECK(category IN ('defensive_heuristic', 'user_preference', 'technical_decision', 'mistake_correction', 'project_gotcha')),
    scope TEXT DEFAULT 'global',
    rule_statement TEXT NOT NULL,
    context_reason TEXT,
    confidence REAL DEFAULT 1.0,
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'superseded', 'user_verified', 'archived')),
    is_consolidated INTEGER NOT NULL DEFAULT 0,
    superseded_by INTEGER REFERENCES memory_facts(id) ON DELETE SET NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_memory_facts_scope_status ON memory_facts(scope, status);
CREATE INDEX IF NOT EXISTS idx_memory_facts_consolidation ON memory_facts(is_consolidated, status);
CREATE INDEX IF NOT EXISTS idx_memory_facts_category ON memory_facts(category);

-- 4. Consolidation Audit Ledger
CREATE TABLE IF NOT EXISTS consolidations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    consolidated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    engine_type TEXT NOT NULL,
    facts_processed INTEGER NOT NULL,
    facts_superseded INTEGER NOT NULL,
    summary_of_changes TEXT
);
