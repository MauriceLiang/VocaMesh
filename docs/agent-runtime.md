# 本机 Agent CLI 运行时

## v0.7 ProviderRunner 与 Codex Adapter（当前）

Runtime 通过静态 Provider 分发使用统一的 ProviderRunner；ClaudeCodeRunner 复用既有 ClaudeCli 参数、探测和 JSONL 解析，任务生命周期仍只有一个 Runtime、StartGate、SQLite schema v3、停止/超时和 Unknown Run 恢复实现。CodexCliRunner 独立校验可执行文件、版本及当前 exec --help 必需选项，不复用 Claude 的认证或安全结论；OpenCode 没有 Runner。

Codex CLI 版本 0.162.1 和当前 codex exec --help 只读探测已检查。启动命令用独立参数构造 codex exec --json --sandbox read-only --ignore-user-config --ignore-rules --ephemeral --cd <快照目录> -，Prompt 经 stdin 传递，不经过 Shell。JSONL 只接受已知 thread/turn/item 事件；只提取 agent_message 文本，完成必须同时具备允许的摘要事件、turn.completed 及零退出码。失败、缺少协议终态、未知/畸形/超长事件均不能成功；原始未知事件和错误内容不持久化。

Codex 子进程环境不继承 HOME、USERPROFILE、APPDATA、LOCALAPPDATA 和 CODEX_HOME，但这不证明 CLI 无法通过系统用户信息或其它机制发现配置、凭据、插件/MCP、可读文件、子进程或网络出口。Codex 认证保持 Unsupported；生产 readiness 使用独立的 execution.codex-safety-unverified，启动入口在创建 Task/Run 前重查并阻断。真实 Codex 任务未运行。测试专用策略只在 cfg(test) 构建中存在。

