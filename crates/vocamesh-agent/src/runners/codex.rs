use crate::{
    execution::{classify_cli_stderr, read_bounded_stderr, safe_output, ExecutionEnvironment},
    path_resolver::is_windows_script,
    runner::{ProviderOutput, ProviderRunner},
};
use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use vocamesh_core::{AuthenticationState, Provider, ServiceError};

const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
const PROBE_OUTPUT_LIMIT: usize = 64 * 1024;
const REQUIRED_EXEC_FLAGS: &[&str] = &[
    "--json",
    "--sandbox",
    "--ignore-user-config",
    "--ignore-rules",
    "--ephemeral",
    "--cd",
];
const ERR_INCOMPATIBLE: ServiceError = ServiceError::new(
    "execution.codex-version-incompatible",
    "Codex CLI 缺少本阶段要求的非交互只读参数",
);

pub struct CodexCliRunner {
    executable: PathBuf,
    environment: ExecutionEnvironment,
    version: String,
}

impl CodexCliRunner {
    pub fn new(
        executable: PathBuf,
        environment: ExecutionEnvironment,
    ) -> Result<Self, ServiceError> {
        if !executable.is_absolute() || !executable.is_file() || is_windows_script(&executable) {
            return Err(ERR_INCOMPATIBLE);
        }
        let version_output = capture(&executable, &["--version"], &environment)?;
        if !version_output.status.success() {
            return Err(ERR_INCOMPATIBLE);
        }
        let version = parse_version(&version_output.stdout).ok_or(ERR_INCOMPATIBLE)?;
        let help = capture(&executable, &["exec", "--help"], &environment)?;
        if !help.status.success() || !supports_required_flags(&help.stdout) {
            return Err(ERR_INCOMPATIBLE);
        }
        Ok(Self {
            executable,
            environment,
            version,
        })
    }

    fn verify_compatible(&self) -> Result<(), ServiceError> {
        let version = capture(&self.executable, &["--version"], &self.environment)?;
        let help = capture(&self.executable, &["exec", "--help"], &self.environment)?;
        if version.status.success()
            && parse_version(&version.stdout).as_deref() == Some(self.version.as_str())
            && help.status.success()
            && supports_required_flags(&help.stdout)
        {
            Ok(())
        } else {
            Err(ERR_INCOMPATIBLE)
        }
    }
}

impl ProviderRunner for CodexCliRunner {
    fn provider(&self) -> Provider {
        Provider::Codex
    }

    fn executable(&self) -> &Path {
        &self.executable
    }

    fn version(&self) -> Option<String> {
        Some(self.version.clone())
    }

    fn probe(&self) -> Result<AuthenticationState, ServiceError> {
        self.verify_compatible()?;
        Ok(AuthenticationState::Unsupported)
    }

    fn launch_command(&self, _prompt: &str, working_directory: &Path) -> Command {
        let mut command = Command::new(&self.executable);
        command
            .args([
                "exec",
                "--json",
                "--sandbox",
                "read-only",
                "--ignore-user-config",
                "--ignore-rules",
                "--ephemeral",
                "--cd",
            ])
            .arg(working_directory)
            .arg("-")
            .current_dir(working_directory)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        self.environment.apply(&mut command);
        command
    }

    fn prompt_on_stdin(&self) -> bool {
        true
    }

    fn classify_failure(&self, stderr: &[u8]) -> &'static str {
        classify_cli_stderr(stderr).error_code()
    }
}

pub fn parse_jsonl_event(line: &[u8]) -> ProviderOutput {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(line) else {
        return ProviderOutput::ProtocolError;
    };
    match value.get("type").and_then(serde_json::Value::as_str) {
        Some("thread.started") => ProviderOutput::Ignore,
        Some("turn.started") => ProviderOutput::ToolProgress("Codex 已开始处理只读任务"),
        Some("item.started" | "item.updated") => {
            ProviderOutput::ToolProgress("Codex 正在处理只读任务")
        }
        Some("item.completed") => {
            let item = &value["item"];
            if item.get("type").and_then(serde_json::Value::as_str) == Some("agent_message") {
                item.get("text")
                    .and_then(serde_json::Value::as_str)
                    .map(safe_output)
                    .map(ProviderOutput::Final)
                    .unwrap_or(ProviderOutput::Ignore)
            } else {
                ProviderOutput::Ignore
            }
        }
        Some("turn.completed") => ProviderOutput::ProtocolComplete,
        Some("turn.failed" | "error") => ProviderOutput::ProtocolFailed,
        _ => ProviderOutput::ProtocolError,
    }
}

