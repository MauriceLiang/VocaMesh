//! UI-independent domain contracts shared by the runtime and Tauri shell.

pub mod agent;
pub mod catalog;
pub mod error;
pub mod events;
pub mod execution;

pub use agent::{
    now_unix_millis, AuthenticationState, LocalTool, Provider, ToolRuntime, ToolSource, ToolState,
};
pub use catalog::{
    AddTeamMemberInput, AgentPermissions, AgentRole, AppearanceSettings, CatalogStatus,
    CreateAgentInput, CreateTeamInput, CreateWorkspaceInput, TeamMemberRecord, TeamRecord,
    UpdateAgentInput, UpdateTeamInput, UpdateWorkspaceScopesInput, WorkspaceRecord,
    WorkspaceScopes,
};
pub use error::ServiceError;
pub use events::{ScanPhase, ToolScanEvent};
pub use execution::{
    ExecutionReadiness, ExecutionRequest, ProcessGroupStatus, RecoveryInspection,
    RecoveryResolution, RecoveryStatus, RunEvent, RunRecord, RunState, SnapshotStatus, TaskRecord,
    UnknownRunSummary,
};
