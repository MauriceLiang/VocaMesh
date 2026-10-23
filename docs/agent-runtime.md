# 本机 Agent CLI 运行时

## 支持范围与安全就绪状态

只有 Claude Code 有单 Agent Runtime 实现。Codex/OpenCode 仍可发现，但不能执行。Windows 真实执行固定关闭。当前 Unix/macOS readiness 也因 managed hooks 策略无法可靠核实而 fail-closed；用户界面和后端任务入口都会拒绝启动。受限参数和 fake CLI 测试不等于真实执行安全门禁通过。

CLI 必须是绝对路径上的普通可执行文件，最低版本为 `2.1.259`：`--restricted` 从 `2.1.248` 起可用，`--permission-prompts none` 从 `2.1.259` 起可用。低版本、版本探测失败、认证状态不明或策略门禁未通过时拒绝执行，不回退到默认权限。本机 Claude Code `2.1.296` 是先前验收轮次记录的版本；本轮没有重新运行认证探测或真实 CLI 任务。

## managed hooks 门禁

受限参数只能约束 CLI 的工具/配置来源，不能证明整个 CLI 进程没有其他副作用。当前 Claude Code 官方 CLI 文档说明：`--restricted` 仍加载 managed settings；`--safe-mode` 会停用用户/项目自定义项，但组织 managed policy 中的 hooks 仍可能运行。官方还说明 managed settings 可由系统/MDM 配置或 Claude 管理端下发，且 hooks 可运行 Shell 命令。

VocaMesh 没有受支持的非交互 preflight 能力来可靠判断当前会话是否可能运行 managed hooks。Runtime 不读取管理配置正文或认证材料，也不使用非受支持参数试图绕过策略。因此 readiness 固定返回 `execution.managed-policy-unverified`，任务入口重复拒绝。单元测试的内部策略注入只用于覆盖 fake CLI 生命周期；没有生产配置路径可把该值设为 verified。

