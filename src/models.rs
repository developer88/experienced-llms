use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactCategory {
    DefensiveHeuristic,
    UserPreference,
    TechnicalDecision,
    MistakeCorrection,
    ProjectGotcha,
}

impl FactCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DefensiveHeuristic => "defensive_heuristic",
            Self::UserPreference => "user_preference",
            Self::TechnicalDecision => "technical_decision",
            Self::MistakeCorrection => "mistake_correction",
            Self::ProjectGotcha => "project_gotcha",
        }
    }
}

impl fmt::Display for FactCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseEnumError(pub String);

impl fmt::Display for ParseEnumError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unknown enum value: {}", self.0)
    }
}

impl std::error::Error for ParseEnumError {}

impl FromStr for FactCategory {
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "defensive_heuristic" => Ok(Self::DefensiveHeuristic),
            "user_preference" => Ok(Self::UserPreference),
            "technical_decision" => Ok(Self::TechnicalDecision),
            "mistake_correction" => Ok(Self::MistakeCorrection),
            "project_gotcha" => Ok(Self::ProjectGotcha),
            other => Err(ParseEnumError(other.to_string())),
        }
    }
}

impl rusqlite::ToSql for FactCategory {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::from(self.as_str()))
    }
}

impl rusqlite::types::FromSql for FactCategory {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let text = value.as_str()?;
        Self::from_str(text).map_err(|e| rusqlite::types::FromSqlError::Other(Box::new(e)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactStatus {
    Active,
    Superseded,
    UserVerified,
    Archived,
}

impl FactStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Superseded => "superseded",
            Self::UserVerified => "user_verified",
            Self::Archived => "archived",
        }
    }
}

impl fmt::Display for FactStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for FactStatus {
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "active" => Ok(Self::Active),
            "superseded" => Ok(Self::Superseded),
            "user_verified" => Ok(Self::UserVerified),
            "archived" => Ok(Self::Archived),
            other => Err(ParseEnumError(other.to_string())),
        }
    }
}

impl rusqlite::ToSql for FactStatus {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::from(self.as_str()))
    }
}

impl rusqlite::types::FromSql for FactStatus {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let text = value.as_str()?;
        Self::from_str(text).map_err(|e| rusqlite::types::FromSqlError::Other(Box::new(e)))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryFact {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub category: FactCategory,
    #[serde(default = "default_scope")]
    pub scope: String,
    pub rule_statement: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_reason: Option<String>,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
    #[serde(default = "default_status")]
    pub status: FactStatus,
    #[serde(default)]
    pub is_consolidated: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

fn default_scope() -> String {
    "global".to_string()
}

fn default_confidence() -> f64 {
    1.0
}

fn default_status() -> FactStatus {
    FactStatus::Active
}

impl MemoryFact {
    pub fn new(category: FactCategory, rule_statement: impl Into<String>) -> Self {
        Self {
            id: None,
            session_id: None,
            category,
            scope: default_scope(),
            rule_statement: rule_statement.into(),
            context_reason: None,
            confidence: 1.0,
            status: FactStatus::Active,
            is_consolidated: 0,
            superseded_by: None,
            created_at: None,
            updated_at: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsolidationAudit {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consolidated_at: Option<String>,
    pub engine_type: String,
    pub facts_processed: i64,
    pub facts_superseded: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_of_changes: Option<String>,
}

impl ConsolidationAudit {
    pub fn new(
        engine_type: impl Into<String>,
        facts_processed: i64,
        facts_superseded: i64,
        summary: Option<String>,
    ) -> Self {
        Self {
            id: None,
            consolidated_at: None,
            engine_type: engine_type.into(),
            facts_processed,
            facts_superseded,
            summary_of_changes: summary,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillFile {
    pub key: String,
    pub file_path: String,
    pub role_description: String,
    pub rules_summary: String,
    #[serde(default)]
    pub token_count: i64,
    #[serde(default = "default_skill_status")]
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

fn default_skill_status() -> String {
    "active".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionExtractResult {
    pub session_id: String,
    #[serde(default)]
    pub facts: Vec<MemoryFact>,
    #[serde(default)]
    pub raw_markdown: String,
}
