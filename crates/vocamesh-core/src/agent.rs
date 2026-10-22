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
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Opencode => "opencode",
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Claude => "Claude Code",
            Self::Codex => "Codex CLI",
            Self::Opencode => "OpenCode",
        }
    }

    pub const fn adapter(self) -> &'static str {
        match self {
            Self::Claude => "claude-code",
            Self::Codex => "codex-cli",
            Self::Opencode => "opencode",
        }
    }

    pub const fn executable(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Opencode => "opencode",
        }
    }
}

impl FromStr for Provider {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "claude" => Ok(Self::Claude),
            "codex" => Ok(Self::Codex),
            "opencode" => Ok(Self::Opencode),
            _ => Err(()),
        }
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalTool {
    pub id: String,
    pub name: String,
    pub adapter: String,
    pub version: String,
    pub path: String,
    pub state: ToolState,
    pub note: String,
    pub auth_state: AuthenticationState,
    pub detected_at: u64,
    pub source: Option<ToolSource>,
}

impl LocalTool {
    pub fn unavailable(provider: Provider, state: ToolState, note: impl Into<String>) -> Self {
        Self {
            id: provider.id().to_owned(),
            name: provider.name().to_owned(),
            adapter: provider.adapter().to_owned(),
            version: "—".to_owned(),
            path: "未检测到".to_owned(),
            state,
            note: note.into(),
            auth_state: AuthenticationState::Unknown,
            detected_at: now_unix_millis(),
            source: None,
        }
    }
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
}
