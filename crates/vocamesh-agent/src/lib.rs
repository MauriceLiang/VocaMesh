//! Safe local CLI discovery. This crate never starts an agent task or reads credentials.

pub mod discovery;
pub mod execution;
pub mod path_resolver;
pub mod process_probe;
pub mod runner;
pub mod settings;

use crate::path_resolver::{resolve, validate_manual_path, Resolution};
use crate::process_probe::{CliProcessProbe, ProbeError, ProcessProbe};
use crate::settings::{RuntimeSettings, SettingsStore};
use vocamesh_core::{
    derive_tool_state, now_unix_millis, AuthenticationState, LocalTool, Provider, ServiceError,
    ToolDiscoveryStatus, ToolIntegrationStatus, ToolRuntime, ToolSource, ToolState,
    ToolVersionStatus,
};

const ERR_NOT_FOUND: ServiceError = ServiceError::new("tool.not-found", "不支持的本机工具");
const ERR_NOT_READY: ServiceError = ServiceError::new("tool.not-ready", "工具尚未通过可用性验证");
const ERR_ADAPTER_UNAVAILABLE: ServiceError = ServiceError::new(
    "tool.adapter-unavailable",
    "该工具当前仅支持发现，尚无执行适配器",
);
const ERR_SETTINGS_READ: ServiceError = ServiceError::new(
    "settings.read-failed",
    "本机工具设置无法读取，请检查设置文件后重试",
);
const ERR_SETTINGS_WRITE: ServiceError =
    ServiceError::new("settings.write-failed", "本机工具设置无法保存");
const ERR_INVALID_PATH: ServiceError = ServiceError::new(
    "tool.invalid-path",
    "路径必须指向存在且可执行的绝对文件路径",
);

pub struct AgentRuntime<P: ProcessProbe = CliProcessProbe> {
    probe: P,
    settings: SettingsStore,
    latest: Vec<LocalTool>,
}

impl AgentRuntime<CliProcessProbe> {
    pub fn new(config_dir: impl Into<std::path::PathBuf>) -> Self {
        Self::with_probe(CliProcessProbe::default(), config_dir)
    }
}

impl<P: ProcessProbe> AgentRuntime<P> {
    pub fn with_probe(probe: P, config_dir: impl Into<std::path::PathBuf>) -> Self {
        Self {
            probe,
            settings: SettingsStore::new(config_dir),
            latest: Vec::new(),
        }
    }

    pub fn scan_tools(&mut self) -> Result<Vec<LocalTool>, ServiceError> {
        let settings = self.settings.load().map_err(|_| ERR_SETTINGS_READ)?;
        let tools = Provider::ALL
            .into_iter()
            .map(|provider| self.inspect(provider, &settings, false))
            .collect::<Vec<_>>();
        self.latest = tools.clone();
        Ok(tools)
    }

    /// Resolve the configured executable using the existing discovery rules.
    pub fn executable_path(&self, provider: Provider) -> Result<std::path::PathBuf, ServiceError> {
        let settings = self.settings.load().map_err(|_| ERR_SETTINGS_READ)?;
        let manual = settings
            .manual_paths
            .get(provider.id())
            .map(std::path::Path::new);
        match resolve(provider, manual).map_err(|_| ERR_INVALID_PATH)? {
            Resolution::Found { canonical_path, .. } => Ok(canonical_path),
            Resolution::Missing | Resolution::InvalidManual { .. } => Err(ERR_NOT_FOUND),
        }
    }

    pub fn verify_tool(&mut self, id: &str) -> Result<LocalTool, ServiceError> {
        let provider = id.parse::<Provider>().map_err(|_| ERR_NOT_FOUND)?;
        let settings = self.settings.load().map_err(|_| ERR_SETTINGS_READ)?;
        let tool = self.inspect(provider, &settings, true);
        self.latest.retain(|existing| existing.id != tool.id);
        self.latest.push(tool.clone());
        Ok(tool)
    }

