use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

pub const DEFAULT_HON_NAME: &str = "default";
pub const DEFAULT_PROFILE_NAME: &str = "unbound";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Concept {
    #[serde(rename = "魂")]
    Hon,
    #[serde(rename = "魄")]
    Baek,
    #[serde(rename = "心")]
    Sim,
    #[serde(rename = "身")]
    Shin,
    #[serde(rename = "命")]
    Myeong,
    #[serde(rename = "怪異")]
    Kaeyi,
}

impl Concept {
    pub fn hanja(self) -> &'static str {
        match self {
            Self::Hon => "魂",
            Self::Baek => "魄",
            Self::Sim => "心",
            Self::Shin => "身",
            Self::Myeong => "命",
            Self::Kaeyi => "怪異",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AutonomyMode {
    Safe,
    Unbound,
}

impl fmt::Display for AutonomyMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Safe => write!(f, "safe"),
            Self::Unbound => write!(f, "unbound"),
        }
    }
}

impl AutonomyMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "safe" => Some(Self::Safe),
            "unbound" => Some(Self::Unbound),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PermissionSet {
    pub file_read: bool,
    pub file_write: bool,
    pub file_delete: bool,
    pub command_exec: bool,
    pub network: bool,
}

impl PermissionSet {
    pub fn for_mode(mode: AutonomyMode) -> Self {
        match mode {
            AutonomyMode::Safe => Self::safe(),
            AutonomyMode::Unbound => Self::unbound(),
        }
    }

    pub fn unbound() -> Self {
        Self {
            file_read: true,
            file_write: true,
            file_delete: true,
            command_exec: true,
            network: true,
        }
    }

    pub fn safe() -> Self {
        Self {
            file_read: true,
            file_write: true,
            file_delete: false,
            command_exec: false,
            network: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub mode: AutonomyMode,
    pub permissions: PermissionSet,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hon {
    pub id: Uuid,
    pub name: String,
    pub profile: String,
    pub profile_permissions: PermissionSet,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Baek {
    pub provider: String,
    pub model: String,
    pub base_url: String,
    pub api_key_env: String,
    pub secret_source: Option<String>,
    pub configured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sim {
    pub current_intent: Option<String>,
    pub priority: String,
    pub self_check: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shin {
    pub filesystem: bool,
    pub shell: bool,
    pub network: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Myeong {
    pub identity: String,
    pub continuity_events: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum KaeyiSeverity {
    Low,
    Warning,
    Critical,
}

impl fmt::Display for KaeyiSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Warning => write!(f, "warning"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

impl std::str::FromStr for KaeyiSeverity {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "low" | "info" | "notice" => Ok(Self::Low),
            "warning" | "warn" => Ok(Self::Warning),
            "critical" | "severe" | "fatal" => Ok(Self::Critical),
            other => Err(format!(
                "invalid 怪異 severity {other}; use low, warning, or critical"
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum KaeyiState {
    Discovered,
    Observed,
    Contained,
    Resolved,
    Attributed,
}

impl KaeyiState {
    pub fn hanja(&self) -> &'static str {
        match self {
            Self::Discovered => "發現",
            Self::Observed => "觀測",
            Self::Contained => "封印",
            Self::Resolved => "解消",
            Self::Attributed => "歸屬",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "發現" | "discovered" => Some(Self::Discovered),
            "觀測" | "observed" => Some(Self::Observed),
            "封印" | "contained" => Some(Self::Contained),
            "解消" | "resolved" => Some(Self::Resolved),
            "歸屬" | "attributed" => Some(Self::Attributed),
            _ => None,
        }
    }
}

impl fmt::Display for KaeyiState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.hanja())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Kaeyi {
    pub id: Uuid,
    pub title: String,
    pub source_kind: String,
    pub severity: KaeyiSeverity,
    pub state: KaeyiState,
    pub task_id: Option<Uuid>,
    pub evidence: String,
    pub containment_note: Option<String>,
    pub resolution_note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KaeyiScanFinding {
    pub kaeyi: Kaeyi,
    pub created: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Queued => write!(f, "queued"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub hon_id: Uuid,
    pub prompt: String,
    pub status: TaskStatus,
    pub result: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeEvent {
    pub id: Uuid,
    pub at: DateTime<Utc>,
    pub concept: Concept,
    pub kind: String,
    pub message: String,
    pub task_id: Option<Uuid>,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: Uuid,
    pub task_id: Option<Uuid>,
    pub tool: String,
    pub input: String,
    pub output: String,
    pub ok: bool,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderUsage {
    pub id: Uuid,
    pub task_id: Option<Uuid>,
    pub provider: String,
    pub model: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectState {
    pub hons: Vec<Hon>,
    pub profiles: Vec<Profile>,
    pub baek: Baek,
    pub sim: Sim,
    pub shin: Shin,
    pub myeong: Myeong,
    pub kaeyi: Vec<Kaeyi>,
    pub tasks: Vec<Task>,
    pub events: Vec<RuntimeEvent>,
    pub tool_calls: Vec<ToolCall>,
    pub provider_usage: Vec<ProviderUsage>,
}
