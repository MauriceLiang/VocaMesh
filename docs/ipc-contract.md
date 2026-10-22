# IPC 契约

Rust command 使用 snake_case；DTO JSON 使用 camelCase。界面和 Store 只能通过 `useServices()` 访问 IPC；组件不得直接调用原生命令。

## Commands

| Command | 参数 | 返回 | 语义 |
| --- | --- | --- | --- |
| `select_workspace_directory` | 无 | `string \| null` | 原生目录选择；用户取消返回 `null`。 |
| `get_execution_readiness` | 无 | `ExecutionReadiness` | 查询 Claude CLI 版本与受限认证状态；不启动任务。 |
| `start_read_only_task` | `{ request: ExecutionRequest }` | `{ task, run }` | 校验授权、Provider、目录、CLI 和认证；幂等保存 Task/Run 后异步启动。 |
| `stop_run` | `{ runId }` | `RunRecord` | 发出停止请求；未确认进程树退出时返回稳定错误。 |
| `get_run` | `{ runId }` | `RunRecord` | 读取一个本机 Run 快照。 |
| `list_run_events` | `{ runId, after?, limit? }` | `RunEvent[]` | `after` 为排他序号；每页最多 100 条。 |
| `list_tasks` | 无 | `TaskRecord[]` | 返回最多 500 条本机任务，不回退到 Mock。 |
| `get_task` | `{ taskId }` | `TaskRecord` | 读取一个本机任务。 |
| `list_runs` | `{ cursor?, limit? }` | `RunPage` | SQLite 游标分页；页大小限制为 1–100。 |

`ExecutionRequest` 包含 `requestId`、`title`、`prompt`、`providerId`、`workspacePath` 和用户确认的 `consent`。本阶段只接受 Provider `claude`。Runtime 将规范化路径、`read-only-filtered-snapshot` 授权范围和确认时间写入 Task。请求 ID 必须是 UUID；重复 ID 返回原记录，冲突载荷返回 `execution.request-duplicate`。

`TaskRecord` 包含标题、Prompt、Provider、规范化工作目录、任务状态、时间、当前 Run、授权范围和确认时间。`RunRecord` 包含独立 ID、请求 ID、Task ID、Provider、状态、结果、PID（运行时）、起止时间、退出码、摘要、事件序号、截断标记和稳定错误码。事件字段为 `runId`、递增 `sequence`、时间戳、白名单 `kind` 和筛选后的文本。

状态分别使用 `pending`、`starting`、`running`、`stop-requested`、`finished`、`crashed`、`unknown`。Task 面向 UI 映射为 `queued`、`starting`、`running`、`stop-pending`、`stopped`、`done`、`failed`、`unknown`。`finished` 的最终结果需结合 `outcome` 区分成功、停止和超时；Run 状态不会覆盖旧终态。

## 事件

- `run://state` 携带完整 Run 快照；`id` 是 Run ID，终态也通过此事件发布。
- `run://event` 携带一条已筛选并持久化的 `RunEvent`。前端先订阅再补读最近最多 500 条历史事件，按序号排序/去重；历史通过 `list_run_events` 继续分页。
- `tool-scan` 保留既有工具扫描进度事件。

离开页面/检查器时必须取消订阅。遗漏事件可通过 `get_run` 和 `list_run_events` 恢复。未实现的暂停、恢复、重试、部分成果操作返回 `task.operation-unsupported`，不得调用 Mock 定时器。

## 错误

Command 错误序列化为 `{ code, message }`，前端映射为 `ServiceError`。稳定错误包含：

- `execution.provider-not-supported`、`execution.cli-not-ready`、`execution.auth-required`、`execution.workspace-invalid`、`execution.consent-required`、`execution.read-only-policy-unavailable`、`execution.already-running`、`execution.process-state-unknown`、`execution.request-duplicate`、`execution.invalid-request`；
- `run.not-found`、`run.stop-failed`、`run.stop-unconfirmed`；Run 持久化错误码还包括 `run.timeout`、`run.output-protocol-error`、`execution.spawn-failed`；
- `storage.read-failed`、`storage.write-failed`、`storage.schema-unsupported`、`execution.runtime-failed`。

原始 stderr、认证 JSON、系统堆栈和未识别 CLI JSON 不进入前端。Tauri capability 只允许主窗口调用目录选择和监听事件；没有通用 shell 或任意文件命令。
