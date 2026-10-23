//! Fixed Claude Code execution contract. No command text is interpreted by a shell.

use serde_json::json;
use std::{ffi::OsString, path::Path, process::Command};
use vocamesh_core::{AuthenticationState, Provider, ServiceError};

pub const MIN_SAFE_CLAUDE_VERSION: (u64, u64, u64) = (2, 1, 259);
const ERR_PROVIDER: ServiceError = ServiceError::new(
    "execution.provider-not-supported",
    "本阶段只支持 Claude Code 只读执行",
);
const ERR_POLICY: ServiceError = ServiceError::new(
    "execution.read-only-policy-unavailable",
    "当前 Claude Code 版本不支持所需的只读安全参数",
);
const AUTH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
const AUTH_OUTPUT_LIMIT: usize = 16 * 1024;
const STDERR_LIMIT: usize = 16 * 1024;

/// The environment shared by CLI preflight and the eventual task process.
/// Values are deliberately private so they cannot be logged accidentally.
#[derive(Clone, Default)]
pub struct ExecutionEnvironment(Vec<(OsString, OsString)>);

impl ExecutionEnvironment {
    pub fn current() -> Self {
        Self::current_for(Provider::Claude)
    }

    pub fn current_for(provider: Provider) -> Self {
        const CLAUDE_ALLOWED: &[&str] = &[
            "PATH",
            "HOME",
            "USER",
            "USERPROFILE",
            "APPDATA",
            "LOCALAPPDATA",
            "SYSTEMROOT",
            "TEMP",
            "TMP",
            "TMPDIR",
            "LANG",
            "LC_ALL",
            "TERM",
            "NO_COLOR",
            "CLAUDE_CONFIG_DIR",
        ];
        const CODEX_ALLOWED: &[&str] = &[
            "PATH",
            "SYSTEMROOT",
            "TEMP",
            "TMP",
            "TMPDIR",
            "LANG",
            "LC_ALL",
            "TERM",
            "NO_COLOR",
        ];
        let allowed = match provider {
            Provider::Claude => CLAUDE_ALLOWED,
            Provider::Codex => CODEX_ALLOWED,
            Provider::Opencode => &[],
        };
        Self(
            allowed
                .iter()
                .filter_map(|key| std::env::var_os(key).map(|value| (OsString::from(key), value)))
                .collect(),
        )
    }

