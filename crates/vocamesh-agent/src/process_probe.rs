use std::{
    io::{Read, Take},
    path::Path,
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

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
            Self::OutputTooLarge | Self::UnsupportedLauncher | Self::InvalidOutput => {
                "tool.version-unavailable"
            }
            Self::SpawnFailure => "tool.scan-failed",
        }
    }

    pub const fn user_message(self) -> &'static str {
        match self {
            Self::Timeout => "固定版本探测命令超时",
            Self::NonZeroExit => "固定版本探测命令返回失败状态",
            Self::OutputTooLarge => "版本输出超过安全上限，已丢弃",
            Self::UnsupportedLauncher => "当前平台无法安全启动该 CLI shim",
            Self::SpawnFailure => "无法启动固定版本探测命令",
            Self::InvalidOutput => "版本输出无法安全解码",
        }
    }
}

pub trait ProcessProbe: Send + Sync {
    fn version(&self, executable: &Path) -> Result<ProcessOutput, ProbeError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CliProcessProbe;

impl ProcessProbe for CliProcessProbe {
    fn version(&self, executable: &Path) -> Result<ProcessOutput, ProbeError> {
        if crate::path_resolver::is_windows_script(executable) {
            return Err(ProbeError::UnsupportedLauncher);
        }
        let mut child = Command::new(executable)
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
}

#[derive(Debug, Clone, Copy)]
enum Pipe {
    Stdout,
    Stderr,
}

fn spawn_reader<R: Read + Send + 'static>(
    reader: R,
    pipe: Pipe,
    sender: Sender<(Pipe, Result<Vec<u8>, ProbeError>)>,
) {
    thread::spawn(move || {
        let mut bounded: Take<R> = reader.take((OUTPUT_LIMIT + 1) as u64);
        let mut bytes = Vec::with_capacity(OUTPUT_LIMIT + 1);
        let result = bounded
            .read_to_end(&mut bytes)
            .map(|_| bytes)
            .map_err(|_| ProbeError::InvalidOutput);
        let _ = sender.send((pipe, result));
    });
}

fn receive_pipes(
    receiver: Receiver<(Pipe, Result<Vec<u8>, ProbeError>)>,
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
        let bytes = result?;
        if bytes.len() > OUTPUT_LIMIT {
            return Err(ProbeError::OutputTooLarge);
        }
        match pipe {
            Pipe::Stdout => stdout = Some(bytes),
            Pipe::Stderr => stderr = Some(bytes),
        }
    }
    Ok((stdout.unwrap_or_default(), stderr.unwrap_or_default()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

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
            .send((Pipe::Stdout, Ok(vec![b'x'; OUTPUT_LIMIT + 1])))
            .unwrap();
        sender.send((Pipe::Stderr, Ok(Vec::new()))).unwrap();
        assert_eq!(
            receive_pipes(receiver, Instant::now() + Duration::from_secs(1)),
            Err(ProbeError::OutputTooLarge)
        );
    }

    #[test]
    fn process_errors_have_stable_codes_without_output_details() {
        assert_eq!(ProbeError::Timeout.code(), "tool.probe-timeout");
        assert_eq!(ProbeError::NonZeroExit.code(), "tool.version-unavailable");
        assert!(!ProbeError::SpawnFailure.user_message().contains("secret"));
    }
}
