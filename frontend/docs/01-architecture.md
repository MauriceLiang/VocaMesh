# 架构说明

## 分层

```text
页面 (features/*)  →  Pinia Store  →  Service 接口  →  Mock 实现 / Tauri 实现
                                       ↑
                                  adapters/useServices()
```

- **页面**只做展示与交互，不直接访问数据源；不判断运行环境。
- **Store** 负责页面状态、筛选、选中项、提示，并调用 Service。
- **Service** 通过 `services/contracts/*` 定义；`services/mock/*` 提供内存实现。
- **adapters** 决定当前注入哪个实现（`mode: "mock" | "tauri"`），是未来接 Rust 的唯一入口。

## 关键设计

| 决策 | 原因 |
|---|---|
| Service 全部返回 Promise（除订阅） | 与 Tauri IPC 的异步语义一致，避免后面大改签名 |
| 事件用 `subscribe*(): () => void` | 日志/状态是流式的，不能用轮询模型 |
| `MockDb` 单例式内存数据 | 保证工作台、全部任务、审批、通知的数字一致 |
| 状态语义集中在 `lib/status.ts` | 同一状态在所有页面颜色/图标/文案一致 |
| Mock 显式 `sourceLabel` + 顶栏 “mock 演示” 标识 | 不让 Mock 被误认为真实执行 |
| 关系图坐标存 Store，不写回 Service | 组织关系（业务）与视图布局（偏好）分离 |

## 与原型的一致性策略

- 视觉：把原型 `styles.css` 的 token 与组件类**原样移植**为 `tokens.css` + `index.css` 的 `@layer components`（`.v-*`），页面使用这些类 + 少量 Tailwind 布局类，避免“重新设计”。
- 行为：状态机、文案、按钮组合按原型逐页对照实现（见 `docs/04-visual-diff.md`）。

## 目录约定

- 通用组件：`components/ui`（Voca* 前缀）
- 业务组件：`features/<domain>/`（如 `OrgChart`、`TaskFlow`）
- 页面级布局组合：`components/layout`

## 类型安全

- `strict: true`，`noUnusedLocals` / `noUnusedParameters` 开启。
- Service 错误统一为 `ServiceError { code, detail }`，页面据 `code` 决定提示文案。