    pub fn apply(&self, command: &mut Command) {
        command.env_clear().envs(self.0.iter().cloned());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliFailureKind {
    InvalidArguments,
    Authentication,
    Configuration,
    Other,
}

impl CliFailureKind {
    pub const fn error_code(self) -> &'static str {
        match self {
            Self::InvalidArguments => "execution.cli-invalid-arguments",
            Self::Authentication => "execution.auth-required",
            Self::Configuration => "execution.cli-configuration-error",
            Self::Other => "execution.cli-failed",
        }
    }
}

/// Classify bounded stderr without returning or persisting any source text.
pub fn classify_cli_stderr(stderr: &[u8]) -> CliFailureKind {
    let text = String::from_utf8_lossy(stderr).to_ascii_lowercase();
    if [
        "unknown option",
        "unrecognized option",
        "unknown argument",
        "unexpected argument",
        "invalid argument",
        "unknown command",
    ]
    .iter()
    .any(|marker| text.contains(marker))
    {
        CliFailureKind::InvalidArguments
    } else if [
        "not logged in",
        "authentication required",
        "unauthorized",
        "invalid api key",
        "authentication_error",
    ]
    .iter()
    .any(|marker| text.contains(marker))
    {
        CliFailureKind::Authentication
    } else if [
        "failed to load settings",
        "invalid settings",
        "configuration error",
        "config directory",
        "settings.json",
    ]
    .iter()
    .any(|marker| text.contains(marker))
    {
        CliFailureKind::Configuration
    } else {
        CliFailureKind::Other
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaudeOutput {
    TextDelta(String),
    Final(String),
    ToolProgress(&'static str),
    ProtocolError,
    Ignore,
}

#[derive(Clone)]
pub struct ClaudeCli {
    executable: std::path::PathBuf,
    version: (u64, u64, u64),
    environment: ExecutionEnvironment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthenticationProbeError {
    Timeout,
    Failed,
}

impl ClaudeCli {
    pub fn verify(executable: &Path) -> Result<Self, ServiceError> {
        Self::verify_with_environment(executable, ExecutionEnvironment::current())
    }

    pub fn verify_with_environment(
        executable: &Path,
        environment: ExecutionEnvironment,
    ) -> Result<Self, ServiceError> {
        if !executable.is_absolute()
            || !executable.is_file()
            || crate::path_resolver::is_windows_script(executable)
        {
            return Err(ERR_POLICY);
        }
        let output = capture(executable, &["--version"], AUTH_TIMEOUT, &environment)
            .map_err(|_| ERR_POLICY)?;
        if !output.status.success() {
            return Err(ERR_POLICY);
        }
        let version_text = std::str::from_utf8(&output.stdout).map_err(|_| ERR_POLICY)?;
        let version = parse_version(version_text).ok_or(ERR_POLICY)?;
        if version < MIN_SAFE_CLAUDE_VERSION {
            return Err(ERR_POLICY);
        }
        Ok(Self {
            executable: executable.to_path_buf(),
            version,
            environment,
        })
    }

    pub fn version(&self) -> (u64, u64, u64) {
        self.version
    }

    pub fn authentication_state(&self) -> AuthenticationState {
        self.authentication_probe_state()
            .unwrap_or(AuthenticationState::Unknown)
    }

    pub fn authentication_probe_state(
        &self,
    ) -> Result<AuthenticationState, AuthenticationProbeError> {
        let output = capture(
            &self.executable,
            &["auth", "status"],
            AUTH_TIMEOUT,
            &self.environment,
        )
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::TimedOut {
                AuthenticationProbeError::Timeout
            } else {
                AuthenticationProbeError::Failed
            }
        })?;
        Ok(parse_auth_status(output.status.code(), &output.stdout))
    }

    pub fn configure_command(&self, command: &mut Command) {
        self.environment.apply(command);
    }

    pub fn read_only_args(prompt: &str) -> Vec<String> {
        let settings = json!({
            "permissions": {
                "deny": [
                    "Read(**/.env)", "Read(**/.env.*)", "Read(**/.ssh/**)",
                    "Read(**/.aws/**)", "Read(**/.config/**)", "Read(**/.claude/**)",
                    "Read(**/.codex/**)", "Read(**/.opencode/**)", "Read(**/.git/**)",
                    "Read(**/*id_rsa*)", "Read(**/*id_ed25519*)", "Read(**/*.pem)",
                    "Read(**/*.key)", "Read(**/*.p12)", "Read(**/*.pfx)", "Agent"
                ]
            }
        })
        .to_string();
        vec![
            "--restricted".into(),
            "--safe-mode".into(),
            "--permission-mode".into(),
            "plan".into(),
            "--permission-prompts".into(),
            "none".into(),
            "--tools".into(),
            "Read,Glob,Grep".into(),
            "--disallowedTools".into(),
            "mcp__*".into(),
            "--settings".into(),
            settings,
            "--no-session-persistence".into(),
            "--output-format".into(),
            "stream-json".into(),
            "--include-partial-messages".into(),
            "-p".into(),
            prompt.into(),
        ]
    }
}

pub fn parse_auth_status(exit_code: Option<i32>, output: &[u8]) -> AuthenticationState {
    let Ok(json) = serde_json::from_slice::<serde_json::Value>(output) else {
        return AuthenticationState::Unknown;
    };
    let method = json.get("authMethod").and_then(serde_json::Value::as_str);
    match (exit_code, method) {
        (Some(1), Some("none")) => AuthenticationState::Unauthenticated,
        (
            Some(0),
            Some("claude.ai" | "oauth_token" | "api_key" | "api_key_helper" | "third_party"),
        ) => AuthenticationState::Authenticated,
        _ => AuthenticationState::Unknown,
    }
}

pub fn validate_execution_provider(provider: Provider) -> Result<(), ServiceError> {
    if provider == Provider::Claude {
        Ok(())
    } else {
        Err(ERR_PROVIDER)
    }
}

pub fn parse_stream_json(line: &[u8]) -> ClaudeOutput {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(line) else {
        return ClaudeOutput::ProtocolError;
    };
    match value.get("type").and_then(serde_json::Value::as_str) {
        Some("stream_event") => {
            let event = &value["event"];
            match event.get("type").and_then(serde_json::Value::as_str) {
                Some("content_block_delta") if event["delta"]["type"] == "text_delta" => event
                    ["delta"]["text"]
                    .as_str()
                    .map(safe_output)
                    .map(ClaudeOutput::TextDelta)
                    .unwrap_or(ClaudeOutput::Ignore),
                Some("content_block_start") if event["content_block"]["type"] == "tool_use" => {
                    ClaudeOutput::ToolProgress("正在读取工作区文件")
                }
                _ => ClaudeOutput::Ignore,
            }
        }
        Some("result") => value["result"]
            .as_str()
            .map(safe_output)
            .map(ClaudeOutput::Final)
            .unwrap_or(ClaudeOutput::Ignore),
        Some("assistant" | "system" | "user") => ClaudeOutput::Ignore,
        _ => ClaudeOutput::ProtocolError,
    }
}

pub(crate) fn safe_output(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let suspicious = [
        "sk-ant-",
        "sk-live-",
        "sk-proj-",
        "sk-test-",
        "ghp_",
        "gho_",
        "github_pat_",
        "akia",
        "-----begin ",
        "api_key=",
        "api_key\":",
        "client_secret",
        "private_key",
        "access_token",
        "refresh_token",
        "password=",
        "authorization:",
        "token=",
        "bearer ",
    ]
    .iter()
    .any(|marker| lower.contains(marker));
    if suspicious {
        return "[内容已隐藏：检测到疑似凭据]".into();
    }
    let mut end = text.len().min(64 * 1024);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_owned()
}

fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    let start = text.find(|ch: char| ch.is_ascii_digit())?;
    let version = text[start..].split_whitespace().next()?;
    let mut pieces = version.split('.').map(str::parse::<u64>);
    Some((
        pieces.next()?.ok()?,
        pieces.next()?.ok()?,
        pieces.next()?.ok()?,
    ))
}

struct Captured {
    status: std::process::ExitStatus,
    stdout: Vec<u8>,
}

fn capture(
    executable: &Path,
    args: &[&str],
    timeout: std::time::Duration,
    environment: &ExecutionEnvironment,
) -> std::io::Result<Captured> {
    use std::{
        process::{Command, Stdio},
        sync::mpsc,
        thread,
        time::Instant,
    };
    let mut command = Command::new(executable);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    environment.apply(&mut command);
    let mut child = command.spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| std::io::Error::other("stdout unavailable"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| std::io::Error::other("stderr unavailable"))?;
    let (tx, rx) = mpsc::channel();
    let stdout_tx = tx.clone();
    thread::spawn(move || {
        let _ = stdout_tx.send(read_capped(stdout, AUTH_OUTPUT_LIMIT));
    });
    thread::spawn(move || {
        let _ = read_bounded(stderr, STDERR_LIMIT);
    });
    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "CLI probe timed out",
            ));
        }
        thread::sleep(std::time::Duration::from_millis(10));
    };
    let stdout = rx
        .recv_timeout(timeout)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "CLI output timed out"))?;
    if stdout.len() > AUTH_OUTPUT_LIMIT {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "CLI output too large",
        ));
    }
    Ok(Captured { status, stdout })
}

