use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Environment {
    Sandbox,
    Dev,
    Stg,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Sandbox => "sandbox",
            Environment::Dev => "dev",
            Environment::Stg => "stg",
        }
    }
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Environment::Sandbox => "Sandbox",
            Environment::Dev => "Development",
            Environment::Stg => "Staging",
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum StreamType {
    Stdout,
    Stderr,
    System,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct LogMsg {
    pub timestamp: DateTime<Local>,
    pub stream: StreamType,
    pub content: String,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ScriptStatus {
    Idle,
    Running,
    Succeeded,
    Failed(i32),
    Cancelled,
    AppError(String),
}

#[derive(Clone, PartialEq, Debug)]
pub struct HistoryEntry {
    pub script_name: String,
    pub environment: Environment,
    pub start_time: DateTime<Local>,
    pub end_time: DateTime<Local>,
    pub status: ScriptStatus,
}

#[derive(Clone, PartialEq, Debug)]
pub enum WorkflowStep {
    Neo,
    Authentication,
    Apim,
    Landing,
    EventGrid,
    Raw,
    ServiceBus,
    ContainerAppJobs,
    Processing,
    Curated,
    Verification,
    Quarantine,
    Rejected,
}

impl WorkflowStep {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Neo => "NEO",
            Self::Authentication => "Authentication",
            Self::Apim => "APIM",
            Self::Landing => "Landing",
            Self::EventGrid => "Event Grid",
            Self::Raw => "Raw",
            Self::ServiceBus => "Service Bus",
            Self::ContainerAppJobs => "Container App Jobs",
            Self::Processing => "Processing",
            Self::Curated => "Curated",
            Self::Verification => "Verification",
            Self::Quarantine => "Quarantine",
            Self::Rejected => "Rejected",
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct AppState {
    pub selected_env: Option<Environment>,
    pub selected_script: Option<String>,
    pub script_status: ScriptStatus,
    pub logs: Vec<LogMsg>,
    pub active_start_time: Option<DateTime<Local>>,
    pub active_end_time: Option<DateTime<Local>>,
    pub active_step: Option<WorkflowStep>,
    pub step_history: Vec<WorkflowStep>,
    pub history: Vec<HistoryEntry>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            selected_env: None,
            selected_script: None,
            script_status: ScriptStatus::Idle,
            logs: Vec::new(),
            active_start_time: None,
            active_end_time: None,
            active_step: None,
            step_history: Vec::new(),
            history: Vec::new(),
        }
    }
}
