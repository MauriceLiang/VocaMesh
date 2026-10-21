# 前端交接说明（给 Rust / 后续开发）

## 1. 已完成

- 16 个页面路由 + 桌面外壳（标题栏、固定侧栏、顶栏、命令面板、Toast）。
- 14 个通用组件（Voca*），其中弹窗/下拉/选择/开关/复选框/标签页/提示用 Reka UI 封装。
- 10 个 Service 接口 + Mock 实现（内存唯一数据源），Pinia Store 分离业务状态。
- 关系图使用 Vue Flow（自定义节点 + 平移/缩放/拖拽 + 节点菜单 + 环检测）。
- 主题/密度/灰度/平台切换，窗口缩放适配（≤1080 检查器抽屉）。
- 单元测试（Service 契约、状态语义）与 Playwright（导航、任务链路、关系图、视觉截图）脚本。

## 2. 仍由 Mock 提供的能力

| 能力 | Mock 行为 | 需要 Rust 提供 |
|---|---|---|
| 工具探测/认证 | 固定 4 条 + 状态迁移 | PATH/版本/登录状态读取；**不得读凭据** |
| 任务执行 | 定时器迁移状态 | 真实调度的状态事件（含安全边界） |
| 暂停/停止 | 定时器 → 目标态 | 运行时确认信号（失败/超时也要有） |
| 重试 | 生成新 runId | Run 不可变存储与关联 |
| 日志 | 14 行回放 + 心跳 | 流式日志 + 背压/截断 |
| 审批 | 内存记录 | 授权作用域校验、撤销、审计 |
| 定向指令 | 四段定时 | 适配器能力探测、投递确认 |
| 工作空间 | 内存对象 | 真实目录校验与边界执行 |
| 设置 | 内存 | 持久化；窗口/退出行为需桌面进程配合 |

## 3. 对接点（只改一处）

1. 新建 `src/services/tauri/*.ts` 实现 `services/contracts/*` 的接口。
2. 在 `src/services/adapters/index.ts` 增加 `createTauriServices()` 并切换 `useServices()`。
3. 事件用 `listen()` 包装为 `subscribe*(): () => void`；组件中已有取消订阅的写法（`InspectorPanel`）。

> 不要在前端伪造 Rust 命令名。命令/事件名以 Rust 侧实现为准，前端只按接口签名对接。

## 4. 已知问题 / 风险

1. **未执行构建与测试**：当前环境禁止在仓库目录运行包管理器（详见 `docs/06-test-report.md`），因此类型错误/样式偏差需在本机首次 `pnpm build` 后修复。
2. **Vue Flow 样式**：`@vue-flow/core` 默认主题与原型不同，已在节点内覆盖；但 `Controls`/`MiniMap` 未启用，若后续启用需要重新对齐视觉。
3. **Toast / 弹层 z-index**：`.v-toasts` 90、`.v-menu` 75、`.v-dialog` 70；新增浮层时请沿用该层级。
4. **任务详情布局**：使用 sticky 输入区；在极窄窗口（<900）下建议改为固定底栏。
5. **筛选**：任务“时间”筛选目前只影响 UI 状态（Mock 数据无时间戳），接 Rust 后需按 `createdAt` 过滤。
6. **导出**：CSV/Markdown 为前端 Blob 下载；正式实现应改为 Rust 生成文件并返回路径。
7. **权限**：前端只呈现权限开关；真实权限执行必须由 Rust 侧强制。

## 5. 建议的下一步

1. 本机执行 `pnpm install && pnpm typecheck && pnpm test && pnpm dev`，修掉编译告警。
2. 运行 `pnpm test:e2e` 生成 `test-results/visual/*.png`，按 `docs/04-visual-diff.md` §4 逐项核对并勾选。
3. Rust Core 先实现 `ToolService` 与 `TaskService`（最小闭环：检测 → 创建 → 状态事件），再补审批与日志流。
