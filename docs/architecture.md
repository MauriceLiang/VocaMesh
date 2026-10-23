# 架构

## v0.7 ProviderRunner 与统一执行协调器

Vue/Pinia 通过 Tauri 请求 Provider-aware readiness 和任务服务；ExecutionRuntime 是唯一生命周期与持久化协调器，包含 provider_policy、快照、StartGate、停止/恢复及 SQLite schema v3 Task/Run/Event。vocamesh-agent 的静态 ProviderRunner 分发到 ClaudeCodeRunner（复用 ClaudeCli）或 CodexCliRunner（codex exec --json）；OpenCode 暂无 Adapter。

ProviderRunner 只负责固定启动方式、Provider 探测、输出协议解析和错误分类；Runtime 继续统一执行入口、单活跃约束、快照、进程监督、状态迁移、事件持久化、取消/超时和 Unknown Run 恢复。Provider ID 经过静态分发，不复制 Runtime 或数据库生命周期。SQLite schema v3 未改变。

任务创建对话框可选择 Claude Code 或 Codex，并请求该 Provider 的 readiness；Tauri 命令要求 providerId，按同一 Provider 返回状态。Codex 的 Adapter 和 fake 生命周期测试已实现，但生产策略 execution.codex-safety-unverified 在新建 Task/Run 前阻断；Claude 继续由 execution.managed-policy-unverified 阻断。前端展示状态不构成授权，Runtime 启动入口始终重新检查。详见 [本机 Agent CLI 运行时](agent-runtime.md) 与 [安全边界](security.md)。

## v0.4 本地 Catalog 与桌面服务

```text
Vue / Pinia / Service contracts
  ├─ Browser preview → MockServices（保留完整演示数据）
  └─ Tauri → Native services（显式组装，不继承 Mock）
                  ├─ Agent / Workspace / Team Catalog
                  ├─ Appearance settings
                  ├─ Empty Approval / Notification services
                  └─ Existing Task / Run / Event services
                         │ invoke / listen
                    Tauri Commands
                         │
                    ExecutionRuntime
                    ├─ one SQLite Store (schema v3)
                    │  ├─ v2 Task / Run / Event and recovery history
                    │  └─ Agent / Workspace / Team / audit / settings
                    └─ execution readiness (still FAIL-CLOSED)
```

Tauri 的 `createServices()` 显式构造所有原生 Service。Catalog IPC 失败会返回稳定错误，Pinia 页面显示错误和重试；原生模式没有 Mock seed 回退。浏览器预览仍走原有 Mock 服务，桌面模式不会改变浏览器演示数据。

## Catalog 数据与关系

`vocamesh-runtime` 复用 `ExecutionRuntime` 的 SQLite `Store`，不另建数据库。schema v3 在保留 v2 `tasks`、`runs`、`run_events`、PID/进程组和 Unknown Run 恢复字段的基础上，增加：

- `agent_roles` 与 `agent_permission_configs`：保存用户配置的 Agent 角色、Provider、指令和权限草稿。
- `workspaces` 与 `workspace_scopes`：登记规范化本地目录和未来范围草稿。
- `teams` 与 `team_members`：使用稳定 Agent/Workspace ID 保存主管、直属上级和团队关系。
- `catalog_audit`：仅记录对象 ID、动作、版本与时间，不复制指令、目录正文或凭据。
- `catalog_settings`：只存主题、密度、灰度预览和减少动态效果等外观偏好。

迁移在事务中将 v0/v1/v2 数据库升级到 v3；未来版本拒绝打开。名称唯一、Provider、目录规范化与符号链接、范围路径、外键引用、主管唯一和团队无环均由 Rust 校验。归档 Agent 或 Workspace 时，仍被活跃 Team 引用会被拒绝。成功写入递增 revision，并支持 expectedRevision 冲突保护。

Team 成员的名称、职责和 Provider 从 Agent 记录查询，不重复保存；Team 使用 workspace ID，Workspace 或 Agent 改名不会破坏关联。Workspace 任务数只按本机 Task 中精确匹配的路径统计；不确定的旧路径不猜测。失效目录保留记录并显示“目录需检查”。

## 配置与执行权限分离

Catalog 的 Agent 权限和 Workspace 读/写/禁止范围只是配置草稿，不授权 Runtime 访问文件、运行 Shell 或联网。Provider 工具已安装或已探测也不代表角色可执行。原生 Agent 显示执行暂不可用；CRUD 和外观设置不检查执行 readiness，也不会改变执行门禁。

生产真实执行仍因 `ManagedPolicyStatus::Unverified` 返回 `execution.managed-policy-unverified` 而 fail-closed。Catalog 成功、桌面构建成功或 fake CLI 测试都不能证明真实 Agent 执行已通过验收。原生 Approval 和 Notification 使用真实空状态；授权入口只留在浏览器 Mock 预览。

## v0.3 单 Agent 执行链路

```text
Vue / Pinia
  ├─ Browser → 完整 Mock Services
  └─ Tauri → Execution / Task / Run / Log Services
                    ↓ invoke / listen
              Tauri Commands（薄桥接）
                    ↓
              vocamesh-runtime
              ├─ 启动前校验、状态迁移、取消和恢复
              ├─ SQLite Task / Run / Event 存储
              ├─ 过滤后的临时工作区快照
              └─ 本机进程监督
                 ↙          ↘
       vocamesh-agent      SQLite
       Claude Code CLI     app data
```

`vocamesh-core` 定义跨层 DTO、状态和错误，不依赖 Tauri。`vocamesh-agent` 负责 CLI 路径解析、版本/认证探测、固定启动参数与输出解析。`vocamesh-runtime` 负责数据库、路径授权校验、快照、进程生命周期和事件持久化。Tauri Commands 只做参数传递、阻塞任务调度和事件转发。

当前只有 Claude Code 受限只读执行实现；启动前检查策略、CLI、认证、路径和用户确认。应用退出时请求停止并等待；重启会把残留活跃记录置为 `unknown`，不会自动重启。生产安全策略未核验前真实任务保持禁用。Windows 真实执行关闭；平台、GUI 与真实执行验收状态见 [implementation-report.md](implementation-report.md)。

## 目录

- `crates/vocamesh-core/`：跨层 DTO、Provider、状态、错误和事件契约。
- `crates/vocamesh-agent/`：CLI 发现、认证探测、固定命令构造和 JSONL 安全解析。
- `crates/vocamesh-runtime/`：Catalog 与 Task/Run/Event SQLite 存储、路径验证、状态机、进程监督。
- `frontend/src-tauri/`：Tauri 入口、目录选择、薄 IPC 命令和安全 capability。
- `frontend/src/services/contracts/`：前端服务组合契约。
- `frontend/src/services/tauri/`：原生 `invoke()` / `listen()` 服务适配器。
- `frontend/src/services/mock/`：浏览器预览及未接入领域的 Mock 实现。
