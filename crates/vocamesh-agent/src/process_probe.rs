use crate::execution::{AuthenticationProbeError, ClaudeCli, ExecutionEnvironment};
use std::{
    io::Read,
    path::Path,
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};
use vocamesh_core::{AuthenticationState, Provider};

const PROBE_TIMEOUT: Duration = Duration::from_secs(3);
const OUTPUT_LIMIT: usize = 8 * 1024;
const POLL_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessOutput {
    pub stdout: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeError {
    Timeout,
    NonZeroExit,
    OutputTooLarge,
    UnsupportedLauncher,
    SpawnFailure,
    InvalidOutput,
}

impl ProbeError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Timeout => "tool.probe-timeout",
            Self::NonZeroExit => "tool.version-unavailable",
            Self::OutputTooLarge | Self::InvalidOutput => "tool.version-unavailable",
            Self::UnsupportedLauncher => "tool.unsupported-launcher",
            Self::SpawnFailure => "tool.scan-failed",
        }
    }

    pub const fn user_message(self) -> &'static str {
        match self {
            Self::Timeout => "固定版本探测命令超时",
            Self::NonZeroExit => "固定版本探测命令返回失败状态",
            Self::OutputTooLarge => "版本输出超过安全上限，已丢弃",
            Self::UnsupportedLauncher => "检测到启动器，但当前无法安全探测",
            Self::SpawnFailure => "无法启动固定版本探测命令",
            Self::InvalidOutput => "版本输出无法安全解码",
        }
    }
}

pub trait ProcessProbe: Send + Sync {
    fn version(&self, executable: &Path) -> Result<ProcessOutput, ProbeError>;

    fn authentication(&self, provider: Provider, executable: &Path) -> AuthenticationProbeResult {
        let _ = executable;
        if provider == Provider::Claude {
            AuthenticationProbeResult {
                state: AuthenticationState::Unknown,
                reason_code: Some("execution.auth-unknown"),
            }
        } else {
            AuthenticationProbeResult {
                state: AuthenticationState::Unsupported,
                reason_code: None,
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthenticationProbeResult {
    pub state: AuthenticationState,
    pub reason_code: Option<&'static str>,
}

#[derive(Clone)]
pub struct CliProcessProbe {
    environment: ExecutionEnvironment,
}

impl Default for CliProcessProbe {
    fn default() -> Self {
        Self {
            environment: ExecutionEnvironment::current(),
        }
    }
}

impl ProcessProbe for CliProcessProbe {
    fn version(&self, executable: &Path) -> Result<ProcessOutput, ProbeError> {
        if crate::path_resolver::is_windows_script(executable) {
            return Err(ProbeError::UnsupportedLauncher);
        }
        let mut command = Command::new(executable);
        self.environment.apply(&mut command);
        let mut child = command
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| ProbeError::SpawnFailure)?;

        let (Some(stdout), Some(stderr)) = (child.stdout.take(), child.stderr.take()) else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ProbeError::SpawnFailure);
        };
        let (sender, receiver) = mpsc::channel();
        spawn_reader(stdout, Pipe::Stdout, sender.clone());
        spawn_reader(stderr, Pipe::Stderr, sender);

        let started = Instant::now();
        let deadline = started + PROBE_TIMEOUT;
        let status = loop {
            match child.try_wait() {
                Err(_) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(ProbeError::SpawnFailure);
                }
                Ok(Some(status)) => break status,
                Ok(None) if Instant::now() >= deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(ProbeError::Timeout);
                }
                Ok(None) => thread::sleep(POLL_INTERVAL),
            }
        };