Claude 认证探测的当前机器可读契约记录于官方 [Claude Code CLI usage 文档](https://code.claude.com/docs/en/cli-usage)：固定命令输出 JSON 的 authMethod，退出码 0 表示已登录、1 表示未登录；已知方法为 none、claude.ai、oauth_token、api_key、api_key_helper、third_party。解析器只在“退出码 0 + 已知非 none 方法”时返回已认证，或“退出码 1 + none”时返回未认证；矛盾、未知、格式错误均返回 unknown。经一次明确授权，本机 Claude Code 2.1.296 的只读 claude auth status 分类为 authenticated、退出码 0；原始 stdout/stderr 未保存，configDirectory 等其它字段不读取。认证成功不解除 Claude 的 managed policy 门禁。

## v0.5 本机工具发现与能力状态

Rust Core 的 Provider 能力表是 Claude Code、Codex CLI、OpenCode 的单一能力来源：包含 Provider ID、显示名、可执行文件名、发现/版本探测能力、VocaMesh 执行适配器、认证探测能力、可证明的最低版本和稳定阻断原因。Claude Code 的最低版本为 `2.1.259`；Codex/OpenCode 只报告实际解析到的版本，不据此推断兼容执行。当前只有 Claude Code 有适配器。

`LocalTool` 将发现、版本、认证、集成能力和 Runtime readiness 分开序列化。路径无效、探测失败、版本未知/不兼容、认证未知/未登录、适配器缺失和策略阻断分别通过枚举及 `reasonCode` 表达。`note` 仅用于界面文案。字段不持久化，因此不修改 SQLite schema v3。Agent 发现层不推断全局执行策略；Tauri 普通扫描、认证验证和保存路径时基于已有 `LocalTool` 调用 `ExecutionRuntime::readiness_for_tool()`，不会重复启动认证探测。显式请求完整 readiness 时才调用 `ExecutionRuntime::readiness()` 并执行一次只读预检。

发现按进程 `PATH` 顺序优先，再按固定顺序检查 macOS `/opt/homebrew/bin`、`/usr/local/bin`、Home 下已有的 `.local/bin`、`.bun/bin`，以及 `.cargo/bin`、`.volta/bin`、`.asdf/shims`、`.local/share/mise/shims`、`.npm-global/bin`。不运行 Shell、读取 Shell 配置或递归扫描 Home。候选目录规范化后去重；结果保留展示路径和 canonical 目标路径。手动路径必须为 Provider 文件名匹配的绝对可执行普通文件；保存只验证路径，不启动程序，之后重新检测才会直接启动该程序并传入固定 `--version`。版本/认证输出均有上限、超时和固定错误码，不记录原始输出。Windows `.cmd`/`.bat` shim 继续安全拒绝。

工具页的“设为偏好”仅保存偏好：扫描或 Runtime 阻断不会清除此值；只有具有执行适配器、路径已发现且版本已解析的 Provider 可设为偏好。偏好不等于授权。任务入口仍只由 Runtime readiness 控制，并在 Rust 启动入口复查。Tauri 的缺失时间和来源不冒充演示数据，IPC 错误不回退到 Mock；浏览器 Mock 继续独立运行。

认证探测复用 `ClaudeCli` 的固定 `claude auth status`、清理后环境、有界 stdout/stderr 和严格 JSON/退出码解析。JSON 必须包含布尔 `loggedIn` 与已知 `authMethod`；登录字段、方法和退出码互相矛盾时保持未知，格式不识别、超时、探测失败及 CLI 参数不兼容分别使用稳定原因码。认证成功不会改变 `execution.managed-policy-unverified`。Codex/OpenCode 的认证状态保持 unsupported/unknown，不运行登录态命令、不读配置/凭据、不发起模型请求。官方契约和本机只读探测证据见 ADR 0001 v0.6 补充记录。

## 支持范围与安全就绪状态

只有 Claude Code 有单 Agent Runtime 实现。Codex/OpenCode 仍可发现，但不能执行。Windows 真实执行固定关闭。当前 Unix/macOS readiness 也因 managed hooks 策略无法可靠核实而 fail-closed；用户界面和后端任务入口都会拒绝启动。受限参数和 fake CLI 测试不等于真实执行安全门禁通过。

CLI 必须是绝对路径上的普通可执行文件，最低版本为 `2.1.259`：`--restricted` 从 `2.1.248` 起可用，`--permission-prompts none` 从 `2.1.259` 起可用。低版本、版本探测失败、认证状态不明或策略门禁未通过时拒绝执行，不回退到默认权限。本轮固定只读探测记录 Claude Code `2.1.296` 与认证已确认；没有启动真实模型任务。

## managed hooks 门禁

受限参数只能约束 CLI 的工具/配置来源，不能证明整个 CLI 进程没有其他副作用。当前 Claude Code 官方 CLI 文档说明：`--restricted` 仍加载 managed settings；`--safe-mode` 会停用用户/项目自定义项，但组织 managed policy 中的 hooks 仍可能运行。官方还说明 managed settings 可由系统/MDM 配置或 Claude 管理端下发，且 hooks 可运行 Shell 命令。

VocaMesh 没有受支持的非交互 preflight 能力来可靠判断当前会话是否可能运行 managed hooks。Runtime 不读取管理配置正文或认证材料，也不使用非受支持参数试图绕过策略。因此 readiness 固定返回 `execution.managed-policy-unverified`，任务入口重复拒绝。单元测试的内部策略注入只用于覆盖 fake CLI 生命周期；没有生产配置路径可把该值设为 verified。

官方依据：[CLI reference](https://code.claude.com/docs/en/cli-reference)、[settings files and precedence](https://code.claude.com/docs/en/settings)、[hooks guide](https://code.claude.com/docs/en/hooks-guide)。

## CLI 探测与统一环境

`--version`、`auth status` 和任务进程使用同一份 `ExecutionEnvironment` allowlist，避免探测通过、实际任务环境却不同。环境仅继承 `PATH`、用户目录、临时目录、locale、终端标识与 `CLAUDE_CONFIG_DIR`，并清除其他环境变量。认证探测使用固定 `claude auth status` 参数，最多保留 16 KiB stdout/stderr。解析器要求 JSON 对象中的 `loggedIn` 为布尔值、`authMethod` 为官方已知枚举（`none`、`claude.ai`、`oauth_token`、`api_key`、`api_key_helper`、`third_party`），并要求字段与退出码一致：退出码 0、`loggedIn=true` 且方法非 `none` 才分类为已认证；退出码 1、`loggedIn=false` 且方法为 `none` 才分类为未认证。有效但不能判断的退出状态返回未知；缺字段、类型错误、未知枚举、矛盾或畸形输出返回 `execution.auth-output-unrecognized`。原始输出不进入错误消息、UI、日志或持久化存储。

任务 `stderr` 最多在内存保留 16 KiB，同时持续排空剩余管道。纯分类器只返回固定的无效参数、认证、配置或一般失败类别，对应稳定错误码；原始 stderr 不进入数据库、事件、UI 或日志。认证探测只用有界 stderr 分类 CLI 参数不兼容，其余原文随即丢弃；版本探测 stderr 只排空并丢弃。

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

`ExecutionReadiness` 保留兼容字段并附带独立的 `checks`、`blockers` 和 `policyStatus`；分别报告存储、活动 Run、Unknown Run/进程状态、平台、CLI 发现/版本、认证、managed policy 与任务启动结果。认证/CLI 预检与策略检查独立运行，首要 `reasonCode` 在存在策略阻断时仍优先报告 `execution.managed-policy-unverified`。`ready` 是所有硬门禁的合取；`start_read_only_task()` 不接受或复用 readiness 快照，仍在后端重新验证启动门禁。桌面恢复面板展示短 Run ID、任务标题、状态、错误码、快照存在状态和操作指引；不展示绝对路径、不接受前端进程标识，不提供强杀、删快照解锁或重试。

## 输出与结果

stdout 按最多 64 KiB 单行的 JSONL 解析，只发布允许的文本增量、final 摘要和固定工具进度消息。未知事件丢弃，协议损坏只生成固定错误文本；疑似凭据的可见文本整条遮蔽。Run 事件最多 2 MiB，超限只保存一次截断标记后继续排空。成功要求 exit code 0 和合法 final 事件；非零退出、缺 final、协议错误、超时、取消、进程树未知和数据库异常不自动重试。

`implementation-report.md` 记录本地自动化证据、managed policy 阻塞、一次先前失败的真实 CLI 任务，以及 macOS GUI、Windows 和 Linux 的未验收范围。Fake CLI 测试不代表真实模型/账户、费用或桌面 GUI 已验收。