    pub fn set_tool_path(&mut self, id: &str, path: &str) -> Result<LocalTool, ServiceError> {
        let provider = id.parse::<Provider>().map_err(|_| ERR_NOT_FOUND)?;
        let (display_path, canonical_path) =
            validate_manual_path(std::path::Path::new(path), provider)
                .map_err(|_| ERR_INVALID_PATH)?;
        let mut settings = self.settings.load().map_err(|_| ERR_SETTINGS_READ)?;
        settings.manual_paths.insert(
            provider.id().to_owned(),
            display_path.to_string_lossy().into_owned(),
        );
        self.settings
            .save(&settings)
            .map_err(|_| ERR_SETTINGS_WRITE)?;

        let mut tool = detected_tool(
            provider,
            &display_path,
            Some(&canonical_path),
            ToolSource::Manual,
            "—",
            ToolStatuses {
                discovery: ToolDiscoveryStatus::Found,
                version: ToolVersionStatus::Unknown,
                auth: AuthenticationState::Unknown,
                integration: if provider.capabilities().execution_adapter_available {
                    ToolIntegrationStatus::AdapterAvailable
                } else {
                    ToolIntegrationStatus::DiscoveryOnly
                },
                reason_code: Some("tool.version-unverified"),
            },
            "路径已保存，尚未验证；重新检测只运行固定版本参数",
        );
        tool.state = derive_tool_state(&tool);
        self.latest.retain(|existing| existing.id != tool.id);
        self.latest.push(tool.clone());
        Ok(tool)
    }

    pub fn set_default_tool(&mut self, id: &str) -> Result<(), ServiceError> {
        let provider = id.parse::<Provider>().map_err(|_| ERR_NOT_FOUND)?;
        let capabilities = provider.capabilities();
        if !capabilities.execution_adapter_available {
            return Err(ERR_ADAPTER_UNAVAILABLE);
        }
        if !self.latest.iter().any(|tool| {
            tool.id == provider.id()
                && tool.discovery_status == ToolDiscoveryStatus::Found
                && tool.version_status == ToolVersionStatus::Known
        }) {
            return Err(ERR_NOT_READY);
        }
        let mut settings = self.settings.load().map_err(|_| ERR_SETTINGS_READ)?;
        settings.default_tool_id = Some(provider.id().to_owned());
        self.settings
            .save(&settings)
            .map_err(|_| ERR_SETTINGS_WRITE)
    }

    pub fn default_tool_id(&self) -> Result<Option<String>, ServiceError> {
        let settings = self.settings.load().map_err(|_| ERR_SETTINGS_READ)?;
        Ok(settings.default_tool_id)
    }