        let (stdout, _stderr) = receive_pipes(receiver, deadline)?;
        if !status.success() {
            return Err(ProbeError::NonZeroExit);
        }
        Ok(ProcessOutput {
            stdout: String::from_utf8(stdout).map_err(|_| ProbeError::InvalidOutput)?,
        })
    }

    fn authentication(&self, provider: Provider, executable: &Path) -> AuthenticationProbeResult {
        if provider != Provider::Claude || crate::path_resolver::is_windows_script(executable) {
            return AuthenticationProbeResult {
                state: AuthenticationState::Unsupported,
                reason_code: None,
            };
        }
        let probe = ClaudeCli::verify_with_environment(executable, self.environment.clone())
            .map_err(|_| AuthenticationProbeError::Failed)
            .and_then(|cli| cli.authentication_probe_state());
        classify_authentication_probe(probe)
    }
}

fn classify_authentication_probe(
    probe: Result<AuthenticationState, AuthenticationProbeError>,
) -> AuthenticationProbeResult {
    let (state, reason_code) = match probe {
        Ok(AuthenticationState::Authenticated) => (AuthenticationState::Authenticated, None),
        Ok(AuthenticationState::Unauthenticated) => (
            AuthenticationState::Unauthenticated,
            Some("execution.auth-required"),
        ),
        Ok(AuthenticationState::Unknown) => {
            (AuthenticationState::Unknown, Some("execution.auth-unknown"))
        }
        Ok(AuthenticationState::Unsupported) => (AuthenticationState::Unsupported, None),
        Err(AuthenticationProbeError::Timeout) => (
            AuthenticationState::Unknown,
            Some("execution.auth-probe-timeout"),
        ),
        Err(AuthenticationProbeError::Failed) => (
            AuthenticationState::Unknown,
            Some("execution.auth-probe-failed"),
        ),
        Err(AuthenticationProbeError::Unknown) => {
            (AuthenticationState::Unknown, Some("execution.auth-unknown"))
        }
        Err(AuthenticationProbeError::OutputUnrecognized) => (
            AuthenticationState::Unknown,
            Some("execution.auth-output-unrecognized"),
        ),
        Err(AuthenticationProbeError::CliIncompatible) => (
            AuthenticationState::Unknown,
            Some("execution.auth-cli-incompatible"),
        ),
    };
    AuthenticationProbeResult { state, reason_code }
}

#[derive(Debug, Clone, Copy)]
enum Pipe {
    Stdout,
    Stderr,
}

struct PipeCapture {
    bytes: Vec<u8>,
    oversized: bool,
}

fn spawn_reader<R: Read + Send + 'static>(
    reader: R,
    pipe: Pipe,
    sender: Sender<(Pipe, Result<PipeCapture, ProbeError>)>,
) {
    thread::spawn(move || {
        let mut reader = reader;
        let mut bytes = Vec::with_capacity(OUTPUT_LIMIT + 1);
        let mut oversized = false;
        let mut buffer = [0; 4096];
        let result = loop {
            match reader.read(&mut buffer) {
                Ok(0) => break Ok(PipeCapture { bytes, oversized }),
                Ok(count) => {
                    let available = (OUTPUT_LIMIT + 1).saturating_sub(bytes.len());
                    bytes.extend_from_slice(&buffer[..count.min(available)]);
                    oversized |= count > available || bytes.len() > OUTPUT_LIMIT;
                }
                Err(_) => break Err(ProbeError::InvalidOutput),
            }
        };
        let _ = sender.send((pipe, result));
    });
}

