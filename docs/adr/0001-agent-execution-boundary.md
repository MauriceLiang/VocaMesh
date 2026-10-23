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

## v0.5 本机发现补充证据与待决事项

本阶段只补充本地 CLI 发现和能力表达，没有改变上述执行决策。Claude Code 的安全策略仍无法由 VocaMesh 可靠核验，所以认证探测成功、偏好保存和本地版本发现均不会解除 `execution.managed-policy-unverified`。

Codex CLI 官方文档说明 `codex login status` 输出当前认证模式并以退出码表示是否存在凭据，但没有定义机器可读字段契约；CLI 实现会通过认证管理器读取凭据状态，部分认证方式会读取并打印掩码后的 API key。VocaMesh 不运行该命令，也不读取其输出。OpenCode v1 CLI 文档将 `opencode auth list` 描述为列出存储的 Provider；v2 文档另外说明 `--format json`。审查到的 v2 实现会连接 OpenCode server、先请求模型 readiness，再读取 integration connections，并输出 Provider 名称与 credential/environment 来源标签；这些连接会涉及服务器配置的项目目录/环境范围，不能作为 VocaMesh 所需的本地、隔离、可版本约束认证探测。VocaMesh 因而不运行该命令，不读取配置/凭据内容，也不把登录状态标成已验证。依据（查阅日期：2026-10-23）：[Codex CLI reference](https://developers.openai.com/codex/cli/reference)、[Codex login implementation](https://github.com/openai/codex/blob/main/codex-rs/cli/src/login.rs)、[OpenCode v1 CLI](https://dev.opencode.ai/docs/cli/)、[OpenCode v2 CLI](https://opencode.ai/v2/docs/cli/commands)、[OpenCode auth list implementation](https://github.com/anomalyco/opencode/blob/4d30794bcf4dccdcadbcb1f62bb35b4943a96426/packages/cli/src/commands/handlers/auth/list.ts)、[OpenCode auth command helper](https://github.com/anomalyco/opencode/blob/4d30794bcf4dccdcadbcb1f62bb35b4943a96426/packages/cli/src/commands/handlers/auth/shared.ts)。

后续路线需要维护者基于完整责任面决策；本阶段不替维护者选择或实施 B/C：

| 维度 | A. 继续等待 CLI 可核验边界 | B. 直接 API Provider | C. CLI 置于 OS 沙箱/VM |
| --- | --- | --- | --- |
| Provider 范围 | 继续 Claude Code CLI；等待提供方可验证策略/Hook 边界 | 需选定 API 模型/Provider，并实现受控工具协议 | 继续 CLI，但另行定义沙箱支持的 Provider 与版本 |
| 凭据与费用 | 沿用用户 CLI 账户；费用由 CLI 计划和提供方决定 | 用户另配 API 凭据；可能产生 CLI 订阅外的 API 费用 | 仍需把 CLI 凭据放入可访问环境；另有 VM/沙箱资源成本 |
| Prompt、文件与日志数据流 | CLI 及其托管策略控制网络行为；VocaMesh 的只读快照和日志边界继续生效 | Prompt/允许读取的文件经 VocaMesh 的 API 传输层发给 Provider，需定义请求/响应/日志保留 | CLI 在受限文件系统中访问快照；仍需验证凭据、输出与网络出口 |
| 网络策略 | 当前不能证明 Hooks 或 CLI 的完整网络边界 | 需明确出口域名、代理、重试与遥测策略 | 需验证操作系统网络隔离是否能约束 CLI 与 Hook |
| 授权撤销 | 由 CLI/Provider 管理登录态；VocaMesh 无法统一撤销 | 需实现本机密钥存储、清除、轮换和 Provider 侧撤销说明 | 需定义沙箱内凭据注入、擦除、轮换和会话销毁 |
| 恢复语义 | 保留现有 Task/Run、停止与 Unknown Run 恢复；不自动重试 | 定义 API 超时/部分结果/幂等请求与计费后的重试语义 | 在沙箱崩溃、VM 快照和主机重启后证明进程清理与 Unknown Run 恢复 |
| 平台与维护成本 | 等待官方能力，并逐平台验收 | 维护 API 协议、密钥/计费/网络和数据处理；逐平台验收 | 分别维护 macOS、Windows、Linux 沙箱与打包/更新恢复流程 |

当前选择仍是 A。要进入 B，维护者需明确接受独立凭据、计费、网络和数据流设计；要进入 C，需提供逐平台文件、进程、凭据、网络、更新及恢复验收。以上材料是决策准备，不构成方案批准。

## v0.6 认证诊断与 readiness 补充决策

本阶段仍采用方案 A：Claude Code CLI 生产执行保持 fail-closed。`execution.managed-policy-unverified` 仍为生产策略状态；认证成功只更新认证诊断，不授权任务、不开放策略开关，也不引入 API Provider 或 OS 沙箱。

### 已核实事实

- 官方 [Claude Code CLI reference](https://code.claude.com/docs/en/cli-reference) 为 `claude auth status` 定义 JSON 输出、登录/未登录退出状态和 `loggedIn`、`authMethod` 字段；已认证方法枚举包含 `claude.ai`、`oauth_token`、`api_key`、`api_key_helper`、`third_party`，未认证方法为 `none`。
- 本机 Claude Code 版本 `2.1.296`。在与应用相同的环境 allowlist、固定参数、5 秒超时和 16 KiB 输出上限下，只读调用一次 `claude auth status`：退出码类别为成功（0），格式为 JSON object，`loggedIn=true`、`authMethod=oauth_token`，解析为已认证。没有保存或展示原始输出、邮箱、组织标识或配置路径；没有发送 Prompt 或发起模型请求。
- 认证解析要求 `loggedIn`、`authMethod` 和退出码相互一致；格式不识别、字段矛盾、未知退出状态、超时、探测失败和 CLI 参数不兼容分别保持未知并使用稳定原因码。普通扫描不会触发认证探测。
- Runtime readiness 同时返回存储、活动/未知 Run、平台、CLI 发现/版本、认证、managed policy 与任务启动检查；策略与认证状态独立。即使认证已确认，只要 managed policy 未核验，`ready=false`，启动入口仍会重新执行硬门禁检查。

### 仍未知或未验收

本机认证探测不能证明 managed hooks 的副作用边界，也不能证明真实任务可以成功运行。没有启动模型任务。macOS 原生 GUI 验收因 Computer Use 未获准访问 VocaMesh 而 BLOCKED；Windows、Linux、Playwright E2E 和远端 CI 本轮未运行。以上证据不改变方案 A，也不授权路线 B/C。
