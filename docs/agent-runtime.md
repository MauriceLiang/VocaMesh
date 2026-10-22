# 本机 Agent CLI 运行时

## 支持范围与版本门槛

只有 Claude Code 接入真实任务。Codex/OpenCode 仍可发现，但不能执行。CLI 必须是绝对路径上的普通可执行文件，运行时会在启动任务前重新核验路径、版本和认证。

当前安全参数最低版本设为 `2.1.259`：`--restricted` 自 `2.1.248` 起可用，`--permission-prompts none` 自 `2.1.259` 起可用。低版本、认证状态不明或参数校验失败时拒绝启动，不回退到默认权限。2026-10-22 本机记录的 Claude Code 版本为 `2.1.296`；自动化测试不会运行真实模型任务。

## 认证探测

仅运行固定的 `claude auth status` 参数。实现最多在内存中保留 16 KiB stdout，并只解析退出码与 `authMethod`：退出码 0 且方法是 `claude.ai` 或 `oauth_token` 才显示已认证；退出码 1 且方法为 `none` 才显示未认证；API key、helper、third-party、超时、异常退出、无效 JSON 和其他组合都为 `unknown` 并阻止执行。认证 JSON、邮箱、配置目录、令牌、stderr 和完整输出不写入数据库、UI 或日志。

限制 API key/helper/third-party 方法是因为本应用清理了任务子进程环境，不会复制或读取 API Key、云凭据或任意用户环境变量；这些认证方式不能被安全确认可供任务进程使用。

## 固定只读启动参数

Prompt 作为独立 `-p` 参数，不经 Shell。Runtime 使用固定参数集合：

- `--restricted`：只加载 managed settings 和本次 `--settings`，不读取用户/项目配置，并把内置文件工具限制在工作目录；
- `--safe-mode`：禁用用户/项目自定义 hooks、MCP、插件、agents、skills、命令、`CLAUDE.md` 等；
- `--permission-mode plan` 与 `--permission-prompts none`；
- `--tools Read,Glob,Grep`，以及 `--disallowedTools mcp__*`；
- `--settings` 写入拒绝已知凭据路径的内联策略；
- `--no-session-persistence`、`--output-format stream-json`、`--include-partial-messages`、`-p <prompt>`。

不使用 `--bare`：官方 CLI 文档说明它仍可访问 Bash、文件读取和编辑工具，且跳过认证相关行为。也不使用 `--allowedTools` 作为白名单，因为它只预批准工具而不限制工具可用集合。受限模式负责把内置文件工具限制到工作目录；Runtime 仍将源码复制为过滤快照并在快照目录启动 CLI。

CLI 来源与参数语义：[CLI reference](https://code.claude.com/docs/en/cli-reference)、[Configure permissions](https://code.claude.com/docs/en/permissions)、[Security](https://code.claude.com/docs/en/security)。这些是官方文档依据，实际模型/账户的手工执行尚未验收。`--safe-mode` 不会禁用 managed policy 中部署的 hooks；组织策略可能仍应用，这是本实现的已知限制。

## 输出与进程

stdout 按最多 64 KiB 的 JSONL 记录解析，只发布 allowlist 的文本增量、最终摘要和固定工具进度消息。未知事件丢弃，协议损坏只生成固定错误文本；stderr 并发读取并丢弃，避免管道阻塞。疑似凭据命中的整条文本会被遮蔽。UTF-8 字节上限为 64 KiB，Run 事件总量最多 2 MiB，超限只持久化一次截断标记后继续消费输出。

任务环境清空后只继承 PATH、用户目录、临时目录、locale、终端标识和 `CLAUDE_CONFIG_DIR`；不打印变量值。stdin 关闭，stdout/stderr 并发读取，硬超时 10 分钟。Unix 进程以独立进程组启动；停止先发 SIGTERM，等待 2 秒后发 SIGKILL，再确认组内进程退出。Windows 执行入口关闭，直到 Job Object 或等效方案完成验证。

正常完成需 exit code 0 和有效 final 事件。非零退出、缺少 final、非法协议、超时、取消和无法确认进程退出映射为失败/停止/未知，不自动重试。历史事件按 `(runId, sequence)` 递增写入 SQLite；进程启动后才进入 `running`。