fn receive_pipes(
    receiver: Receiver<(Pipe, Result<PipeCapture, ProbeError>)>,
    deadline: Instant,
) -> Result<(Vec<u8>, Vec<u8>), ProbeError> {
    let mut stdout = None;
    let mut stderr = None;
    while stdout.is_none() || stderr.is_none() {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(ProbeError::Timeout);
        }
        let (pipe, result) = receiver
            .recv_timeout(remaining)
            .map_err(|_| ProbeError::Timeout)?;
        let capture = result?;
        if capture.oversized || capture.bytes.len() > OUTPUT_LIMIT {
            return Err(ProbeError::OutputTooLarge);
        }
        match pipe {
            Pipe::Stdout => stdout = Some(capture.bytes),
            Pipe::Stderr => stderr = Some(capture.bytes),
        }
    }
    Ok((stdout.unwrap_or_default(), stderr.unwrap_or_default()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn authentication_probe_errors_remain_distinct_from_not_logged_in() {
        let unauthenticated =
            classify_authentication_probe(Ok(AuthenticationState::Unauthenticated));
        assert_eq!(unauthenticated.state, AuthenticationState::Unauthenticated);
        assert_eq!(unauthenticated.reason_code, Some("execution.auth-required"));

        let timed_out = classify_authentication_probe(Err(AuthenticationProbeError::Timeout));
        assert_eq!(timed_out.state, AuthenticationState::Unknown);
        assert_eq!(timed_out.reason_code, Some("execution.auth-probe-timeout"));

        let failed = classify_authentication_probe(Err(AuthenticationProbeError::Failed));
        assert_eq!(failed.state, AuthenticationState::Unknown);
        assert_eq!(failed.reason_code, Some("execution.auth-probe-failed"));

        let unknown = classify_authentication_probe(Err(AuthenticationProbeError::Unknown));
        assert_eq!(unknown.state, AuthenticationState::Unknown);
        assert_eq!(unknown.reason_code, Some("execution.auth-unknown"));

        let output =
            classify_authentication_probe(Err(AuthenticationProbeError::OutputUnrecognized));
        assert_eq!(output.state, AuthenticationState::Unknown);
        assert_eq!(
            output.reason_code,
            Some("execution.auth-output-unrecognized")
        );

        let incompatible =
            classify_authentication_probe(Err(AuthenticationProbeError::CliIncompatible));
        assert_eq!(incompatible.state, AuthenticationState::Unknown);
        assert_eq!(
            incompatible.reason_code,
            Some("execution.auth-cli-incompatible")
        );
    }

    #[test]
    fn output_capture_is_bounded_and_does_not_include_full_overflow() {
        let cursor = Cursor::new(vec![b'x'; OUTPUT_LIMIT + 200]);
        let mut bounded = cursor.take((OUTPUT_LIMIT + 1) as u64);
        let mut bytes = Vec::new();
        bounded.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes.len(), OUTPUT_LIMIT + 1);
        assert_eq!(
            ProbeError::OutputTooLarge.user_message(),
            "版本输出超过安全上限，已丢弃"
        );
        let (sender, receiver) = mpsc::channel();
        sender
            .send((
                Pipe::Stdout,
                Ok(PipeCapture {
                    bytes: vec![b'x'; OUTPUT_LIMIT + 1],
                    oversized: true,
                }),
            ))
            .unwrap();
        sender
            .send((
                Pipe::Stderr,
                Ok(PipeCapture {
                    bytes: Vec::new(),
                    oversized: false,
                }),
            ))
            .unwrap();
        assert_eq!(
            receive_pipes(receiver, Instant::now() + Duration::from_secs(1)),
            Err(ProbeError::OutputTooLarge)
        );
    }

    #[test]
    fn reader_keeps_bounded_prefix_and_drains_all_overflow() {
        let (sender, receiver) = mpsc::channel();
        spawn_reader(
            Cursor::new(vec![b'x'; OUTPUT_LIMIT + 100_000]),
            Pipe::Stdout,
            sender,
        );
        let (pipe, result) = receiver.recv_timeout(Duration::from_secs(1)).unwrap();
        let capture = result.unwrap();
        assert!(matches!(pipe, Pipe::Stdout));
        assert_eq!(capture.bytes.len(), OUTPUT_LIMIT + 1);
        assert!(capture.oversized);
    }

    #[test]
    fn process_errors_have_stable_codes_without_output_details() {
        assert_eq!(ProbeError::Timeout.code(), "tool.probe-timeout");
        assert_eq!(ProbeError::NonZeroExit.code(), "tool.version-unavailable");
        assert!(!ProbeError::SpawnFailure.user_message().contains("secret"));
    }
}
