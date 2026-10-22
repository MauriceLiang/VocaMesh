# 架构

## 服务边界

```text
Vue 页面 / Pinia Store
        ↓
frontend/src/services/contracts (Services)
        ├── 浏览器：完整 Mock
        └── Tauri：本机 ToolService + 其他 Mock
                   ↓ IPC
frontend/src-tauri Commands
        ↓
vocamesh-core (领域类型、错误、ToolRuntime 契约)
        ↑
vocamesh-agent (发现、版本探测、非敏感设置)
```

Core 不依赖 Tauri 或 UI。Commands 负责状态注入、IPC 参数和事件转发；Provider 映射、路径处理、CLI 探测与设置存储留在 Agent crate。

浏览器根据 `isTauri()` 选择完整 Mock，不调用原生 IPC。桌面端只把 `ToolService` 和工具扫描事件切换为 Tauri 实现；Agent、任务、团队、审批、运行历史、工作区等其余能力仍来自 Mock。应用外壳始终显示该混合模式。

本阶段不启动 CLI 主工作模式，不执行 Agent 任务，也不提供任务日志、取消、重试或 Run 生命周期。

## 目录

- `crates/vocamesh-core/`：跨层 DTO、Provider、状态、错误、事件和 `ToolRuntime` 契约。
- `crates/vocamesh-agent/`：PATH/标准位置解析、固定版本探测、认证状态映射和原子设置存储。
- `frontend/src-tauri/`：Tauri 2 桌面入口、最小命令面和 capabilities。
- `frontend/src/services/contracts/`：与 Mock 解耦的前端服务组合契约。
- `frontend/src/services/tauri/`：原生 `invoke` / `listen` 的唯一前端封装点。