官方依据：[CLI reference](https://code.claude.com/docs/en/cli-reference)、[settings files and precedence](https://code.claude.com/docs/en/settings)、[hooks guide](https://code.claude.com/docs/en/hooks-guide)。

## CLI 探测与统一环境

`--version`、`auth status` 和任务进程使用同一份 `ExecutionEnvironment` allowlist，避免探测通过、实际任务环境却不同。环境仅继承 `PATH`、用户目录、临时目录、locale、终端标识与 `CLAUDE_CONFIG_DIR`，并清除其他环境变量。认证探测使用固定 `claude auth status` 参数，最多保留 16 KiB stdout 并只解析退出码和 `authMethod`：退出码 0 且方法是 `claude.ai` 或 `oauth_token` 才分类为已认证；退出码 1 且方法为 `none` 才分类为未认证；其他组合均为 `unknown`。

任务 `stderr` 最多在内存保留 16 KiB，同时持续排空剩余管道。纯分类器只返回固定的无效参数、认证、配置或一般失败类别，对应稳定错误码；原始 stderr 不进入数据库、事件、UI 或日志。版本/认证探测的 stderr 也只排空并丢弃。

## 固定只读启动参数

Prompt 作为独立 `-p` 参数，不经过 Shell。Rust 测试精确断言完整参数集合：

- `--restricted`、`--safe-mode`；
- `--permission-mode plan`、`--permission-prompts none`；
- `--tools Read,Glob,Grep`、`--disallowedTools mcp__*`；
- `--settings` 内联 deny 规则拒绝常见凭据路径；
- `--no-session-persistence`、`--output-format stream-json`、`--include-partial-messages`、`-p <prompt>`。

不使用 Shell 文本、写权限、MCP 工具、子 Agent 或危险权限绕过。`--safe-mode` 不应被描述为关闭 managed hooks；因此实际执行仍受上述策略门禁控制。官方 CLI 文档链接同时说明 `--restricted` 的 managed settings 行为与 `--safe-mode` 的 managed policy 例外。

## 快照与进程生命周期

Runtime 先规范化并拒绝过宽工作区，再在 Unix/macOS 打开并保留根目录描述符。目录和普通文件通过 `openat`/`O_NOFOLLOW` 与 `fstat` 做相对打开及对象校验；复制期间限制实际读取量，整个快照最多 100,000 个文件、512 MiB。敏感目录/文件按名称排除；符号链接、特殊文件、路径替换、权限错误或其他不能安全处理的输入会阻止整个快照，不能把部分快照交给 CLI。目标目录必须不存在。

Unix 进程以独立进程组启动。停止/超时先 `SIGTERM`，等待 2 秒后 `SIGKILL`；无论主进程是否正常退出，都检查进程组是否仍有成员。残留后代会被终止，确认后将任务记为失败；无法确认退出时 Run 记为 `unknown`。

只有终态写入 SQLite 成功且结果并非 `unknown` 后才清理快照。无法确认进程树、状态查询失败或终态写入失败时保留快照和已有 PID/进程组号，清除内存停止通道，但 SQLite `unknown`/active 状态阻止后续任务。

SQLite schema v2 增加 `process_group_id`、`recovery_status`（`none`、`required`、`resolved`、`manual_required`）和 `recovery_checked_at`。v1→v2 migration 使用事务并保留所有 Task、Run、Event；未来 schema 版本拒绝打开，不降级、不重建。旧 active Run 进入 `unknown` 并产生固定 `recovery-required` 事件；没有持久化 PGID 的历史 Run 只能进入 `manual_required`。

Runtime 提供 `list_unknown_runs`、`inspect_unknown_run` 和 `resolve_unknown_run`。检查仅报告进程组状态并更新最近检查时间，不改变 Run 结果、不杀进程；`ESRCH` 只产生 `exited-candidate`。PGID 存在、状态无法核验、PGID 缺失或平台不支持时，Unknown Run、快照和启动阻塞都保留。进程组退出也不能证明脱离原组的后代不存在。

归档需要用户在操作系统进程管理工具中人工核验并明确确认。Runtime 在 `start_gate` 内重新检查 PGID、事务更新任务/Run 并写入审计事件；Run 归档为失败，原 `error_code` 保留。数据库提交后才删除由受信任数据目录和 UUID 请求 ID 派生的快照；Unix 清理通过 `O_NOFOLLOW` 目录描述符和 `unlinkat` 执行，清理失败留下固定审计事件并经 `RecoveryResolution.snapshotStatus` 返回，不会将 Run 改为成功或自动重跑。恢复状态、PGID 和最近检查时间不会改变 `execution.managed-policy-unverified` 的生产门禁。

`ExecutionReadiness.reasonCode` 区分 Unknown Run、managed policy、CLI 缺失/不兼容、认证未知/未登录、平台禁用与存储错误。存储检查错误不会被归类成进程未知；策略门禁仍先于 CLI 认证探测。桌面恢复面板展示短 Run ID、任务标题、状态、错误码、快照存在状态和操作指引；不展示绝对路径、不接受前端进程标识，不提供强杀、删快照解锁或重试。

## 输出与结果

stdout 按最多 64 KiB 单行的 JSONL 解析，只发布允许的文本增量、final 摘要和固定工具进度消息。未知事件丢弃，协议损坏只生成固定错误文本；疑似凭据的可见文本整条遮蔽。Run 事件最多 2 MiB，超限只保存一次截断标记后继续排空。成功要求 exit code 0 和合法 final 事件；非零退出、缺 final、协议错误、超时、取消、进程树未知和数据库异常不自动重试。

`implementation-report.md` 记录本地自动化证据、managed policy 阻塞、一次先前失败的真实 CLI 任务，以及 macOS GUI、Windows 和 Linux 的未验收范围。Fake CLI 测试不代表真实模型/账户、费用或桌面 GUI 已验收。