    fn inspect(
        &self,
        provider: Provider,
        settings: &RuntimeSettings,
        verify_auth: bool,
    ) -> LocalTool {
        let manual_path = settings
            .manual_paths
            .get(provider.id())
            .map(std::path::Path::new);
        let resolution = match resolve(provider, manual_path) {
            Ok(resolution) => resolution,
            Err(_) => {
                let mut tool = LocalTool::unavailable(
                    provider,
                    ToolState::Undetectable,
                    "保存的路径当前无法访问，请重新指定并验证",
                );
                tool.discovery_status = ToolDiscoveryStatus::PathInvalid;
                tool.reason_code = Some("tool.path-unavailable".to_owned());
                return tool;
            }
        };
        let (path, canonical_path, source) = match resolution {
            Resolution::Missing => {
                return LocalTool::unavailable(
                    provider,
                    ToolState::Missing,
                    "PATH 与支持的标准安装目录中未找到该 CLI",
                )
            }
            Resolution::InvalidManual { path } => {
                return detected_tool(
                    provider,
                    &path,
                    None,
                    ToolSource::Manual,
                    "未知版本",
                    ToolStatuses {
                        discovery: ToolDiscoveryStatus::PathInvalid,
                        version: ToolVersionStatus::Unknown,
                        auth: AuthenticationState::Unknown,
                        integration: integration_for(provider),
                        reason_code: Some("tool.path-unavailable"),
                    },
                    "保存的手动路径当前不是可访问的可执行文件，请重新指定路径",
                )
            }
            Resolution::Found {
                path,
                canonical_path,
                source,
            } => (path, Some(canonical_path), source),
        };

        if path_resolver::is_windows_script(&path) {
            return detected_tool(
                provider,
                &path,
                canonical_path.as_deref(),
                source,
                "未知版本",
                ToolStatuses {
                    discovery: ToolDiscoveryStatus::Found,
                    version: ToolVersionStatus::Unknown,
                    auth: AuthenticationState::Unsupported,
                    integration: ToolIntegrationStatus::UnsupportedPlatform,
                    reason_code: Some("tool.unsupported-launcher"),
                },
                "检测到 Windows 脚本 shim；为避免经过 cmd.exe 启动可执行任意脚本，无法安全探测",
            );
        }

        let executable = canonical_path.as_deref().unwrap_or(&path);
        match self.probe.version(executable) {
            Ok(output) => match discovery::parse_version(&output.stdout) {
                Some(version) => {
                    let version_status = if is_compatible(provider, &version) {
                        ToolVersionStatus::Known
                    } else {
                        ToolVersionStatus::Incompatible
                    };
                    let auth_probe = if provider == Provider::Claude
                        && verify_auth
                        && version_status == ToolVersionStatus::Known
                    {
                        self.probe
                            .authentication(provider, canonical_path.as_deref().unwrap_or(&path))
                    } else if provider == Provider::Claude {
                        crate::process_probe::AuthenticationProbeResult {
                            state: AuthenticationState::Unknown,
                            reason_code: Some("execution.auth-not-verified"),
                        }
                    } else {
                        crate::process_probe::AuthenticationProbeResult {
                            state: AuthenticationState::Unsupported,
                            reason_code: None,
                        }
                    };
                    let auth_state = auth_probe.state;
                    let reason_code = match version_status {
                        ToolVersionStatus::Incompatible => Some("tool.version-incompatible"),
                        ToolVersionStatus::Known
                            if !provider.capabilities().execution_adapter_available =>
                        {
                            Some("tool.adapter-unavailable")
                        }
                        ToolVersionStatus::Known if auth_probe.reason_code.is_some() => {
                            auth_probe.reason_code
                        }
                        _ => None,
                    };
                    let note = match reason_code {
                        Some("tool.adapter-unavailable") => {
                            "CLI 已找到并读取版本；当前仅支持本地发现，尚无执行适配器"
                        }
                        Some("tool.version-incompatible") => "CLI 版本低于当前受支持的最低版本",
                        Some("execution.auth-required") => "Claude Code CLI 尚未登录",
                        Some("execution.auth-unknown") => {
                            "认证探测返回有效信息，但无法确认 Claude Code 登录状态"
                        }
                        Some("execution.auth-not-verified") => {
                            "CLI 版本已读取；尚未执行 Claude Code 认证探测"
                        }
                        Some("execution.auth-output-unrecognized") => {
                            "Claude Code 认证输出无法识别，登录状态保持未知"
                        }
                        Some("execution.auth-probe-timeout") => {
                            "Claude Code 认证探测超时，登录状态保持未知"
                        }
                        Some("execution.auth-probe-failed") => {
                            "Claude Code 认证探测失败，登录状态保持未知"
                        }
                        Some("execution.auth-cli-incompatible") => {
                            "当前 Claude Code 版本不支持受控认证探测"
                        }
                        _ => "CLI 版本已读取，认证与 Runtime 就绪状态已分别记录",
                    };
                    detected_tool(
                        provider,
                        &path,
                        canonical_path.as_deref(),
                        source,
                        &version,
                        ToolStatuses {
                            discovery: ToolDiscoveryStatus::Found,
                            version: version_status,
                            auth: auth_state,
                            integration: integration_for(provider),
                            reason_code,
                        },
                        note,
                    )
                }
                None => detected_tool(
                    provider,
                    &path,
                    canonical_path.as_deref(),
                    source,
                    "未知版本",
                    ToolStatuses {
                        discovery: ToolDiscoveryStatus::Found,
                        version: ToolVersionStatus::Unknown,
                        auth: if provider == Provider::Claude {
                            AuthenticationState::Unknown
                        } else {
                            AuthenticationState::Unsupported
                        },
                        integration: integration_for(provider),
                        reason_code: Some("tool.version-unavailable"),
                    },
                    "CLI 已找到，但版本输出为空或格式无法识别；认证状态未知",
                ),
            },
            Err(error) => detected_tool(
                provider,
                &path,
                canonical_path.as_deref(),
                source,
                "未知版本",
                ToolStatuses {
                    discovery: ToolDiscoveryStatus::ProbeFailed,
                    version: ToolVersionStatus::Unknown,
                    auth: if provider == Provider::Claude {
                        AuthenticationState::Unknown
                    } else {
                        AuthenticationState::Unsupported
                    },
                    integration: integration_for(provider),
                    reason_code: Some(error.code()),
                },
                error.user_message(),
            ),
        }
    }
}

