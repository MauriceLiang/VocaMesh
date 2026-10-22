# IPC 契约

Rust 使用 snake_case 命令名；返回 DTO 使用 camelCase JSON。UI/Store 只通过 `useServices()` 访问这些命令。

| Command | 参数 | 返回 | 语义 |
| --- | --- | --- | --- |
| `scan_tools` | 无 | `LocalTool[]` | 每次返回 `claude`、`codex`、`opencode` 三项真实扫描结果；单项探测失败保留为 `undetectable`。 |
| `verify_tool` | `{ id }` | `LocalTool` | 重新执行固定版本探测，不运行 Agent；不支持安全认证探测时仍不可用。 |
| `set_tool_path` | `{ id, path }` | `LocalTool` | 仅接受绝对、存在、普通且可执行文件；保存后状态为 `undetectable`，等待复核。 |
| `get_default_tool_id` | 无 | `string \| null` | 只返回当前进程扫描快照中 `ready` 的工具；重启后需重新扫描确认。 |
| `set_default_tool` | `{ id }` | `void` | 只允许当前扫描快照中状态为 `ready` 的工具。 |

## `LocalTool`

字段：`id`、`name`、`adapter`、`version`、`path`、`state`、`note`、`authState`、`detectedAt`（Unix epoch 毫秒）和 `source`（`path`、`standard-location`、`manual` 或 `null`）。认证和安装状态分开；`unknown`、`unsupported` 均不能产生 `ready`。

`state` 沿用前端状态：`ready`、`pending`、`auth-required`、`missing`、`incompatible`、`undetectable`。扫描终态不会使用 `pending`。当前三个 Provider 都没有已核准的安全认证探测，因此即使版本读取成功，状态仍为 `undetectable`。

## 错误和事件

命令错误为 `{ code, message }`，前端映射为 `ServiceError`。代码包括 `tool.not-found`、`tool.invalid-path`、`tool.not-ready`、`tool.probe-timeout`、`tool.version-unavailable`、`tool.scan-failed`、`tool.scan-superseded`、`settings.read-failed`、`settings.write-failed`。单个 CLI 探测错误作为该工具的状态和固定说明返回，不把原始 stdout/stderr 放进错误。

事件 `tool-scan` 的 payload 为 `{ phase: "scanning" | "done" | "failed", message?: string }`。事件不带完整命令输出、环境变量或认证材料。后端只转发最新扫描请求的终态事件；前端按请求代数丢弃过期快照，扫描结果以 `scan_tools` 的返回值为准。

此事件通过 Tauri core event 的 `allow-listen` / `allow-unlisten` capability 提供。自定义 Commands 仅由 `generate_handler!` 注册这五个固定能力；没有通用 shell、任意进程或任意文件 IPC。
