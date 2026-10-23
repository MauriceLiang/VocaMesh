use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Claude,
    Codex,
    Opencode,
}

impl Provider {
    pub const ALL: [Self; 3] = [Self::Claude, Self::Codex, Self::Opencode];

    pub const fn id(self) -> &'static str {
        provider_descriptor(self).id
    }

    pub const fn name(self) -> &'static str {
        provider_descriptor(self).name
    }

    pub const fn adapter(self) -> &'static str {
        provider_descriptor(self).adapter
    }

    pub const fn executable(self) -> &'static str {
        provider_descriptor(self).executable
    }

    pub fn capabilities(self) -> ProviderCapabilities {
        let provider = provider_descriptor(self);
        ProviderCapabilities {
            provider_id: provider.id.to_owned(),
            display_name: provider.name.to_owned(),
            executable_name: provider.executable.to_owned(),
            adapter: provider.adapter.to_owned(),
            discovery_supported: provider.discovery_supported,
            version_probe_supported: provider.version_probe_supported,
            execution_adapter_available: provider.execution_adapter_available,
            authentication_probe_supported: provider.authentication_probe_supported,
            minimum_version: provider.minimum_version.map(str::to_owned),
            unsupported_reason_code: provider.unsupported_reason_code.map(str::to_owned),
        }
    }
}

struct ProviderDescriptor {
    id: &'static str,
    name: &'static str,
    executable: &'static str,
    adapter: &'static str,
    discovery_supported: bool,
    version_probe_supported: bool,
    execution_adapter_available: bool,
    authentication_probe_supported: bool,
    minimum_version: Option<&'static str>,
    unsupported_reason_code: Option<&'static str>,
}

const PROVIDERS: [ProviderDescriptor; 3] = [
    ProviderDescriptor {
        id: "claude",
        name: "Claude Code",
        executable: "claude",
        adapter: "claude-code",
        discovery_supported: true,
        version_probe_supported: true,
        execution_adapter_available: true,
        authentication_probe_supported: true,
        minimum_version: Some("2.1.259"),
        unsupported_reason_code: None,
    },
    ProviderDescriptor {
        id: "codex",
        name: "Codex CLI",
        executable: "codex",
        adapter: "codex-cli",
        discovery_supported: true,
        version_probe_supported: true,
        execution_adapter_available: false,
        authentication_probe_supported: false,
        minimum_version: None,
        unsupported_reason_code: Some("tool.adapter-unavailable"),
    },
    ProviderDescriptor {
        id: "opencode",
        name: "OpenCode",
        executable: "opencode",
        adapter: "opencode",
        discovery_supported: true,
        version_probe_supported: true,
        execution_adapter_available: false,
        authentication_probe_supported: false,
        minimum_version: None,
        unsupported_reason_code: Some("tool.adapter-unavailable"),
    },
];

