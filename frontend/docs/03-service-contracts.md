# Service 接口与 Rust 对接说明

所有接口位于 `src/services/contracts/`，Mock 位于 `src/services/mock/`，注入点是 `src/services/adapters/index.ts`。

## 接口清单

| 文件 | 接口 | 方法 | 现状 |
|---|---|---|---|
| `agent-service.ts` | `AgentService` | listAgents / createAgent / updateAgent / deleteAgent / setPermissions | Mock 可用 |
| `tool-service.ts` | `ToolService` | scanTools / verifyTool / setToolPath / setDefaultTool / getDefaultToolId | Mock 可用 |
| `team-service.ts` | `TeamService` | listTeams / getTeam / createTeam / updateTeam / addMember / removeMember / setReportsTo / listMembers | Mock 可用（含环检测） |
| `task-service.ts` | `TaskService` | listTasks / getTask / createTask / pauseTask / resumeTask / stopTask / retryFailed / skipAndFinalizePartial / listSubtasks / subscribeTaskState | Mock 可用（定时器模拟） |
| `approval-service.ts` | `ApprovalService` | listApprovals / getApproval / decide / canGrantTaskScope / subscribe | Mock 可用 |
| `run-service.ts` | `RunService` | listRuns / getPartialArtifacts / listArtifacts / exportRunsCsv | Mock 可用 |
| `workspace-service.ts` | `WorkspaceService` | listWorkspaces / createWorkspace / updateBoundaries / resolveConflict / resolveOpenPath | Mock 可用 |
| `settings-service.ts` | `SettingsService` | getSettings / updateSettings / resetAppearance / resetDemoData | Mock 可用 |
| `notification-service.ts` | `NotificationService` | listNotifications / markRead / markAllRead / dismiss / unreadCount | Mock 可用 |
| `event-service.ts` | `EventService` | subscribeTaskLogs / subscribeTaskState / subscribeApprovals / subscribeToolScan | Mock 可用 |

每个方法的 JSDoc 都包含：用途、调用时机、参数、返回、错误码、Mock 行为、Rust 职责、必要的权限/状态限制。

## 事件模型（不要用轮询替代）

```ts
const off = services.events.subscribeTaskLogs(taskId, (line: LogLine) => { … });
onUnmounted(off);                       // 必须取消订阅
services.events.subscribeTaskState((task) => { … });
services.events.subscribeApprovals((approval) => { … });
```

- 日志是流式：初帧回放 + 增量推送；界面只在“日志”Tab 订阅。
- 任务状态、审批状态同样是事件驱动；页面不做 `setInterval` 轮询。
- Rust 侧建议：`emit("task://log", payload)` 等事件通道（**具体命令/事件名以 Rust 实现为准，本工程不臆造**）。

## 错误约定

```ts
throw new ServiceError("该名称已存在", "agent.duplicate", name);
```

页面/Store 根据 `code` 决定提示与后续动作；`detail` 用于携带环路径等补充信息。

## 接入 Rust 的步骤（给后端同学）

1. 在 Rust Core 实现同名能力（角色 CRUD、团队关系、任务编排、审批、Run 历史、事件推送…）。
2. 新增 `src/services/tauri/*.ts`，用 `invoke`/`listen` 实现同样的接口签名。
3. 在 `services/adapters/index.ts` 中新增 `createTauriServices()`，并把 `useServices()` 的默认 mode 切到 `"tauri"`。
4. 保持 `types/index.ts` 的领域类型不变（或在此做映射），页面无需改动。
5. 事件通道必须保证：暂停有“请求/确认”两段、停止有“请求/结果”、日志可取消订阅。

## 尚未确定的真实能力（本工程用明确的降级表现）

- 增量定向指令：不支持时显示“等待安全投递点”。
- 任务内授权 + 撤销：`canGrantTaskScope()` 返回 false，界面渲染禁用的“规划中”。
- 关闭窗口后继续执行：仅在设置中说明限制，不承诺行为。
