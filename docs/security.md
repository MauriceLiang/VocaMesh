# 安全边界

## v0.7 Provider 独立执行门禁

Claude 与 Codex 的探测、认证状态和 Runtime 策略分开。Claude 的策略阻断码仍为 execution.managed-policy-unverified；Codex 有独立的 execution.codex-safety-unverified，表示其 CLI 配置/插件/MCP、凭据、文件访问、进程与网络出口边界尚未证明。OpenCode 仍因没有 Adapter 返回 execution.adapter-unavailable。已安装、版本兼容、Claude 已认证或 Adapter 存在均不代表 Provider 可执行。

Codex Adapter 的 fake CLI 闭环只验证固定参数、有限 JSONL 解析与共享 Task/Run/Event 生命周期。生产 Runtime 在策略门禁处拒绝 Codex，且拒绝发生在保存 Task/Run 之前。Codex 子进程不会继承 HOME 等常见 Home 环境变量；此措施不能代替 OS 沙箱，因为 CLI 可能从系统用户信息、配置目录、凭据代理或其它进程/网络机制获得访问。--sandbox read-only 只约束 Codex 自身命令运行沙箱，不证明整个 CLI 进程的文件、凭据、Hook、子进程和网络副作用都受控。

Claude claude auth status 认证状态只基于官方规定的 JSON authMethod 与退出码；VocaMesh 不读取凭据正文，也不保存原始输出。一次经授权的本机只读探测对 Claude Code 2.1.296 得到 authenticated/exit 0；这只证明 CLI 状态分类，不解除 managed hooks 的执行门禁。威胁模型、隔离路线及未执行 PoC 见 [Claude 隔离威胁模型](security/claude-isolation-threat-model.md) 和 [ADR 0001](adr/0001-agent-execution-boundary.md)。

## v0.5 本机工具发现边界

本机 CLI 发现只检查进程 `PATH` 和固定的 macOS 标准目录，不读取 Shell profile、不递归扫描 Home、不访问认证配置或凭据文件。每个候选必须与 Provider 的可执行文件名匹配；手动路径还必须是绝对路径、普通文件且可执行。常见 symlink 保留用于展示的输入路径，并单独校验 canonical 目标。版本探测仅直接启动所选二进制并传入固定 `--version`，不经 Shell；超时、输出大小和错误返回均受限制。

发现、版本、认证、适配器能力和 Runtime readiness 是独立字段。已发现或已认证都不会授权执行；只有 Runtime readiness 可控制任务入口，且 `execution.managed-policy-unverified` 仍在生产默认值与启动入口生效。Codex/OpenCode 的认证探测未建立，因此界面明确显示 unsupported/unknown。浏览器 Mock 不覆盖 Tauri 本机状态。

依据官方 CLI 文档的认证命令没有符合 VocaMesh 机器可读与凭据最小访问要求的稳定契约，本阶段不调用这些命令，不读取凭据内容，也不以模型请求验证登录状态。具体调研依据和待决路线记录在 [ADR 0001](adr/0001-agent-execution-boundary.md) 与实现报告中。

## 当前真实执行门禁

桌面端真实执行目前保持 fail-closed。`--restricted` 会加载 Claude Code 的 managed settings 与本次 `--settings`；`--safe-mode` 会关闭常规自定义项，但官方 CLI 文档明确说明 managed policy hooks 仍可运行。Hooks 可执行 Shell 命令。组织策略既可能来自设备文件/MDM，也可能通过 Claude 管理端下发；本机没有受支持的非交互查询可证明这些策略不存在或无副作用。

VocaMesh 不读取完整管理配置、认证文件、令牌、Cookie、钥匙串或凭据正文来绕过这个问题。Unix readiness 返回 `execution.managed-policy-unverified`，任务启动接口也重复执行同一门禁；前端未就绪时禁用开始按钮。自动化 fake CLI 用例通过仅在单元测试中的内部门禁注入运行，不代表生产 readiness 已通过。当前 Runtime 不会为此门禁失败的任务启动 CLI，也不会向模型提供方发送工作区内容。

