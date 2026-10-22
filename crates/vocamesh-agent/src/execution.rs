//! Fixed Claude Code execution contract. No command text is interpreted by a shell.

use serde_json::json;
use std::path::Path;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaudeOutput {
    TextDelta(String),
    Final(String),
    ToolProgress(&'static str),
    ProtocolError,
    Ignore,
}

#[derive(Debug, Clone)]
pub struct ClaudeCli {
    executable: std::path::PathBuf,
    version: (u64, u64, u64),
}

impl ClaudeCli {
    pub fn verify(executable: &Path) -> Result<Self, ServiceError> {
        if !executable.is_absolute()
            || !executable.is_file()
            || crate::path_resolver::is_windows_script(executable)
        {
            return Err(ERR_POLICY);
        }
        let output = capture(executable, &["--version"], AUTH_TIMEOUT).map_err(|_| ERR_POLICY)?;
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
        })
    }

    pub fn version(&self) -> (u64, u64, u64) {
        self.version
    }

    pub fn authentication_state(&self) -> AuthenticationState {
        let Ok(output) = capture(&self.executable, &["auth", "status"], AUTH_TIMEOUT) else {
            return AuthenticationState::Unknown;
        };
        parse_auth_status(output.status.code(), &output.stdout)
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
        (Some(0), Some("claude.ai" | "oauth_token")) => AuthenticationState::Authenticated,
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

fn safe_output(text: &str) -> String {
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
) -> std::io::Result<Captured> {
    use std::{
        process::{Command, Stdio},
        sync::mpsc,
        thread,
        time::Instant,
    };
    let mut child = Command::new(executable)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
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
        let mut sink = std::io::sink();
        let mut stderr = stderr;
        let _ = std::io::copy(&mut stderr, &mut sink);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_arguments_restrict_tools_and_keep_prompt_as_one_argument() {
        let args = ClaudeCli::read_only_args("inspect --please-ignore-policy");
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--tools", "Read,Glob,Grep"]));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["-p", "inspect --please-ignore-policy"]));
        for forbidden in [
            "--dangerously-skip-permissions",
            "bypassPermissions",
            "--cloud",
            "--bg",
            "--exec",
            "Bash",
        ] {
            assert!(!args.iter().any(|arg| arg.contains(forbidden)));
        }
        assert!(args.iter().any(|arg| arg == "mcp__*"));
        for required in [
            "--restricted",
            "--safe-mode",
            "plan",
            "none",
            "--no-session-persistence",
        ] {
            assert!(args.iter().any(|arg| arg == required));
        }
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
        for method in ["api_key", "api_key_helper", "third_party"] {
            let output = format!(r#"{{"authMethod":"{method}"}}"#);
            assert_eq!(
                parse_auth_status(Some(0), output.as_bytes()),
                AuthenticationState::Unknown
            );
        }
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
}