impl<P: ProcessProbe> ToolRuntime for AgentRuntime<P> {
    fn scan_tools(&mut self) -> Result<Vec<LocalTool>, ServiceError> {
        AgentRuntime::scan_tools(self)
    }

    fn verify_tool(&mut self, id: &str) -> Result<LocalTool, ServiceError> {
        AgentRuntime::verify_tool(self, id)
    }

    fn set_tool_path(&mut self, id: &str, path: &str) -> Result<LocalTool, ServiceError> {
        AgentRuntime::set_tool_path(self, id, path)
    }

    fn set_default_tool(&mut self, id: &str) -> Result<(), ServiceError> {
        AgentRuntime::set_default_tool(self, id)
    }

    fn default_tool_id(&self) -> Result<Option<String>, ServiceError> {
        AgentRuntime::default_tool_id(self)
    }
}

struct ToolStatuses {
    discovery: ToolDiscoveryStatus,
    version: ToolVersionStatus,
    auth: AuthenticationState,
    integration: ToolIntegrationStatus,
    reason_code: Option<&'static str>,
}

fn detected_tool(
    provider: Provider,
    path: &std::path::Path,
    canonical_path: Option<&std::path::Path>,
    source: ToolSource,
    version: &str,
    statuses: ToolStatuses,
    note: &str,
) -> LocalTool {
    let mut tool = LocalTool {
        id: provider.id().to_owned(),
        name: provider.name().to_owned(),
        adapter: provider.adapter().to_owned(),
        version: version.to_owned(),
        path: path.to_string_lossy().into_owned(),
        canonical_path: canonical_path.map(|path| path.to_string_lossy().into_owned()),
        state: ToolState::Pending,
        note: note.to_owned(),
        auth_state: statuses.auth,
        discovery_status: statuses.discovery,
        version_status: statuses.version,
        integration_status: statuses.integration,
        reason_code: statuses.reason_code.map(str::to_owned),
        runtime_ready: false,
        runtime_reason_code: None,
        capabilities: provider.capabilities(),
        last_checked_at: now_unix_millis(),
        source: Some(source),
    };
    tool.state = derive_tool_state(&tool);
    tool
}

fn integration_for(provider: Provider) -> ToolIntegrationStatus {
    if provider.capabilities().execution_adapter_available {
        ToolIntegrationStatus::AdapterAvailable
    } else {
        ToolIntegrationStatus::DiscoveryOnly
    }
}

