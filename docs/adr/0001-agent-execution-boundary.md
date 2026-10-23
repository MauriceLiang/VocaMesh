# ADR 0001：Agent 执行边界

- 状态：已采纳（继续 fail-closed；其他路线等待维护者决策）
- 日期：2026-10-23
- 适用阶段：v0.3.1

## 背景

VocaMesh 当前通过 Claude Code CLI 执行受限只读任务。`--restricted` 可移除部分命令执行工具并限制文件工具，但 CLI 仍加载 managed settings 和本次 `--settings`；`--safe-mode` 会关闭常规自定义项，却仍应用 managed policy 中配置的 hooks。Claude Code 官方文档也说明 hooks 可在生命周期中自动运行 Shell 命令，组织策略可由本机管理文件、MDM 或服务端策略提供。`--settings` 的优先级低于 managed settings，不能覆盖组织策略。

因此，VocaMesh 当前没有受支持的非交互检查，可以证明一次 CLI 会话不会运行额外的组织托管 hook 或产生超出 VocaMesh 工具白名单的副作用。扫描部分本机文件、检查环境变量、CLI 参数、用户承诺或人工勾选都不能覆盖服务端策略，也不能证明完整运行时副作用不存在。

依据（查阅日期：2026-10-23）：[CLI reference](https://code.claude.com/docs/en/cli-reference)、[settings files and precedence](https://code.claude.com/docs/en/settings)、[hooks guide](https://code.claude.com/docs/en/hooks-guide)。

## 决策

继续采用方案 A：Claude Code CLI 生产执行保持 fail-closed。`ManagedPolicyStatus::Unverified` 仍是生产默认值，readiness 和启动入口继续返回 `execution.managed-policy-unverified`。本 ADR 不加入生产 verified 配置、环境变量开关或用户风险自担开关，也不实现新的执行后端。

v0.3.1 的恢复 UI 只解决已存在 Unknown Run 的状态核验、人工确认归档和快照管理，不会开放新任务，不会把 Run 改为成功，也不会自动重试。恢复操作与当前 managed policy 门禁相互独立。

## 方案比较

| 方案 | 安全边界 | 凭据与网络 | 费用 | 跨平台代价 | 决策 |
| --- | --- | --- | --- | --- | --- |
| A. Claude Code CLI 保持 fail-closed | 不改变当前边界；VocaMesh 无法证明 managed hooks 不产生额外副作用，因此生产任务继续关闭 | 复用用户 CLI 登录态；CLI/Hook 的网络行为不能由 VocaMesh 完整核验 | 由用户现有 CLI 账户/计划决定，具体行为由提供方控制 | 当前实现已有 Unix 进程组与快照逻辑，但桌面/平台验收仍未完成 | **本阶段继续采用** |
| B. 新增直接 API Provider 和受控工具面 | VocaMesh 自己实现工具、权限与调用协议，不经过 Claude Code CLI hooks；仍需定义网络出口、模型响应和工具输入的边界 | 需要用户单独配置 API 凭据，并设计密钥存储、轮换、网络代理与日志脱敏 | 可能使用独立 API 计费，不能假设包含在 CLI 订阅内 | Provider 协议、密钥存储与工具沙箱需逐平台验证 | 只记录；需要维护者批准凭据和计费变化后再设计 |
| C. CLI 置于 OS 级沙箱或独立 VM | 可由操作系统限制文件、进程或网络，但必须证明凭据不可滥用、策略与沙箱组合有效；进程组不是沙箱 | CLI 仍可读取其授权范围内凭据；网络策略必须显式定义 | 沿用 CLI 账户模型，另有沙箱/VM 资源成本 | macOS、Windows、Linux 的沙箱 API、打包、更新、故障恢复和体验差异明显 | 只做可行性研究；不实现 |
| D. 用户勾选“我承担风险”继续执行 | 不改变 hook 能力，也不限制凭据访问或副作用；把未知安全风险转交用户 | 仍沿用 CLI 凭据与不确定网络行为 | 不变 | 表面实现简单，但安全验证问题没有解决 | **拒绝** |

## 后续决策所需证据

- CLI 路线只有在提供方给出受支持且可自动核验的策略/Hook 禁用边界，并通过故障注入与真实桌面验收后，才可以重新评估。配置扫描、版本号、环境值、`--safe-mode` 或用户确认均不是充分证据。
- 直接 API Provider 需要维护者明确接受单独 API 凭据、API 计费、密钥存储责任、网络请求策略和新的支持范围，再进入产品/安全设计。
- OS 沙箱路线需要平台逐项证明文件系统、进程、凭据、网络与更新边界，并提供 macOS/Windows/Linux 的可复现桌面验收；单纯启动 CLI 子进程不构成沙箱。
- 任一路线都需要明确日志/遥测保留、供应商数据处理、网络故障与用户可撤销授权的行为。

## 后果

- 当前真实 Claude Code 任务继续被 `execution.managed-policy-unverified` 阻断，v0.3 真实执行验收仍未完成。
- v0.3.1 可以安全检查和恢复历史 Unknown Run，但不会改变真实执行能力。
- 在维护者明确批准方案 B 或 C，或提供方给出足够的方案 A 证据之前，不启动新执行后端、不开放真实任务、不开始多 Agent 并行或写权限阶段。
