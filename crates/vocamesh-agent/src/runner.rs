//! Static provider dispatch. This crate builds fixed commands and parses bounded
//! provider output; Runtime remains responsible for process and task lifecycle.

use crate::execution::{
    classify_cli_stderr, AuthenticationProbeError, ClaudeCli, ClaudeOutput, ExecutionEnvironment,
};
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
};
use vocamesh_core::{AuthenticationState, Provider, ServiceError};

#[path = "runners/codex.rs"]
pub mod codex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderOutput {
    TextDelta(String),
    Final(String),
    FinalAndComplete(String),
    ToolProgress(&'static str),
    ProtocolComplete,
    ProtocolFailed,
    ProtocolError,
    Ignore,
}

pub trait ProviderRunner: Send + Sync {
    fn provider(&self) -> Provider;
    fn executable(&self) -> &Path;
    fn version(&self) -> Option<String>;
    fn probe(&self) -> Result<AuthenticationState, ServiceError>;
    fn launch_command(&self, prompt: &str, working_directory: &Path) -> Command;
    fn prompt_on_stdin(&self) -> bool;
    fn classify_failure(&self, stderr: &[u8]) -> &'static str;
}

pub fn parse_provider_output(provider: Provider, line: &[u8]) -> ProviderOutput {
    match provider {
        Provider::Claude => map_claude_output(crate::execution::parse_stream_json(line)),
        Provider::Codex => codex::parse_jsonl_event(line),
        Provider::Opencode => ProviderOutput::ProtocolError,
    }
}

pub fn runner_for(
    provider: Provider,
    executable: PathBuf,
    environment: ExecutionEnvironment,
) -> Result<Arc<dyn ProviderRunner>, ServiceError> {
    match provider {
        Provider::Claude => Ok(Arc::new(ClaudeCodeRunner::new(executable, environment)?)),
        Provider::Codex => Ok(Arc::new(codex::CodexCliRunner::new(
            executable,
            environment,
        )?)),
        Provider::Opencode => Err(ServiceError::new(
            "execution.adapter-unavailable",
            "OpenCode 当前没有受控执行适配器",
        )),
    }
}

struct ClaudeCodeRunner {
    executable: PathBuf,
    environment: ExecutionEnvironment,
    cli: ClaudeCli,
}

impl ClaudeCodeRunner {
    fn new(executable: PathBuf, environment: ExecutionEnvironment) -> Result<Self, ServiceError> {
        let cli = ClaudeCli::verify_with_environment(&executable, environment.clone())?;
        Ok(Self {
            executable,
            environment,
            cli,
        })
    }
}

impl ProviderRunner for ClaudeCodeRunner {
    fn provider(&self) -> Provider {
        Provider::Claude
    }

    fn executable(&self) -> &Path {
        &self.executable
    }

    fn version(&self) -> Option<String> {
        let (major, minor, patch) = self.cli.version();
        Some(format!("{major}.{minor}.{patch}"))
    }

    fn probe(&self) -> Result<AuthenticationState, ServiceError> {
        let cli = ClaudeCli::verify_with_environment(&self.executable, self.environment.clone())?;
        cli.authentication_probe_state()
            .map_err(|error| match error {
                AuthenticationProbeError::Timeout => ServiceError::new(
                    "execution.auth-probe-timeout",
                    "Claude Code 认证状态探测超时",
                ),
                AuthenticationProbeError::Failed => ServiceError::new(
                    "execution.auth-probe-failed",
                    "Claude Code 认证状态探测失败",
                ),
                AuthenticationProbeError::Unknown => {
                    ServiceError::new("execution.auth-unknown", "无法确认 Claude Code 登录状态")
                }
                AuthenticationProbeError::OutputUnrecognized => ServiceError::new(
                    "execution.auth-output-unrecognized",
                    "Claude Code 认证状态输出无法识别",
                ),
                AuthenticationProbeError::CliIncompatible => ServiceError::new(
                    "execution.auth-cli-incompatible",
                    "当前 Claude Code 版本不支持受控认证探测",
                ),
            })
    }

    fn launch_command(&self, prompt: &str, working_directory: &Path) -> Command {
        let mut command = Command::new(&self.executable);
        command
            .args(ClaudeCli::read_only_args(prompt))
            .current_dir(working_directory)
            .stdin(Stdio::null())
            .env_clear();
        self.environment.apply(&mut command);
        command
    }

    fn prompt_on_stdin(&self) -> bool {
        false
    }

    fn classify_failure(&self, stderr: &[u8]) -> &'static str {
        classify_cli_stderr(stderr).error_code()
    }
}

fn map_claude_output(output: ClaudeOutput) -> ProviderOutput {
    match output {
        ClaudeOutput::TextDelta(text) => ProviderOutput::TextDelta(text),
        ClaudeOutput::Final(text) => ProviderOutput::FinalAndComplete(text),
        ClaudeOutput::ToolProgress(text) => ProviderOutput::ToolProgress(text),
        ClaudeOutput::ProtocolError => ProviderOutput::ProtocolError,
        ClaudeOutput::Ignore => ProviderOutput::Ignore,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_never_falls_back_for_opencode() {
        let result = runner_for(
            Provider::Opencode,
            PathBuf::from("/unused"),
            ExecutionEnvironment::default(),
        );
        let error = match result {
            Ok(_) => panic!("OpenCode unexpectedly resolved to a runner"),
            Err(error) => error,
        };
        assert_eq!(error.code, "execution.adapter-unavailable");
    }

    #[cfg(unix)]
    #[test]
    fn provider_environment_does_not_pass_home_to_codex() {
        let environment = ExecutionEnvironment::current_for(Provider::Codex);
        let mut command = Command::new("/usr/bin/env");
        environment.apply(&mut command);
        let output = command.output().unwrap();
        assert!(output.status.success());
        let keys = String::from_utf8(output.stdout).unwrap();
        assert!(!keys.lines().any(|line| line.starts_with("HOME=")));
        assert!(!keys.lines().any(|line| line.starts_with("CODEX_HOME=")));
    }

    #[test]
    fn claude_parser_maps_its_final_record_to_a_completed_result() {
        assert_eq!(
            map_claude_output(ClaudeOutput::Final("fixture".into())),
            ProviderOutput::FinalAndComplete("fixture".into())
        );
    }
}
