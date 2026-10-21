# 视觉对照记录（原型 ↔ 实现）

- 基线：`UI/qa/evidence/*.png`（画布 1:1 导出，映射见 `UI/qa/evidence/INDEX.md`）+ `UI/prototype/`（运行时行为）
- 实现：`frontend/`（本工程）
- 对比方式：同视口（1280×800 / 1024×720 / 900×650）、同主题、同 Mock seed；Playwright 截图输出 `test-results/visual/`

> **验证状态（如实说明）**
> - 原型侧：已在本会话中实际运行、逐页操作并做 DOM 几何断言（详见 `UI/qa/UX_QA_REPORT.md`）。
> - 实现侧：2026-10-21 已通过 `pnpm build` 与 `pnpm test`（9 个单元测试）。
> - Playwright E2E 与视觉截图尚未完成：本机缺少 Playwright Chromium 可执行文件。
> 因此下表的“实现值”仍是**按 Token 与组件类推导的目标值**，需要浏览器可用后逐行核对并更新为“已核对”。

## 1. 结构对照（逐页）

| 页面 | 原型结构 | 实现 | 状态 |
|---|---|---|---|
| 工作台 | 标题+操作 / 输入卡 / 4 指标 / 任务列+右栏 | `features/dashboard/DashboardView.vue`：PageHeader、composer card、`.v-metrics`、`.v-stack` + `.v-col-rail` | 结构一致（待截图核对） |
| 全部任务 | 筛选行 + 表格 + 说明 | `TasksView.vue`：搜索 + 3 个下拉 + 清除；`.v-table` | 结构一致 |
| 任务详情 | 头部动作 / 横幅 / 摘要 / 流程 / 对话 / sticky 输入 / 检查器 | `TaskDetailView.vue` + `TaskFlow` + `TaskChat` + `InspectorPanel` | 结构一致 |
| 待审批 | 左队列 + 右详情（命令/风险/作用域/决定/记录） | `ApprovalsView.vue` | 结构一致 |
| 历史记录 | 筛选 + 表格 + 右栏部分成果 | `HistoryView.vue` | 结构一致 |
| Agent 角色 | 左列表 320 + 右编辑器 | `AgentsView.vue` | 结构一致 |
| 本地工具 | 4 指标 + 工具卡 + 右栏指南 | `ToolsView.vue` | 结构一致 |
| Agent 团队 | 左树 320 + 关系图（Vue Flow）/ 成员表 | `TeamsView.vue` + `OrgChart.vue` + `AgentNode.vue` | 结构一致（图为 Vue Flow 实现） |
| 工作空间 | 左列表 320 + 右详情 + 冲突横幅 | `WorkspacesView.vue` | 结构一致 |
| 设置 | 左分区 200 + 右面板 | `SettingsView.vue` | 结构一致 |
| 通知 | Tabs + 列表 | `NotificationsView.vue` | 结构一致 |
| 帮助 | 左文章 200 + 正文 + 反馈 | `HelpView.vue` | 结构一致 |
| 状态合集 | 3×3 卡片 | `StatesView.vue` | 结构一致 |
| 首次启动 | 居中卡片 + 工具状态列表 | `WelcomeView.vue` | 结构一致 |

## 2. 关键数值对照

| 项 | 原型 | 实现 | 状态 |
|---|---|---|---|
| 侧栏宽度 | 240（固定，无折叠） | `.v-sidebar { width: var(--sidebar-w) } = 240` | 一致 |
| 页面内边距 | 26 | `.v-content { padding: var(--pad) }` | 一致 |
| 列表列 / 右栏 / 分区导航 | 320 / 340 / 200 | `.v-col-list` / `.v-col-rail` / `.v-col-nav` | 一致 |
| 控件高 / 行高 | 32 / 34 | `--control-h` / `--row-h` | 一致 |
| 圆角 | 4 / 6 / 8 / 10 | `--radius-sm/md/lg/xl` | 一致 |
| 阴影 | 三档 | `--shadow-selected/overlay/dialog` | 一致 |
| 标题左边缘 | 266 | 240 + 26 | 一致 |
| 检查器 | 360，≤1080 抽屉 | `.v-inspector` + 媒体查询 | 一致 |

## 3. 交互对照（必须逐项点过）

| 交互 | 原型 | 实现 | 状态 |
|---|---|---|---|
| 创建任务 → 排队 → 执行 | 1.4s | `MockTaskService.createTask` 定时迁移 | 逻辑一致 |
| 暂停两段式 | pause-pending → paused | 同上 1.7s | 一致 |
| 停止 | stop-pending → stopped | 1.8s | 一致 |
| 重试 | 新 Run + 旧记录提示 | `retryFailed` 生成新 runId | 一致 |
| 部分成果 | 失败/阻塞 → 已跳过 | `skipAndFinalizePartial` | 一致 |
| 审批 | 允许/拒绝改变任务状态 | `ApprovalService.decide` | 一致 |
| 定向指令四段 | queued → delivered → waiting-point → applied | `stores/tasks.setInstruction` | 一致 |
| 关系图 | 平移/缩放/拖节点/加下属/改上级/环检测 | Vue Flow + `TeamService` | 一致（Vue Flow 自带平移缩放） |
| 视图切换唯一 | 团队卡片标题行 | `TeamsView` 内 `.v-segmented` 单处 | 一致 |
| 主题/密度/灰度/平台 | 设置内切换 | `stores/app.watchEffect` 写入 `data-*` | 一致 |

## 4. 待核对清单（开发机执行截图后填写）

- [ ] 工作台：输入卡与指标卡间距、右栏宽度、任务行状态徽标位置
- [ ] 任务详情：摘要条数字对齐、流程节点 44–52px 行高、sticky 输入区不遮挡审批卡
- [ ] 检查器：日志四列宽度（52/40/68/1fr）、WARN 行底色、底部按钮对齐
- [ ] 表格：表头/行左右 padding 均为 12
- [ ] 设置：分区导航与面板顶部对齐
- [ ] 暗色：边框可见度、选中阴影
- [ ] 1024 / 900：无横向溢出，检查器为抽屉
- [ ] 关系图：节点尺寸 196×54、连线颜色、`+ / ⋯` 位置
