//! Persistent local configuration entities. These records do not grant runtime permissions.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPermissions {
    pub read: bool,
    pub write: bool,
    pub shell: bool,
    pub network: bool,
}

impl Default for AgentPermissions {
    fn default() -> Self {
        Self {
            read: true,
            write: false,
            shell: false,
            network: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRole {
    pub id: String,
    pub name: String,
    pub role: String,
    pub provider_id: String,
    pub description: String,
    pub instructions: String,
    pub permissions: AgentPermissions,
    pub revision: u64,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAgentInput {
    pub name: String,
    pub role: String,
    pub provider_id: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub instructions: String,
    #[serde(default)]
    pub permissions: AgentPermissions,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAgentInput {
    pub name: Option<String>,
    pub role: Option<String>,
    pub provider_id: Option<String>,
    pub description: Option<String>,
    pub instructions: Option<String>,
    pub permissions: Option<AgentPermissions>,
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceScopes {
    pub read: Vec<String>,
    pub write: Vec<String>,
    pub deny: Vec<String>,
}

impl Default for WorkspaceScopes {
    fn default() -> Self {
        Self {
            read: vec![".".to_owned()],
            write: Vec::new(),
            deny: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRecord {
    pub id: String,
    pub name: String,
    pub path: String,
    pub path_available: bool,
    pub task_count: u64,
    pub scopes: WorkspaceScopes,
    pub revision: u64,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceInput {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWorkspaceScopesInput {
    pub scopes: WorkspaceScopes,
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamMemberRecord {
    pub agent_id: String,
    pub name: String,
    pub role: String,
    pub provider_id: String,
    /// `None` is the single supervisor and is represented as `user` in the frontend DTO.
    pub reports_to_agent_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamRecord {
    pub id: String,
    pub name: String,
    pub supervisor_id: String,
    pub workspace_id: String,
    pub workspace: String,
    pub members: Vec<TeamMemberRecord>,
    pub revision: u64,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTeamInput {
    pub name: String,
    pub supervisor_id: String,
    pub workspace_id: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTeamInput {
    pub name: Option<String>,
    pub supervisor_id: Option<String>,
    pub workspace_id: Option<String>,
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddTeamMemberInput {
    pub agent_id: String,
    pub reports_to_agent_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogStatus {
    pub schema_version: u32,
    pub agents: u64,
    pub workspaces: u64,
    pub teams: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceSettings {
    pub theme: String,
    pub density: String,
    pub grayscale_preview: bool,
    pub reduced_motion: bool,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: "system".to_owned(),
            density: "comfortable".to_owned(),
            grayscale_preview: false,
            reduced_motion: false,
        }
    }
}