fn is_compatible(provider: Provider, version: &str) -> bool {
    let Some(minimum) = provider.capabilities().minimum_version else {
        return true;
    };
    let parse = |value: &str| {
        let stable = value.split(['+', '-']).next()?;
        let mut parts = stable.split('.').map(str::parse::<u64>);
        Some((
            parts.next()?.ok()?,
            parts.next()?.ok()?,
            parts.next()?.ok()?,
        ))
    };
    match (parse(version), parse(&minimum)) {
        (Some(actual), Some(minimum)) if actual == minimum => !version.contains('-'),
        (Some(actual), Some(minimum)) => actual > minimum,
        _ => false,
    }
}

impl From<ProbeError> for ServiceError {
    fn from(value: ProbeError) -> Self {
        ServiceError::new(value.code(), value.user_message())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process_probe::{ProbeError, ProcessOutput};
    use std::{fs, path::Path};

    #[derive(Clone)]
    struct FakeProbe(Result<ProcessOutput, ProbeError>);

    impl ProcessProbe for FakeProbe {
        fn version(&self, _: &Path) -> Result<ProcessOutput, ProbeError> {
            self.0.clone()
        }

        fn authentication(
            &self,
            provider: Provider,
            _: &Path,
        ) -> crate::process_probe::AuthenticationProbeResult {
            if provider == Provider::Claude {
                crate::process_probe::AuthenticationProbeResult {
                    state: AuthenticationState::Authenticated,
                    reason_code: None,
                }
            } else {
                crate::process_probe::AuthenticationProbeResult {
                    state: AuthenticationState::Unsupported,
                    reason_code: None,
                }
            }
        }
    }

    fn setup(probe: FakeProbe) -> (tempfile::TempDir, AgentRuntime<FakeProbe>) {
        setup_for(Provider::Codex, probe)
    }

    fn setup_for(
        provider: Provider,
        probe: FakeProbe,
    ) -> (tempfile::TempDir, AgentRuntime<FakeProbe>) {
        let dir = tempfile::tempdir().unwrap();
        #[cfg(windows)]
        let binary = dir.path().join(format!("{}.exe", provider.executable()));
        #[cfg(not(windows))]
        let binary = dir.path().join(provider.executable());
        fs::write(&binary, b"test executable").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let settings_dir = dir.path().join("settings");
        let mut settings = RuntimeSettings::default();
        settings.manual_paths.insert(
            provider.id().to_owned(),
            binary.to_string_lossy().into_owned(),
        );
        SettingsStore::new(&settings_dir).save(&settings).unwrap();
        (dir, AgentRuntime::with_probe(probe, settings_dir))
    }

    fn version(stdout: &str) -> Result<ProcessOutput, ProbeError> {
        Ok(ProcessOutput {
            stdout: stdout.to_owned(),
        })
    }

    #[test]
    fn version_success_does_not_claim_authentication_or_readiness() {
        let (_dir, mut runtime) = setup(FakeProbe(version("codex 1.2.3\n")));
        let tools = runtime.scan_tools().unwrap();
        let codex = tools.iter().find(|tool| tool.id == "codex").unwrap();
        assert_eq!(codex.version, "1.2.3");
        assert_eq!(codex.auth_state, AuthenticationState::Unsupported);
        assert_eq!(codex.state, ToolState::Undetectable);
        assert_eq!(codex.discovery_status, ToolDiscoveryStatus::Found);
        assert_eq!(codex.version_status, ToolVersionStatus::Known);
        assert_eq!(
            codex.integration_status,
            ToolIntegrationStatus::AdapterAvailable
        );
        assert_eq!(codex.reason_code, None);
        assert!(!tools.iter().any(|tool| tool.state == ToolState::Ready));
    }

    #[test]
    fn claude_version_threshold_and_explicit_auth_verification_are_separate() {
        let (_dir, mut scan_runtime) =
            setup_for(Provider::Claude, FakeProbe(version("Claude Code 2.1.259")));
        let scanned = scan_runtime
            .scan_tools()
            .unwrap()
            .into_iter()
            .find(|tool| tool.id == "claude")
            .unwrap();
        assert_eq!(scanned.version_status, ToolVersionStatus::Known);
        assert_eq!(scanned.auth_state, AuthenticationState::Unknown);
        assert_eq!(
            scanned.reason_code.as_deref(),
            Some("execution.auth-not-verified")
        );
        assert_eq!(
            scanned.integration_status,
            ToolIntegrationStatus::AdapterAvailable
        );

        let verified = scan_runtime.verify_tool("claude").unwrap();
        assert_eq!(verified.auth_state, AuthenticationState::Authenticated);
        assert!(!verified.runtime_ready);
        assert_eq!(verified.state, ToolState::Undetectable);

        let (_dir, mut old_runtime) =
            setup_for(Provider::Claude, FakeProbe(version("Claude Code 2.1.258")));
        let old = old_runtime.scan_tools().unwrap().remove(0);
        assert_eq!(old.version_status, ToolVersionStatus::Incompatible);
        assert_eq!(
            old.reason_code.as_deref(),
            Some("tool.version-incompatible")
        );
    }

    #[test]
    fn timeout_nonzero_empty_and_unparseable_outputs_never_become_ready() {
        for output in [
            Err(ProbeError::Timeout),
            Err(ProbeError::NonZeroExit),
            version(""),
            version("version unknown"),
        ] {
            let (_dir, mut runtime) = setup(FakeProbe(output));
            let codex = runtime
                .verify_tool("codex")
                .expect("probe outcome is represented as a tool state");
            assert_eq!(codex.state, ToolState::Undetectable);
            assert_ne!(codex.auth_state, AuthenticationState::Authenticated);
        }
    }

    #[test]
    fn codex_adapter_can_be_preferred_without_becoming_runtime_ready() {
        let (_dir, mut runtime) = setup(FakeProbe(version("codex 1.2.3")));
        runtime.scan_tools().unwrap();
        runtime.set_default_tool("codex").unwrap();
        assert_eq!(runtime.default_tool_id().unwrap().as_deref(), Some("codex"));
        let codex = runtime
            .latest
            .iter()
            .find(|tool| tool.id == "codex")
            .unwrap();
        assert!(!codex.runtime_ready);
    }

    #[test]
    fn discovery_does_not_clear_a_saved_default_preference() {
        let (_dir, mut runtime) = setup(FakeProbe(version("codex 1.2.3")));
        let mut settings = runtime.settings.load().unwrap();
        settings.default_tool_id = Some("codex".to_owned());
        runtime.settings.save(&settings).unwrap();

        runtime.scan_tools().unwrap();

        assert_eq!(
            runtime.settings.load().unwrap().default_tool_id.as_deref(),
            Some("codex")
        );
        assert_eq!(runtime.default_tool_id().unwrap().as_deref(), Some("codex"));
    }

    #[test]
    fn adapter_and_valid_version_allow_a_default_preference_while_runtime_is_blocked() {
        let (_dir, mut runtime) =
            setup_for(Provider::Claude, FakeProbe(version("Claude Code 2.1.296")));
        runtime.scan_tools().unwrap();

        runtime.set_default_tool("claude").unwrap();

        assert_eq!(
            runtime.default_tool_id().unwrap().as_deref(),
            Some("claude")
        );
    }

    #[test]
    fn manual_path_must_be_absolute_executable_file_and_is_pending_verification() {
        let (_dir, mut runtime) = setup(FakeProbe(version("codex 1.2.3")));
        assert_eq!(
            runtime
                .set_tool_path("codex", "relative/path")
                .unwrap_err()
                .code,
            "tool.invalid-path"
        );
        let dir_path = tempfile::tempdir().unwrap();
        assert_eq!(
            runtime
                .set_tool_path("codex", dir_path.path().to_str().unwrap())
                .unwrap_err()
                .code,
            "tool.invalid-path"
        );
        let saved_path = runtime
            .settings
            .load()
            .unwrap()
            .manual_paths
            .remove("codex")
            .unwrap();
        let tool = runtime.set_tool_path("codex", &saved_path).unwrap();
        assert_eq!(tool.state, ToolState::Undetectable);
        assert_eq!(tool.auth_state, AuthenticationState::Unknown);
        assert_eq!(tool.source, Some(ToolSource::Manual));
        assert!(tool.note.contains("尚未验证"));
    }

    #[test]
    fn manual_path_must_match_the_selected_provider_executable_name() {
        let (_dir, mut runtime) = setup(FakeProbe(version("codex 1.2.3")));
        let dir = tempfile::tempdir().unwrap();
        #[cfg(windows)]
        let binary = dir.path().join("claude.exe");
        #[cfg(not(windows))]
        let binary = dir.path().join("claude");
        fs::write(&binary, b"executable").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).unwrap();
        }

