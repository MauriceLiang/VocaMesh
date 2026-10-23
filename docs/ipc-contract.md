# IPC 契约

Rust command 使用 snake_case；DTO JSON 使用 camelCase。界面和 Store 只能通过 `useServices()` 访问 IPC；组件不得直接调用原生命令。

## Commands

| Command | 参数 | 返回 | 语义 |
| --- | --- | --- | --- |
| `get_catalog_status` | 无 | `CatalogStatus` | 返回 schema 版本和非归档 Agent/Workspace/Team 数量；独立于 execution readiness。 |
| `list_agents` / `get_agent` | `{ id? }` | `AgentRole[]` / `AgentRole` | 查询本机角色配置，不扫描或启动 CLI。 |
| `create_agent` | `{ input: CreateAgentInput }` | `AgentRole` | 校验 Provider/name 并落库，权限是配置草稿。 |
| `update_agent` | `{ id, input: UpdateAgentInput }` | `AgentRole` | 更新配置、审计并递增 revision；可传 `expectedRevision`。 |
| `set_agent_permissions` | `{ id, permissions, expectedRevision? }` | `AgentRole` | 独立保存权限草稿并检查 revision；不会改变 Runtime 能力。 |
| `archive_agent` | `{ id }` | `void` | 活跃 Team 仍引用时返回 `agent.in-use`。 |
| `list_workspaces` / `get_workspace` | `{ id? }` | `WorkspaceRecord[]` / `WorkspaceRecord` | 查询登记目录、路径可用状态、范围草稿和真实 Task 路径计数。 |
| `create_workspace` | `{ input: CreateWorkspaceInput }` | `WorkspaceRecord` | 校验真实目录、规范路径和敏感范围后登记。 |
| `update_workspace_scopes` | `{ id, input: UpdateWorkspaceScopesInput }` | `WorkspaceRecord` | 保存相对路径范围草稿并递增 revision；使用加载时的 `expectedRevision` 拒绝旧草稿覆盖新值。 |
| `rename_workspace` | `{ id, name, expectedRevision? }` | `WorkspaceRecord` | 保持 Workspace ID 与 Team 关系，检查同名和 revision。 |
| `archive_workspace` | `{ id }` | `void` | 活跃 Team 仍引用时返回 `workspace.in-use`；不删除 Task/Run 历史。 |
| `list_teams` / `get_team` | `{ id? }` | `TeamRecord[]` / `TeamRecord` | 关系来自持久化成员 ID；显示数据从 Agent 记录派生。 |
| `list_team_members` | `{ id }` | `TeamMemberRecord[]` | 读取指定团队成员。 |
| `create_team` | `{ input: CreateTeamInput }` | `TeamRecord` | 创建单主管 Team，主管同时成为唯一根成员。 |
| `update_team` | `{ id, input: UpdateTeamInput }` | `TeamRecord` | 更新名称/主管/Workspace；校验关系并递增 revision。 |
| `add_team_member` | `{ id, input: AddTeamMemberInput }` | `TeamRecord` | 事务内校验直属主管、自指、重复成员与无环关系。 |
| `remove_team_member` | `{ id, agentId }` | `TeamRecord` | 主管或仍有下属的成员不可移除。 |
| `set_team_reports_to` | `{ id, agentId, reportsToAgentId }` | `TeamRecord` | 更新直属关系；同团队校验，无环失败时回滚。 |
| `get_appearance_settings` / `update_appearance_settings` | 无 / `{ settings }` | `AppearanceSettings` | SQLite 持久化主题、密度、灰度预览和减少动态效果。 |
| `select_workspace_directory` | 无 | `string \| null` | 原生目录选择；用户取消返回 `null`。 |
| `get_execution_readiness` | 无 | `ExecutionReadiness` | 查询 Claude CLI 版本与受限认证状态；不启动任务。 |
| `list_unknown_runs` | 无 | `UnknownRunSummary[]` | 列出 Unknown Run、任务标题与快照存在状态；不返回 PGID 或快照路径。 |
| `inspect_unknown_run` | `{ runId }` | `RecoveryInspection` | 后端更新最近检查时间并检查 Unix 进程组；不改变 Run 终态、不终止进程。 |
| `resolve_unknown_run` | `{ runId, userConfirmed }` | `RecoveryResolution` | 仅接受用户明确确认；Runtime 在 `start_gate` 内重新核验 PGID、事务归档为失败，再尝试清理快照并返回快照清理状态。 |
| `start_read_only_task` | `{ request: ExecutionRequest }` | `{ task, run }` | 校验授权、Provider、目录、CLI 和认证；幂等保存 Task/Run 后异步启动。 |
| `stop_run` | `{ runId }` | `RunRecord` | 发出停止请求；未确认进程树退出时返回稳定错误。 |
| `get_run` | `{ runId }` | `RunRecord` | 读取一个本机 Run 快照。 |
| `list_run_events` | `{ runId, after?, limit? }` | `RunEvent[]` | `after` 为排他序号；每页最多 100 条。 |
| `list_tasks` | 无 | `TaskRecord[]` | 返回最多 500 条本机任务，不回退到 Mock。 |
| `get_task` | `{ taskId }` | `TaskRecord` | 读取一个本机任务。 |
| `list_runs` | `{ cursor?, limit? }` | `RunPage` | SQLite 游标分页；页大小限制为 1–100。 |

