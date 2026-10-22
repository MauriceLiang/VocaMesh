use crate::AuthenticationState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionReadiness {
    pub ready: bool,
    pub version: Option<String>,
    pub auth_state: AuthenticationState,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunState {
    Pending,
    Starting,
    Running,
    StopRequested,
    Finished,
    Crashed,
    Unknown,
}

impl RunState {
    pub const fn is_active(self) -> bool {
        matches!(
            self,
            Self::Pending | Self::Starting | Self::Running | Self::StopRequested
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionRequest {
    pub request_id: String,
    pub title: String,
    pub prompt: String,
    pub provider_id: String,
    pub workspace_path: String,
    pub consent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRecord {
    pub id: String,
    pub title: String,
    pub prompt: String,
    pub provider_id: String,
    pub workspace_path: String,
    pub state: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub current_run_id: String,
    pub authorization_scope: String,
    pub consent_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRecord {
    pub id: String,
    pub request_id: String,
    pub task_id: String,
    pub provider_id: String,
    pub state: RunState,
    pub outcome: Option<String>,
    pub started_at: Option<u64>,
    pub finished_at: Option<u64>,
    pub exit_code: Option<i32>,
    pub summary: Option<String>,
    pub last_sequence: u64,
    pub output_truncated: bool,
    pub process_id: Option<u32>,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunEvent {
    pub run_id: String,
    pub sequence: u64,
    pub timestamp: u64,
    pub kind: String,
    pub text: String,
}