        assert_eq!(
            runtime
                .set_tool_path("codex", binary.to_str().unwrap())
                .unwrap_err()
                .code,
            "tool.invalid-path"
        );
    }

    #[test]
    fn removed_manual_path_is_reported_as_unavailable_not_missing_cli() {
        let (_dir, mut runtime) = setup(FakeProbe(version("codex 1.2.3")));
        let path = runtime
            .settings
            .load()
            .unwrap()
            .manual_paths
            .get("codex")
            .unwrap()
            .clone();
        fs::remove_file(path).unwrap();

        let codex = runtime
            .scan_tools()
            .unwrap()
            .into_iter()
            .find(|tool| tool.id == "codex")
            .unwrap();

        assert_eq!(codex.discovery_status, ToolDiscoveryStatus::PathInvalid);
        assert_eq!(codex.reason_code.as_deref(), Some("tool.path-unavailable"));
        assert_eq!(codex.state, ToolState::Undetectable);
    }

    #[test]
    fn changing_the_default_tool_path_preserves_the_old_default_preference() {
        let (_dir, mut runtime) =
            setup_for(Provider::Claude, FakeProbe(version("Claude Code 2.1.296")));
        let saved_path = runtime
            .settings
            .load()
            .unwrap()
            .manual_paths
            .remove("claude")
            .unwrap();
        let mut settings = runtime.settings.load().unwrap();
        settings.default_tool_id = Some("claude".to_owned());
        runtime.settings.save(&settings).unwrap();
        runtime.latest = vec![detected_tool(
            Provider::Claude,
            Path::new(&saved_path),
            Some(Path::new(&saved_path)),
            ToolSource::Manual,
            "2.1.296",
            ToolStatuses {
                discovery: ToolDiscoveryStatus::Found,
                version: ToolVersionStatus::Known,
                auth: AuthenticationState::Unknown,
                integration: ToolIntegrationStatus::AdapterAvailable,
                reason_code: None,
            },
            "test fixture",
        )];
        runtime.set_default_tool("claude").unwrap();

        runtime.set_tool_path("claude", &saved_path).unwrap();
        assert_eq!(
            runtime.default_tool_id().unwrap().as_deref(),
            Some("claude")
        );
    }

    #[test]
    fn malformed_settings_return_a_recoverable_error_without_echoing_contents() {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::new(dir.path());
        fs::create_dir_all(dir.path()).unwrap();
        fs::write(store.config_path(), "secret-looking malformed input").unwrap();
        assert!(store.load().is_err());
        let mut runtime = AgentRuntime::with_probe(FakeProbe(version("codex 1.2.3")), dir.path());
        let error = runtime.scan_tools().unwrap_err();
        assert_eq!(error.code, "settings.read-failed");
        assert!(!error.message.contains("secret-looking"));
    }

    #[test]
    fn missing_tools_are_reported_without_mock_seed_data() {
        let dir = tempfile::tempdir().unwrap();
        let mut runtime = AgentRuntime::with_probe(FakeProbe(version("codex 1.2.3")), dir.path());
        let tools = runtime.scan_tools().unwrap();
        assert_eq!(tools.len(), 3);
        assert!(tools.iter().all(|tool| tool.id != "omni"));
    }
}
