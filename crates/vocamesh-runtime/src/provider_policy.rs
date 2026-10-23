use vocamesh_core::{Provider, ServiceError};

const CLAUDE_POLICY_UNVERIFIED: ServiceError = ServiceError::new(
    "execution.managed-policy-unverified",
    "无法确认 Claude Code 的组织托管 Hook 是否会产生额外副作用；真实执行已禁用",
);
const CODEX_POLICY_UNVERIFIED: ServiceError = ServiceError::new(
    "execution.codex-safety-unverified",
    "Codex 的配置、凭据、文件、进程和网络边界尚未证明；真实执行已禁用",
);
const ADAPTER_UNAVAILABLE: ServiceError = ServiceError::new(
    "execution.adapter-unavailable",
    "该 Provider 没有受控执行适配器",
);

pub fn evaluate(
    provider: Provider,
    claude_managed_policy_verified: bool,
) -> Result<(), ServiceError> {
    match provider {
        Provider::Claude if claude_managed_policy_verified => Ok(()),
        Provider::Claude => Err(CLAUDE_POLICY_UNVERIFIED),
        Provider::Codex => Err(CODEX_POLICY_UNVERIFIED),
        Provider::Opencode => Err(ADAPTER_UNAVAILABLE),
    }
}