fn supports_required_flags(help: &[u8]) -> bool {
    let text = String::from_utf8_lossy(help);
    REQUIRED_EXEC_FLAGS.iter().all(|flag| text.contains(flag)) && text.contains("read-only")
}

fn parse_version(output: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(output).ok()?;
    let start = text.find(|ch: char| ch.is_ascii_digit())?;
    let version = text[start..]
        .split_whitespace()
        .next()?
        .trim_end_matches(|ch: char| !ch.is_ascii_digit() && ch != '.');
    let mut parts = version.split('.');
    let (major, minor, patch) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some()
        || [major, minor, patch]
            .iter()
            .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return None;
    }
    Some(format!("{major}.{minor}.{patch}"))
}

struct ProbeOutput {
    status: std::process::ExitStatus,
    stdout: Vec<u8>,
}

fn capture(
    executable: &Path,
    args: &[&str],
    environment: &ExecutionEnvironment,
) -> Result<ProbeOutput, ServiceError> {
    let mut command = Command::new(executable);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    environment.apply(&mut command);
    let mut child = command
        .spawn()
        .map_err(|_| ServiceError::new("execution.codex-probe-failed", "Codex CLI 探测失败"))?;
    let stdout = child.stdout.take().ok_or(ERR_INCOMPATIBLE)?;
    let stderr = child.stderr.take().ok_or(ERR_INCOMPATIBLE)?;
    let (tx, rx) = mpsc::channel();
    let stdout_tx = tx.clone();
    thread::spawn(move || {
        let _ = stdout_tx.send(read_capped(stdout, PROBE_OUTPUT_LIMIT));
    });
    thread::spawn(move || {
        let _ = read_bounded_stderr(stderr);
    });
    let deadline = Instant::now() + PROBE_TIMEOUT;
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|_| ServiceError::new("execution.codex-probe-failed", "Codex CLI 探测失败"))?
        {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ServiceError::new(
                "execution.codex-probe-timeout",
                "Codex CLI 探测超时",
            ));
        }
        thread::sleep(Duration::from_millis(10));
    };
    let stdout = rx
        .recv_timeout(PROBE_TIMEOUT)
        .map_err(|_| ServiceError::new("execution.codex-probe-timeout", "Codex CLI 探测超时"))?;
    if stdout.len() > PROBE_OUTPUT_LIMIT {
        return Err(ERR_INCOMPATIBLE);
    }
    Ok(ProbeOutput { status, stdout })
}

fn read_capped<R: Read>(mut reader: R, limit: usize) -> Vec<u8> {
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
    fn codex_jsonl_extracts_allowlisted_text_and_requires_explicit_turn_completion() {
        assert_eq!(
            parse_jsonl_event(
                br#"{"type":"item.completed","item":{"type":"agent_message","text":"VOCAMESH marker"}}"#
            ),
            ProviderOutput::Final("VOCAMESH marker".into())
        );
        assert_eq!(
            parse_jsonl_event(br#"{"type":"turn.completed","usage":{"input_tokens":1}}"#),
            ProviderOutput::ProtocolComplete
        );
        assert_eq!(
            parse_jsonl_event(br#"{"type":"turn.failed","error":{"message":"do not retain"}}"#),
            ProviderOutput::ProtocolFailed
        );
    }

    #[test]
    fn codex_jsonl_rejects_unknown_and_malformed_events_without_persisting_payload() {
        assert_eq!(
            parse_jsonl_event(br#"{"type":"unknown","secret":"sk-ant-test"}"#),
            ProviderOutput::ProtocolError
        );
        assert_eq!(
            parse_jsonl_event(b"not-json"),
            ProviderOutput::ProtocolError
        );
        assert_eq!(parse_jsonl_event(b"\xff"), ProviderOutput::ProtocolError);
        assert_eq!(
            parse_jsonl_event(
                br#"{"type":"item.completed","item":{"type":"agent_message","text":"token=sk-ant-test-secret"}}"#
            ),
            ProviderOutput::Final("[内容已隐藏：检测到疑似凭据]".into())
        );
    }

    #[test]
    fn codex_version_and_required_flags_are_checked_without_a_model_request() {
        assert_eq!(
            parse_version(b"codex-cli 0.162.1\n"),
            Some("0.162.1".into())
        );
        assert_eq!(parse_version(b"codex-cli unknown\n"), None);
        let help = REQUIRED_EXEC_FLAGS
            .iter()
            .copied()
            .chain(["read-only"])
            .collect::<Vec<_>>()
            .join(" ");
        assert!(supports_required_flags(help.as_bytes()));
        assert!(!supports_required_flags(b"codex exec --json"));
    }
}