ExecutionRequest 包含 requestId、title、prompt、providerId、workspacePath 和用户确认的 consent。当前 Claude 与 Codex 进入各自 Adapter/策略校验；Claude 仍因 managed policy 阻断，Codex 在生产策略门禁处以 execution.codex-safety-unverified 阻断，并且均不会因此绕过 Runtime 校验。OpenCode 因缺少 Adapter 返回 execution.adapter-unavailable。成功进入执行阶段的请求才会把规范化路径、read-only-filtered-snapshot 授权范围和确认时间写入 Task。请求 ID 必须是 UUID；重复 ID 返回原记录，冲突载荷返回 execution.request-duplicate。

## v0.7 Provider 相关命令与事件

前端创建任务时以 ExecutionService.readiness(providerId) 请求当前选择项的状态；Tauri adapter 将 { providerId: claude | codex } 传给 get_execution_readiness。Tauri 根据该 ID 调用 ExecutionRuntime::readiness_for，不把 Claude 的策略结果复用到 Codex，也不接受前端提供的“已验证”开关。未知 Provider 请求返回稳定错误。

start_read_only_task 接收原有 ExecutionRequest，后端根据 providerId 选择静态 Runner 并再次核验对应策略、CLI、工作区和用户确认。策略阻断发生在保存 Task/Run 前。Codex 已有 Runner，但生产 readiness 返回 execution.codex-safety-unverified；Claude 仍返回 execution.managed-policy-unverified。两者共享原来的状态和持久化契约，无 schema 迁移或 Provider 专用执行命令。

Codex JSONL 的允许输出映射到既有 Run Event 类型：摘要文本为 final、固定阶段信息为 progress、Provider 拒绝为已过滤的 error、未知/畸形协议为固定 protocol-error。协议终态本身只作为 Runtime 成功判定状态，不持久化原始 JSON。turn.failed、缺失完成事件或非零退出码不会成为成功 Run；未知协议内容不进入 IPC。

Catalog DTO 使用 `camelCase`，所有时间为 Unix epoch 毫秒。Agent、Workspace、Team 使用稳定 ID；Team 成员的 `name`、`role`、Provider 与 `reportsToAgentId` 从已保存记录查询。Workspace 的 `pathAvailable=false` 表示登记路径当前丢失、不可访问或不再通过安全路径检查；记录仍保留，Team 新建或改绑必须选有效目录。Agent `permissions` 与 Workspace `scopes` 仅为草稿，不代表 Runtime 授权。Tauri 服务不会因为 CLI 探测失败而删除或伪造 Catalog 项。

`TaskRecord` 包含标题、Prompt、Provider、规范化工作目录、任务状态、时间、当前 Run、授权范围和确认时间。`RunRecord` 包含独立 ID、请求 ID、Task ID、Provider、状态、结果、PID（运行时）、起止时间、退出码、摘要、事件序号、截断标记、稳定错误码、`recoveryStatus` 和 `recoveryCheckedAt`。PGID 仅留在后端数据库，不通过 IPC 暴露。`ExecutionReadiness.reasonCode` 区分策略、Unknown Run、CLI、认证、平台和存储阻断。事件字段为 `runId`、递增 `sequence`、时间戳、白名单 `kind` 和筛选后的文本。