官方行为依据：[Claude Code CLI reference](https://code.claude.com/docs/en/cli-reference)、[settings files and precedence](https://code.claude.com/docs/en/settings)、[hooks guide](https://code.claude.com/docs/en/hooks-guide)。

## 用户确认与数据发送

未来只有当策略、CLI 版本和认证检查全部通过后，桌面 UI 才允许用户确认目录、只读范围与数据发送并启动任务。届时 Prompt、被读取的文件和输出会由本机 Claude Code CLI 发送至用户配置的模型提供方。Prompt、结果和规范化目录路径会保存在本机应用数据；不要在 Prompt 中放入密码或 Token。当前策略门禁未通过时不会创建 Task/Run，也不会传送这些数据。

认证探测只解析允许的 `authMethod` 与退出码。CLI 版本探测、认证探测和任务执行使用同一份清理后环境，只继承 `PATH`、用户目录、临时目录、locale、终端标识与 `CLAUDE_CONFIG_DIR`；其他变量（包括任意 API key/helper 环境变量）不会传入子进程。

## 只读工作区快照

Runtime 规范化并验证用户选择的目录，拒绝文件系统根、Home 根、已知系统目录和应用数据目录；不扫描磁盘。Unix/macOS 验证时打开工作区根目录描述符，并在快照复制中一直使用该描述符。目录与文件通过 descriptor-relative `openat`、`O_NOFOLLOW`、`fstatat`/`fstat` 检查；不按已枚举的源路径二次打开。复制前后校验文件/目录元数据，复制期间按实际读取字节执行上限。

快照会排除 `.git`、`.ssh`、`.aws`、`.config`、`.claude`、`.codex`、`.opencode`、常见凭据目录/文件、密钥扩展名及构建/依赖目录。发现符号链接、特殊文件、路径替换、权限错误、目标快照已存在或其他无法安全处理的情况时，阻止本次快照；不把不完整副本交给 CLI。最多复制 100,000 个普通文件和 512 MiB。只有快照完整后才会创建 pending Run。非 Unix 路径保留保守复制代码，但 Windows 真实执行仍关闭。

成功或失败且已确认进程树退出的 Run，在终态数据库写入成功后才清理快照。退出未知或终态写入无法确认时保留快照；清理失败不会改写已经持久化的真实 Run 结果。

## CLI 参数、输出与进程

- Prompt 作为独立 `-p` 参数传入，不经过 Shell。固定参数限制工具为 `Read,Glob,Grep`，使用 `--permission-mode plan`、`--permission-prompts none`、`--disallowedTools mcp__*` 和凭据路径 deny 规则；不启用写权限、Shell、MCP 工具或子 Agent。
- `stderr` 最多保留 16 KiB 于内存并继续排空管道。分类器只返回固定的无效参数、认证、配置或一般失败类别；原始 stderr、路径、Token 和认证输出不会写入事件、SQLite、UI 或日志。
- `stdout` 只解析白名单 JSONL 字段；未知事件与原始工具输入不转发。疑似凭据命中的可见文本会遮蔽整条事件。单条事件最多 64 KiB，每 Run 最多保存 2 MiB，达到上限后仍继续排空。
- Unix 任务使用独立进程组。停止与超时先发送 `SIGTERM`、等待 2 秒后发送 `SIGKILL`，并确认进程组消失。即使主进程正常退出，也会检查同组后代；遗留后代会被终止，不能确认进程树退出时 Run 进入 `unknown`。
- `unknown` 保留当前 Run 的 PID/PGID 和快照，并阻止后续新任务。schema v2 在事务性 migration 中增加 `process_group_id`、`recovery_status` 与 `recovery_checked_at`，保留所有旧 Task/Run/Event；不删除或重建用户数据库。旧 active Run 在重启时进入 `unknown` 并写入固定恢复审计事件，v1 中没有可靠 PGID 的旧记录设为 `manual_required`。原有错误码与恢复状态分开保存。
- Unknown Run 恢复只允许后端查询和更新：PGID 缺失为 `manual-required`；Unix `kill(-PGID, 0)` 返回 `ESRCH` 仅为 `exited-candidate`；进程组存在、`EPERM`、其他 syscall 错误或不支持平台都继续阻塞。检查不会终止进程。PGID 不存在不能证明已脱离原进程组的后代已经退出。
- 归档前，用户必须在系统进程管理工具中人工核验相关 VocaMesh/Claude Code 进程并明确确认；Runtime 在 `start_gate` 内重新检查 PGID，并在同一个 SQLite 事务中把 Run 标为失败、保留原始 `error_code`、更新恢复状态并写入固定审计事件。事务提交后才按可信应用数据目录和 UUID 请求 ID 清理快照；Unix 使用逐级 `O_NOFOLLOW` 打开的目录描述符及 `unlinkat`，不沿父目录符号链接删除。清理失败不会回滚到 Unknown 或改写为成功，只追加固定清理失败诊断；不重跑任务。
- Recovery IPC 不接受前端 PID、PGID、快照路径或“进程已退出”布尔值。前端只看到短 Run ID、任务标题、稳定错误码、检查结果、快照存在状态和最近检查时间；浏览器 Mock 返回空恢复列表且不调用 Tauri。
- SQLite 迁移/读取失败不会删除数据库。终态写入失败会尽力将 Run 标成 `unknown`；若数据库仍不可写，active 记录继续阻止新任务，快照保持不清理。
- Windows readiness 固定为不可执行；Linux 未通过本机运行验收。GUI/平台状态见 `implementation-report.md`。

敏感文件名排除、输出脱敏和只读工具配置是纵深防御，不保证识别所有秘密，也不构成通用操作系统沙箱。用户仍需核对所选目录和数据发送范围。

## Tauri 权限

## v0.4 本地 Catalog 边界

Agent 角色、Provider、指令、权限草稿，Workspace 路径与范围草稿，以及 Team 主管/成员关系只保存在本机 SQLite。schema v3 通过事务迁移保留现有 Task/Run/Event 与 Unknown Run 恢复信息；未来版本拒绝打开，迁移失败不会删库或留下半套表。

Catalog 不读取凭据正文，也不会启动 CLI。Agent 的 write/shell/network 与 Workspace 范围从不作为当前 Runtime 授权来源；真实执行继续由独立 readiness 和 managed policy preflight 控制。执行状态仍为 execution.managed-policy-unverified 时，角色或团队 CRUD 不会开放任务启动。

Rust 会规范化并检查目录存在、目录类型、符号链接、系统/过宽路径与相对范围；Team 的外键、主管、重复成员和无环不变量在事务内验证。归档仍被活跃 Team 引用的 Agent 或 Workspace 会失败，路径已失效的 Workspace 不能新建 Team 绑定或作为改绑目标。路径后来失效时，目录记录保留并显示需要检查。并发更新通过 revision 和 expectedRevision 防止旧状态覆盖。

Catalog 审计只保存实体 ID、动作、版本与时间，不复制 Agent 指令、工作区完整路径或密钥。数据库/IPC 错误以固定错误码返回；原生端不回退 Mock。原生审批和通知列表保持空，不提供虚构授权入口；浏览器 Mock 中的审批只用于演示。

主窗口 capability 只授予事件监听与取消监听，以及系统目录选择器 `dialog:allow-open`。Rust 只暴露固定的工具、目录选择和执行/历史命令；没有 Shell 插件、任意命令执行或任意文件 IPC。
