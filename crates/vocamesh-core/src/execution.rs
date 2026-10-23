use crate::AuthenticationState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionReadiness {
    pub ready: bool,
    pub version: Option<String>,
    pub auth_state: AuthenticationState,
    pub message: String,
    pub reason_code: Option<String>,
    pub policy_status: ExecutionPolicyStatus,
    pub checks: Vec<ExecutionCheck>,
    pub blockers: Vec<ExecutionBlocker>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExecutionPolicyStatus {
    Verified,
    Unverified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExecutionCheckId {
    LocalStorage,
    ActiveRun,
    ProcessState,
    Platform,
    CliDiscovery,
    CliVersion,
    Authentication,
    ManagedPolicy,
    TaskStart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionCheckStatus {
    Passed,
    Failed,
    Blocked,
    Unknown,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionCheck {
    pub id: ExecutionCheckId,
    pub status: ExecutionCheckStatus,
    pub reason_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionBlocker {
    pub check: ExecutionCheckId,
    pub reason_code: String,
}
impl ExecutionReadiness {
    pub fn storage_unavailable(message: &str) -> Self {
        let storage_error = "storage.read-failed".to_owned();
        let checks = vec![
            ExecutionCheck {
                id: ExecutionCheckId::LocalStorage,
                status: ExecutionCheckStatus::Failed,
                reason_code: Some(storage_error.clone()),
            },
            ExecutionCheck {
                id: ExecutionCheckId::ActiveRun,
                status: ExecutionCheckStatus::Unknown,
                reason_code: Some(storage_error.clone()),
            },
            ExecutionCheck {
                id: ExecutionCheckId::ProcessState,
                status: ExecutionCheckStatus::Unknown,
                reason_code: Some(storage_error.clone()),
            },
            ExecutionCheck {
                id: ExecutionCheckId::Platform,
                status: if cfg!(windows) {
                    ExecutionCheckStatus::Unsupported
                } else {
                    ExecutionCheckStatus::Passed
                },
                reason_code: cfg!(windows).then(|| "execution.platform-unsupported".into()),
            },
            ExecutionCheck {
                id: ExecutionCheckId::CliDiscovery,
                status: ExecutionCheckStatus::Unknown,
                reason_code: Some("execution.cli-unknown".into()),
            },
            ExecutionCheck {
                id: ExecutionCheckId::CliVersion,
                status: ExecutionCheckStatus::Unknown,
                reason_code: Some("execution.cli-unknown".into()),
            },
            ExecutionCheck {
                id: ExecutionCheckId::Authentication,
                status: ExecutionCheckStatus::Unknown,
                reason_code: Some("execution.auth-unknown".into()),
            },
            ExecutionCheck {
                id: ExecutionCheckId::ManagedPolicy,
                status: ExecutionCheckStatus::Blocked,
                reason_code: Some("execution.managed-policy-unverified".into()),
            },
            ExecutionCheck {
                id: ExecutionCheckId::TaskStart,
                status: ExecutionCheckStatus::Blocked,
                reason_code: Some(storage_error.clone()),
            },
        ];
        let blockers = checks
            .iter()
            .filter_map(|check| {
                let reason_code = check.reason_code.as_ref()?;
                (check.status != ExecutionCheckStatus::Passed).then(|| ExecutionBlocker {
                    check: check.id,
                    reason_code: reason_code.clone(),
                })
            })
            .collect();
        Self {
            ready: false,
            version: None,
            auth_state: AuthenticationState::Unknown,
            message: message.to_owned(),
            reason_code: Some(storage_error),
            policy_status: ExecutionPolicyStatus::Unverified,
            checks,
            blockers,
        }
    }
}

#[cfg(test)]
mod readiness_tests {
    use super::*;

    #[test]
    fn readiness_diagnostics_serialize_with_stable_ipc_names() {
        let readiness = ExecutionReadiness::storage_unavailable("storage unavailable");
        let value = serde_json::to_value(&readiness).unwrap();

        assert_eq!(value["policyStatus"], "unverified");
        assert_eq!(value["authState"], "unknown");
        assert_eq!(value["checks"][0]["id"], "localStorage");
        assert_eq!(value["checks"][0]["status"], "failed");
        assert_eq!(value["blockers"][0]["check"], "localStorage");
        assert_eq!(value["blockers"][0]["reasonCode"], "storage.read-failed");
        assert_eq!(
            serde_json::from_value::<ExecutionReadiness>(value).unwrap(),
            readiness
        );
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecoveryStatus {
    None,
    Required,
    Resolved,
    ManualRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProcessGroupStatus {
    ExitedCandidate,
    StillRunning,
    CannotVerify,
    ManualRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SnapshotStatus {
    Exists,
    Missing,
    CannotVerify,
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
    pub recovery_status: RecoveryStatus,
    pub recovery_checked_at: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnknownRunSummary {
    pub run: RunRecord,
    pub task_title: String,
    pub snapshot_status: SnapshotStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryInspection {
    pub run: RunRecord,
    pub task_title: String,
    pub process_group_status: ProcessGroupStatus,
    pub snapshot_status: SnapshotStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryResolution {
    pub run: RunRecord,
    pub snapshot_status: SnapshotStatus,
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
