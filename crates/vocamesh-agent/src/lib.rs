//! Safe local CLI discovery. This crate never starts an agent task or reads credentials.

pub mod discovery;
pub mod path_resolver;
pub mod process_probe;
pub mod settings;

use crate::path_resolver::{resolve, validate_manual_path, Resolution};
use crate::process_probe::{CliProcessProbe, ProbeError, ProcessProbe};
use crate::settings::{RuntimeSettings, SettingsStore};
use vocamesh_core::{
    now_unix_millis, AuthenticationState, LocalTool, Provider, ServiceError, ToolRuntime,
    ToolSource, ToolState,
};

const ERR_NOT_FOUND: ServiceError = ServiceError::new("tool.not-found", "不支持的本机工具");
const ERR_NOT_READY: ServiceError = ServiceError::new("tool.not-ready", "工具尚未通过可用性验证");
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
        Self::with_probe(CliProcessProbe, config_dir)
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
            .map(|provider| self.inspect(provider, &settings))
            .collect::<Vec<_>>();
        self.latest = tools.clone();

        let mut settings = settings;
        if settings.default_tool_id.as_deref().is_some_and(|id| {
            tools
                .iter()
                .find(|tool| tool.id == id)
                .is_none_or(|tool| tool.state != ToolState::Ready)
        }) {
            settings.default_tool_id = None;
            self.settings
                .save(&settings)
                .map_err(|_| ERR_SETTINGS_WRITE)?;
        }
        Ok(tools)
    }

    pub fn verify_tool(&mut self, id: &str) -> Result<LocalTool, ServiceError> {
        let provider = id.parse::<Provider>().map_err(|_| ERR_NOT_FOUND)?;
        let settings = self.settings.load().map_err(|_| ERR_SETTINGS_READ)?;
        let tool = self.inspect(provider, &settings);
        self.latest.retain(|existing| existing.id != tool.id);
        self.latest.push(tool.clone());
        if tool.state != ToolState::Ready {
            let mut settings = settings;
            if settings.default_tool_id.as_deref() == Some(id) {
                settings.default_tool_id = None;
                self.settings
                    .save(&settings)
                    .map_err(|_| ERR_SETTINGS_WRITE)?;
            }
        }
        Ok(tool)
    }

    pub fn set_tool_path(&mut self, id: &str, path: &str) -> Result<LocalTool, ServiceError> {
        let provider = id.parse::<Provider>().map_err(|_| ERR_NOT_FOUND)?;
        let path =
            validate_manual_path(std::path::Path::new(path)).map_err(|_| ERR_INVALID_PATH)?;
        let mut settings = self.settings.load().map_err(|_| ERR_SETTINGS_READ)?;
        settings.manual_paths.insert(
            provider.id().to_owned(),
            path.to_string_lossy().into_owned(),
        );
        if settings.default_tool_id.as_deref() == Some(provider.id()) {
            settings.default_tool_id = None;
        }
        self.settings
            .save(&settings)
            .map_err(|_| ERR_SETTINGS_WRITE)?;

        let tool = LocalTool {
            id: provider.id().to_owned(),
            name: provider.name().to_owned(),
            adapter: provider.adapter().to_owned(),
            version: "—".to_owned(),
            path: path.to_string_lossy().into_owned(),
            state: ToolState::Undetectable,
            note: "路径已保存，尚未验证；点击重新检测后只运行固定版本参数".to_owned(),
            auth_state: AuthenticationState::Unknown,
            detected_at: now_unix_millis(),
            source: Some(ToolSource::Manual),
        };
        self.latest.retain(|existing| existing.id != tool.id);
        self.latest.push(tool.clone());
        Ok(tool)
    }

    pub fn set_default_tool(&mut self, id: &str) -> Result<(), ServiceError> {
        let provider = id.parse::<Provider>().map_err(|_| ERR_NOT_FOUND)?;
        if !self
            .latest
            .iter()
            .any(|tool| tool.id == provider.id() && tool.state == ToolState::Ready)
        {
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
        Ok(settings.default_tool_id.filter(|id| {
            self.latest
                .iter()
                .any(|tool| tool.id == *id && tool.state == ToolState::Ready)
        }))
    }

    fn inspect(&self, provider: Provider, settings: &RuntimeSettings) -> LocalTool {
        let manual_path = settings
            .manual_paths
            .get(provider.id())
            .map(std::path::Path::new);
        let resolution = match resolve(provider, manual_path) {
            Ok(resolution) => resolution,
            Err(_) => {
                return LocalTool::unavailable(
                    provider,
                    ToolState::Undetectable,
                    "保存的路径当前无法访问，请重新指定并验证",
                )
            }
        };
        let (path, source) = match resolution {
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
                    ToolSource::Manual,
                    "未知版本",
                    ToolState::Undetectable,
                    AuthenticationState::Unknown,
                    "保存的手动路径当前不是可访问的可执行文件，请重新指定路径",
                )
            }
            Resolution::Found { path, source } => (path, source),
        };

        if path_resolver::is_windows_script(&path) {
            return detected_tool(
                provider,
                &path,
                source,
                "未知版本",
                ToolState::Undetectable,
                AuthenticationState::Unsupported,
                "检测到 Windows 脚本 shim；为避免经过 cmd.exe 启动可执行任意脚本，无法安全探测",
            );
        }

        match self.probe.version(&path) {
            Ok(output) => match discovery::parse_version(&output.stdout) {
                Some(version) => detected_tool(
                    provider,
                    &path,
                    source,
                    &version,
                    ToolState::Undetectable,
                    AuthenticationState::Unsupported,
                    "版本已读取；官方资料未确认安全的只读登录状态命令，因此认证状态未验证",
                ),
                None => detected_tool(
                    provider,
                    &path,
                    source,
                    "未知版本",
                    ToolState::Undetectable,
                    AuthenticationState::Unknown,
                    "CLI 已找到，但版本输出为空或格式无法识别；认证状态未知",
                ),
            },
            Err(error) => detected_tool(
                provider,
                &path,
                source,
                "未知版本",
                ToolState::Undetectable,
                AuthenticationState::Unknown,
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

fn detected_tool(
    provider: Provider,
    path: &std::path::Path,
    source: ToolSource,
    version: &str,
    state: ToolState,
    auth_state: AuthenticationState,
    note: &str,
) -> LocalTool {
    LocalTool {
        id: provider.id().to_owned(),
        name: provider.name().to_owned(),
        adapter: provider.adapter().to_owned(),
        version: version.to_owned(),
        path: path.to_string_lossy().into_owned(),
        state,
        note: note.to_owned(),
        auth_state,
        detected_at: now_unix_millis(),
        source: Some(source),
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
    }

    fn setup(probe: FakeProbe) -> (tempfile::TempDir, AgentRuntime<FakeProbe>) {
        let dir = tempfile::tempdir().unwrap();
        let binary = dir.path().join("safe-test-cli");
        fs::write(&binary, b"test executable").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let settings_dir = dir.path().join("settings");
        let mut settings = RuntimeSettings::default();
        settings
            .manual_paths
            .insert("codex".to_owned(), binary.to_string_lossy().into_owned());
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
        assert!(!tools.iter().any(|tool| tool.state == ToolState::Ready));
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
    fn unknown_or_unsupported_authentication_cannot_become_ready_or_default() {
        let (_dir, mut runtime) = setup(FakeProbe(version("codex 1.2.3")));
        runtime.scan_tools().unwrap();
        let error = runtime.set_default_tool("codex").unwrap_err();
        assert_eq!(error.code, "tool.not-ready");
        assert_eq!(runtime.default_tool_id().unwrap(), None);
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
    fn changing_the_default_tool_path_clears_the_old_default() {
        let (_dir, mut runtime) = setup(FakeProbe(version("codex 1.2.3")));
        let saved_path = runtime
            .settings
            .load()
            .unwrap()
            .manual_paths
            .remove("codex")
            .unwrap();
        let mut settings = runtime.settings.load().unwrap();
        settings.default_tool_id = Some("codex".to_owned());
        runtime.settings.save(&settings).unwrap();
        runtime.latest = vec![detected_tool(
            Provider::Codex,
            Path::new(&saved_path),
            ToolSource::Manual,
            "1.2.3",
            ToolState::Ready,
            AuthenticationState::Authenticated,
            "test fixture",
        )];
        runtime.set_default_tool("codex").unwrap();

        runtime.set_tool_path("codex", &saved_path).unwrap();
        assert_eq!(runtime.default_tool_id().unwrap(), None);
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