const fn provider_descriptor(provider: Provider) -> &'static ProviderDescriptor {
    match provider {
        Provider::Claude => &PROVIDERS[0],
        Provider::Codex => &PROVIDERS[1],
        Provider::Opencode => &PROVIDERS[2],
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCapabilities {
    pub provider_id: String,
    pub display_name: String,
    pub executable_name: String,
    pub adapter: String,
    pub discovery_supported: bool,
    pub version_probe_supported: bool,
    pub execution_adapter_available: bool,
    pub authentication_probe_supported: bool,
    pub minimum_version: Option<String>,
    pub unsupported_reason_code: Option<String>,
}

impl FromStr for Provider {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|provider| provider.id() == value)
            .ok_or(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolState {
    Ready,
    Pending,
    AuthRequired,
    Missing,
    Incompatible,
    Undetectable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthenticationState {
    Authenticated,
    Unauthenticated,
    Unknown,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolSource {
    Path,
    StandardLocation,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolDiscoveryStatus {
    Missing,
    Found,
    PathInvalid,
    ProbeFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolVersionStatus {
    Known,
    Unknown,
    Incompatible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolIntegrationStatus {
    DiscoveryOnly,
    AdapterAvailable,
    UnsupportedPlatform,
    BlockedByPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalTool {
    pub id: String,
    pub name: String,
    pub adapter: String,
    pub version: String,
    pub path: String,
    pub canonical_path: Option<String>,
    pub state: ToolState,
    pub note: String,
    pub auth_state: AuthenticationState,
    pub discovery_status: ToolDiscoveryStatus,
    pub version_status: ToolVersionStatus,
    pub integration_status: ToolIntegrationStatus,
    pub reason_code: Option<String>,
    pub runtime_ready: bool,
    pub runtime_reason_code: Option<String>,
    pub capabilities: ProviderCapabilities,
    pub last_checked_at: u64,
    pub source: Option<ToolSource>,
}

impl LocalTool {
    pub fn unavailable(provider: Provider, state: ToolState, note: impl Into<String>) -> Self {
        let discovery_status = if state == ToolState::Missing {
            ToolDiscoveryStatus::Missing
        } else {
            ToolDiscoveryStatus::ProbeFailed
        };
        let capabilities = provider.capabilities();
        Self {
            id: provider.id().to_owned(),
            name: provider.name().to_owned(),
            adapter: provider.adapter().to_owned(),
            version: "—".to_owned(),
            path: "未检测到".to_owned(),
            canonical_path: None,
            state,
            note: note.into(),
            auth_state: if capabilities.authentication_probe_supported {
                AuthenticationState::Unknown
            } else {
                AuthenticationState::Unsupported
            },
            discovery_status,
            version_status: ToolVersionStatus::Unknown,
            integration_status: if capabilities.execution_adapter_available {
                ToolIntegrationStatus::AdapterAvailable
            } else {
                ToolIntegrationStatus::DiscoveryOnly
            },
            reason_code: Some(
                if discovery_status == ToolDiscoveryStatus::Missing {
                    "tool.not-found"
                } else {
                    "tool.scan-failed"
                }
                .to_owned(),
            ),
            runtime_ready: false,
            runtime_reason_code: None,
            capabilities,
            last_checked_at: now_unix_millis(),
            source: None,
        }
    }
}

/// Derives the legacy display state from independent discovery and execution facts.
pub fn derive_tool_state(tool: &LocalTool) -> ToolState {
    match tool.discovery_status {
        ToolDiscoveryStatus::Missing => return ToolState::Missing,
        ToolDiscoveryStatus::PathInvalid | ToolDiscoveryStatus::ProbeFailed => {
            return ToolState::Undetectable;
        }
        ToolDiscoveryStatus::Found => {}
    }
    if tool.version_status == ToolVersionStatus::Incompatible {
        return ToolState::Incompatible;
    }
    if tool.auth_state == AuthenticationState::Unauthenticated {
        return ToolState::AuthRequired;
    }
    if tool.version_status != ToolVersionStatus::Known
        || !tool.capabilities.execution_adapter_available
        || tool.integration_status != ToolIntegrationStatus::AdapterAvailable
        || !tool.runtime_ready
        || tool.auth_state != AuthenticationState::Authenticated
    {
        return ToolState::Undetectable;
    }
    ToolState::Ready
}

/// Runtime boundary consumed by the desktop command layer; no Tauri types cross it.
pub trait ToolRuntime {
    fn scan_tools(&mut self) -> Result<Vec<LocalTool>, crate::ServiceError>;
    fn verify_tool(&mut self, id: &str) -> Result<LocalTool, crate::ServiceError>;
    fn set_tool_path(&mut self, id: &str, path: &str) -> Result<LocalTool, crate::ServiceError>;
    fn set_default_tool(&mut self, id: &str) -> Result<(), crate::ServiceError>;
    fn default_tool_id(&self) -> Result<Option<String>, crate::ServiceError>;
}

pub fn now_unix_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_only_the_three_supported_provider_ids() {
        assert_eq!("claude".parse(), Ok(Provider::Claude));
        assert_eq!("codex".parse(), Ok(Provider::Codex));
        assert_eq!("opencode".parse(), Ok(Provider::Opencode));
        assert!("omni".parse::<Provider>().is_err());
        assert_eq!(Provider::Codex.id(), "codex");
        assert_eq!(Provider::Codex.adapter(), "codex-cli");
    }

    #[test]
    fn auth_states_are_distinct_from_availability_states() {
        let json = serde_json::to_string(&AuthenticationState::Unsupported).unwrap();
        assert_eq!(json, "\"unsupported\"");
        let tool = LocalTool::unavailable(Provider::Claude, ToolState::Undetectable, "unknown");
        assert_ne!(tool.state, ToolState::Ready);
    }

    #[test]
    fn provider_capability_table_keeps_discovery_and_execution_separate() {
        let claude = Provider::Claude.capabilities();
        assert_eq!(claude.provider_id, "claude");
        assert_eq!(claude.executable_name, "claude");
        assert!(claude.execution_adapter_available);
        assert!(claude.authentication_probe_supported);
        assert_eq!(claude.minimum_version.as_deref(), Some("2.1.259"));

        let codex = Provider::Codex.capabilities();
        assert!(codex.discovery_supported);
        assert!(codex.version_probe_supported);
        assert!(!codex.execution_adapter_available);
        assert!(!codex.authentication_probe_supported);
        assert_eq!(
            codex.unsupported_reason_code.as_deref(),
            Some("tool.adapter-unavailable")
        );
    }
}