`RecoveryStatus` 为 `none`、`required`、`resolved`、`manual-required`。进程组检查状态为 `exited-candidate`、`still-running`、`cannot-verify`、`manual-required`；快照状态为 `exists`、`missing`、`cannot-verify`。进程组不存在只产生人工归档候选，不足以证明已脱离原进程组的后代已退出。前端不能传 PID、PGID、快照路径、检查结果或“进程已退出”布尔值；`userConfirmed` 只表示用户在系统进程管理工具完成人工核验，Runtime 会在写入前重新检查。

`RecoveryResolution` 包含归档后的 `run` 和 `snapshotStatus`，使界面能区分快照已清理与清理未能确认。

SQLite schema v2 通过事务性 v1→v2 migration 增加 PGID、恢复状态和最近检查时间。v0.4 的事务性 v2→v3 migration 新增 Catalog 表；旧 Task/Run/Event、恢复字段与审计事件保持原样，未来版本拒绝打开。恢复时数据库先提交 `recovery-confirmed-exited` 审计事件并将 Run 标为失败，之后才清理快照。清理失败保留已归档失败状态并追加固定的 `recovery-snapshot-cleanup-failed` 事件。操作幂等；任何仍未知的 Run 继续阻止新真实任务。

状态分别使用 `pending`、`starting`、`running`、`stop-requested`、`finished`、`crashed`、`unknown`。Task 面向 UI 映射为 `queued`、`starting`、`running`、`stop-pending`、`stopped`、`done`、`failed`、`unknown`。`finished` 的最终结果需结合 `outcome` 区分成功、停止和超时；Run 状态不会覆盖旧终态。

## 事件

- `run://state` 携带完整 Run 快照；`id` 是 Run ID，终态也通过此事件发布。
- `run://event` 携带一条已筛选并持久化的 `RunEvent`。前端先订阅再补读最近最多 500 条历史事件，按序号排序/去重；历史通过 `list_run_events` 继续分页。
- 恢复操作使用 `recovery-required`、`recovery-confirmed-exited` 和 `recovery-snapshot-cleanup-failed` 固定事件类型；恢复不会发布成功结果或创建新 Run。
- `tool-scan` 保留既有工具扫描进度事件。

离开页面/检查器时必须取消订阅。遗漏事件可通过 `get_run` 和 `list_run_events` 恢复。未实现的暂停、恢复、重试、部分成果操作返回 `task.operation-unsupported`，不得调用 Mock 定时器。

## 错误

Command 错误序列化为 `{ code, message }`，前端映射为 `ServiceError`。稳定错误包含：

- `agent.not-found`、`agent.duplicate`、`agent.invalid-input`、`agent.in-use`、`agent.revision-conflict`；
- `workspace.not-found`、`workspace.duplicate`、`workspace.path-missing`、`workspace.invalid-path`、`workspace.invalid-input`、`workspace.invalid-scope`、`workspace.in-use`、`workspace.revision-conflict`；
- `team.not-found`、`team.duplicate`、`team.invalid-input`、`team.member-exists`、`team.member-not-found`、`team.self-management`、`team.cycle`、`team.supervisor-required`、`team.workspace-unavailable`、`team.revision-conflict`；
- `execution.provider-not-supported`、`execution.cli-not-ready`、`execution.cli-not-found`、`execution.cli-incompatible`、`execution.auth-required`、`execution.auth-unknown`、`execution.workspace-invalid`、`execution.consent-required`、`execution.managed-policy-unverified`、`execution.already-running`、`execution.process-state-unknown`、`execution.request-duplicate`、`execution.invalid-request`；
- Provider 执行补充错误：execution.codex-safety-unverified、execution.adapter-unavailable、execution.codex-version-incompatible、execution.codex-probe-failed、execution.codex-probe-timeout、execution.auth-probe-failed、execution.auth-probe-timeout、execution.read-only-policy-unavailable、execution.provider-failed、execution.output-protocol-error、execution.stdin-unavailable、execution.stdin-write-failed。
- `run.not-found`、`run.invalid-id`、`run.recovery-not-required`、`run.recovery-confirmation-required`、`run.recovery-process-present`、`run.recovery-path-invalid`、`run.stop-failed`、`run.stop-unconfirmed`；Run 持久化错误码还包括 `run.timeout`、`run.output-protocol-error`、`execution.spawn-failed`；
- `storage.read-failed`、`storage.write-failed`、`storage.schema-unsupported`、`execution.runtime-failed`。

原始 stderr、认证 JSON、系统堆栈和未识别 CLI JSON 不进入前端。Tauri capability 只允许主窗口调用目录选择和监听事件；没有通用 shell 或任意文件命令。