fn read_capped<R: std::io::Read>(mut reader: R, limit: usize) -> Vec<u8> {
    let mut output = Vec::with_capacity(limit.min(1024));
    let mut buffer = [0u8; 4096];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(count) => {
                if output.len() <= limit {
                    let keep = count.min(limit + 1 - output.len());
                    output.extend_from_slice(&buffer[..keep]);
                }
            }
        }
    }
    output
}

fn read_bounded<R: std::io::Read>(mut reader: R, limit: usize) -> Vec<u8> {
    let mut output = Vec::with_capacity(limit.min(1024));
    let mut buffer = [0u8; 4096];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(count) => {
                let keep = count.min(limit.saturating_sub(output.len()));
                output.extend_from_slice(&buffer[..keep]);
            }
        }
    }
    output
}

pub fn read_bounded_stderr<R: std::io::Read>(reader: R) -> Vec<u8> {
    read_bounded(reader, STDERR_LIMIT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_arguments_restrict_tools_and_keep_prompt_as_one_argument() {
        let args = ClaudeCli::read_only_args("inspect --please-ignore-policy");
        let settings = json!({
            "permissions": {
                "deny": [
                    "Read(**/.env)", "Read(**/.env.*)", "Read(**/.ssh/**)",
                    "Read(**/.aws/**)", "Read(**/.config/**)", "Read(**/.claude/**)",
                    "Read(**/.codex/**)", "Read(**/.opencode/**)", "Read(**/.git/**)",
                    "Read(**/*id_rsa*)", "Read(**/*id_ed25519*)", "Read(**/*.pem)",
                    "Read(**/*.key)", "Read(**/*.p12)", "Read(**/*.pfx)", "Agent"
                ]
            }
        })
        .to_string();
        assert_eq!(
            args,
            vec![
                "--restricted",
                "--safe-mode",
                "--permission-mode",
                "plan",
                "--permission-prompts",
                "none",
                "--tools",
                "Read,Glob,Grep",
                "--disallowedTools",
                "mcp__*",
                "--settings",
                &settings,
                "--no-session-persistence",
                "--output-format",
                "stream-json",
                "--include-partial-messages",
                "-p",
                "inspect --please-ignore-policy",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>()
        );
    }

    #[test]
    fn parser_only_exposes_allowlisted_text_and_redacts_suspected_secrets() {
        let text = br#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"Read README"}}}"#;
        assert_eq!(
            parse_stream_json(text),
            ClaudeOutput::TextDelta("Read README".into())
        );
        let secret = br#"{"type":"result","result":"key sk-ant-abc123"}"#;
        assert_eq!(
            parse_stream_json(secret),
            ClaudeOutput::Final("[内容已隐藏：检测到疑似凭据]".into())
        );
        let unknown = br#"{"type":"user","message":"secret payload"}"#;
        assert_eq!(parse_stream_json(unknown), ClaudeOutput::Ignore);
        assert_eq!(parse_stream_json(b"not-json"), ClaudeOutput::ProtocolError);
    }

    #[test]
    fn rejects_old_cli_versions_for_required_flags() {
        assert!(parse_version("2.1.259 (Claude Code)").unwrap() >= MIN_SAFE_CLAUDE_VERSION);
        assert!(parse_version("2.1.258 (Claude Code)").unwrap() < MIN_SAFE_CLAUDE_VERSION);
    }

    #[test]
    fn auth_status_uses_only_exit_code_and_allowlisted_auth_method() {
        assert_eq!(
            parse_auth_status(
                Some(0),
                br#"{"authMethod":"claude.ai","email":"private@example.test"}"#
            ),
            AuthenticationState::Authenticated
        );
        assert_eq!(
            parse_auth_status(
                Some(1),
                br#"{"authMethod":"none","configDirectory":"/private/path"}"#
            ),
            AuthenticationState::Unauthenticated
        );
        assert_eq!(
            parse_auth_status(Some(1), br#"{"authMethod":"claude.ai"}"#),
            AuthenticationState::Unknown
        );
        assert_eq!(
            parse_auth_status(Some(0), b"invalid"),
            AuthenticationState::Unknown
        );
        assert_eq!(
            parse_auth_status(Some(0), br#"{"authMethod":"none"}"#),
            AuthenticationState::Unknown
        );
        for method in [
            "claude.ai",
            "oauth_token",
            "api_key",
            "api_key_helper",
            "third_party",
        ] {
            let output = format!(r#"{{"authMethod":"{method}"}}"#);
            assert_eq!(
                parse_auth_status(Some(0), output.as_bytes()),
                AuthenticationState::Authenticated
            );
        }
        assert_eq!(
            parse_auth_status(Some(0), br#"{"authMethod":"future_method"}"#),
            AuthenticationState::Unknown
        );
        assert_eq!(
            parse_auth_status(None, br#"{"authMethod":"claude.ai"}"#),
            AuthenticationState::Unknown
        );
    }

    #[test]
    fn safe_output_limit_is_measured_in_bytes_without_splitting_utf8() {
        let output = safe_output(&"界".repeat(30_000));
        assert!(output.len() <= 64 * 1024);
        assert!(std::str::from_utf8(output.as_bytes()).is_ok());
    }

    #[test]
    fn stderr_classifier_returns_only_fixed_categories_for_invalid_and_sensitive_text() {
        assert_eq!(
            classify_cli_stderr(b"error: unknown option --unsafe"),
            CliFailureKind::InvalidArguments
        );
        assert_eq!(
            classify_cli_stderr(b"Authentication required: token=sk-ant-secret /Users/alice"),
            CliFailureKind::Authentication
        );
        assert_eq!(
            classify_cli_stderr(
                b"Failed to load settings from /private/path/.claude/settings.json"
            ),
            CliFailureKind::Configuration
        );
        assert_eq!(classify_cli_stderr(&[0xff, 0xfe]), CliFailureKind::Other);
        assert_eq!(
            CliFailureKind::Authentication.error_code(),
            "execution.auth-required"
        );
    }

    #[test]
    fn stderr_reader_caps_retained_bytes_but_drains_the_pipe() {
        let payload = vec![b'x'; STDERR_LIMIT * 4];
        let mut reader = std::io::Cursor::new(payload);
        assert_eq!(read_bounded(&mut reader, STDERR_LIMIT).len(), STDERR_LIMIT);
        assert_eq!(reader.position(), (STDERR_LIMIT * 4) as u64);
    }

    #[test]
    fn execution_environment_drops_unapproved_variables() {
        let environment = ExecutionEnvironment::current();
        let mut command = std::process::Command::new("/usr/bin/env");
        command.env("VOCAMESH_TEST_SECRET", "must-not-be-inherited");
        environment.apply(&mut command);
        assert!(!command
            .get_envs()
            .any(|(key, _)| key == std::ffi::OsStr::new("VOCAMESH_TEST_SECRET")));
    }

    #[cfg(unix)]
    #[test]
    fn version_auth_and_task_use_the_same_restricted_environment() {
        use std::{
            os::unix::fs::PermissionsExt,
            process::{Command, Stdio},
        };

        let directory = tempfile::tempdir().unwrap();
        let executable = directory.path().join("fake-claude");
        let config_dir = directory.path().join("config");
        std::fs::create_dir(&config_dir).unwrap();
        let config_value = config_dir.to_string_lossy();
        std::fs::write(
            &executable,
            format!(
                r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  [ "$CLAUDE_CONFIG_DIR" = "{config_value}" ] || exit 20
  printf '2.1.296 (Claude Code)\n'
  exit 0
fi
if [ "$1" = "auth" ]; then
  [ "$CLAUDE_CONFIG_DIR" = "{config_value}" ] || exit 21
  printf '{{"authMethod":"oauth_token"}}\n'
  exit 0
fi
[ "$CLAUDE_CONFIG_DIR" = "{config_value}" ] || exit 22
printf '{{"type":"result","result":"env-ok"}}\n'
"#
            ),
        )
        .unwrap();
        let mut permissions = std::fs::metadata(&executable).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&executable, permissions).unwrap();
        let environment = ExecutionEnvironment(vec![
            (
                OsString::from("PATH"),
                std::env::var_os("PATH").unwrap_or_default(),
            ),
            (
                OsString::from("CLAUDE_CONFIG_DIR"),
                config_dir.into_os_string(),
            ),
        ]);

        let cli = ClaudeCli::verify_with_environment(&executable, environment).unwrap();
        assert_eq!(
            cli.authentication_state(),
            AuthenticationState::Authenticated
        );
        let mut command = Command::new(&executable);
        command
            .args(ClaudeCli::read_only_args("fixture"))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        cli.configure_command(&mut command);
        let output = command.output().unwrap();
        assert!(output.status.success());
        assert!(String::from_utf8(output.stdout).unwrap().contains("env-ok"));
    }
}
