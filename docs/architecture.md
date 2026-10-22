# 架构

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

浏览器继续使用完整 Mock。桌面模式的 Task、Run 和任务日志只来自本机 Runtime；首次启动为空，不把 Mock 种子或定时器混入真实列表。Agent、Team、Approval、Notification、Settings 等未接入模块仍是演示数据，并在桌面源说明中标注。

当前仅为 Claude Code 提供真实执行入口。任务先复制到应用数据目录中的过滤快照，再由受限 CLI 在该快照中读取；正常结束后删除快照。Task、Run 和筛选事件保存在应用数据目录的 SQLite 文件。Run 仅允许单并发、10 分钟硬超时，不自动重试。应用退出时请求停止并等待最多 5 秒；重启将残留活跃记录置为 `unknown`，不自动重启。

真实执行仅在 Unix 目标启用，使用独立进程组终止。Windows 的 readiness 明确为不可执行，直到可验证的进程树终止策略实现。自动化测试未调用真实 Claude 模型；GUI、真实登录/任务、Windows 和 Linux 平台行为需参见 `implementation-report.md`。

## 目录

- `crates/vocamesh-core/`：跨层 DTO、Provider、状态、错误和事件契约。
- `crates/vocamesh-agent/`：CLI 发现、认证探测、固定命令构造和 JSONL 安全解析。
- `crates/vocamesh-runtime/`：路径验证、敏感文件过滤、状态机、进程监督和 SQLite 存储。
- `frontend/src-tauri/`：Tauri 入口、目录选择、薄 IPC 命令和安全 capability。
- `frontend/src/services/contracts/`：前端服务组合契约。
- `frontend/src/services/tauri/`：原生 `invoke()` / `listen()` 的唯一前端封装点。
- `frontend/src/services/mock/`：浏览器预览和未接入领域的 Mock 实现。
