# 视觉对照记录（原型 ↔ 实现）

- 基线：`UI/qa/evidence/*.png`（画布 1:1 导出，映射见 `UI/qa/evidence/INDEX.md`）+ `UI/prototype/`（运行时行为）
- 实现：`frontend/`（本工程）
- 对比方式：同视口（1280×800 / 1024×720 / 900×650）、同主题、同 Mock seed；Playwright 截图输出 `test-results/visual/`

> **验证状态（2026-10-22）**
> - 原型侧：已在本机运行并逐页操作，DOM 几何断言见 `UI/qa/UX_QA_REPORT.md`。
> - 实现侧：类型检查、生产构建、18 个单元测试和 99 个 Playwright 用例通过；Playwright 使用本机 Google Chrome 与临时配置运行，环境限制见 `docs/06-test-report.md`。
> - 视觉截图覆盖 1280×800、1024×720、900×650，亮/暗主题；测试检查页面可见及无横向溢出。关键页面另与现有高保真基线人工对照，范围与观察记录见第 4 节。

## 1. 结构对照（逐页）

| 页面 | 原型结构 | 实现 | 状态 |
|---|---|---|---|
| 工作台 | 标题+操作 / 输入卡 / 4 指标 / 任务列+右栏 | `features/dashboard/DashboardView.vue`：PageHeader、composer card、`.v-metrics`、`.v-stack` + `.v-col-rail` | 1280 基线已对照；统计值按当前 Mock Store 数据显示 |
| 全部任务 | 筛选行 + 表格 + 说明 | `TasksView.vue`：搜索 + 3 个下拉 + 清除；`.v-table` | 1280 基线已对照；筛选值使用内部状态码 |
| 任务详情 | 头部动作 / 横幅 / 摘要 / 流程 / 对话 / sticky 输入 / 检查器 | `TaskDetailView.vue` + `TaskFlow` + `TaskChat` + `InspectorPanel` | 1280 亮/暗基线已对照；状态语义按中央映射 |
| 待审批 | 左队列 + 右详情（命令/风险/作用域/决定/记录） | `ApprovalsView.vue` | 多视口截图及溢出断言通过 |
| 历史记录 | 筛选 + 表格 + 右栏部分成果 | `HistoryView.vue` | 多视口截图及溢出断言通过 |
| Agent 角色 | 左列表 320 + 右编辑器 | `AgentsView.vue` | 多视口截图及溢出断言通过 |
| 本地工具 | 4 指标 + 工具卡 + 右栏指南 | `ToolsView.vue` | 多视口截图及溢出断言通过 |
| Agent 团队 | 左树 320 + 关系图（Vue Flow）/ 成员表 | `TeamsView.vue` + `OrgChart.vue` + `AgentNode.vue` | 1280 基线已对照；窄画布改为纵向排布并验证节点与菜单 |
| 工作空间 | 左列表 320 + 右详情 + 冲突横幅 | `WorkspacesView.vue` | 多视口截图及溢出断言通过 |
| 设置 | 左分区 200 + 右面板 | `SettingsView.vue` | 1280 基线已对照；1024/900 布局截图已检查 |
| 通知 | Tabs + 列表 | `NotificationsView.vue` | 多视口截图及溢出断言通过 |
| 帮助 | 左文章 200 + 正文 + 反馈 | `HelpView.vue` | 多视口截图及溢出断言通过 |
| 状态合集 | 3×3 卡片 | `StatesView.vue` | 多视口截图及溢出断言通过 |
| 首次启动 | 居中卡片 + 工具状态列表 | `WelcomeView.vue` | 多视口截图及溢出断言通过 |

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
| 创建任务 → 排队 → 执行 | 团队/单 Agent 1.4s | Mock Service 定时迁移；自定义占位保持 `draft` | 与当前执行模式一致；不代表真实执行 |
| 暂停两段式 | pause-pending → paused | 同上 1.7s | 一致 |
| 停止 | stop-pending → stopped | 1.8s | 一致 |
| 重试 | 新 Run + 旧记录提示 | `retryFailed` 生成新 runId | 一致 |
| 部分成果 | 失败/阻塞 → 已跳过 | `skipAndFinalizePartial` | 一致 |
| 审批 | 允许/拒绝改变任务状态 | `ApprovalService.decide` | 一致 |
| 定向指令四段 | queued → delivered → waiting-point → applied | `stores/tasks.setInstruction` | 一致 |
| 关系图 | 平移/缩放/拖节点/加下属/改上级/环检测 | Vue Flow + `TeamService` | 一致（Vue Flow 自带平移缩放） |
| 视图切换唯一 | 团队卡片标题行 | `TeamsView` 内 `.v-segmented` 单处 | 一致 |
| 主题/密度/灰度/平台 | 设置内切换 | `stores/app.watchEffect` 写入 `data-*` | 一致 |

## 4. 本轮截图与人工核对（2026-10-22）

- `tests/e2e/visual.spec.ts` 为 11 个页面和创建任务弹窗，在 1280×800、1024×720、900×650 及亮/暗主题生成 72 张截图；每张截图前检查页面内容可见且文档无横向溢出。截图位于 `frontend/test-results/visual/{1280x800,1024x720,900x650}/`。
- 人工对照 `UI/qa/evidence/INDEX.md` 的基线：1280×800 检查工作台、全部任务、任务详情、创建弹窗、Agent 团队、设置；暗色额外检查工作台和任务详情。1024×720 与 900×650 检查上述六页的当前布局。
- 预期变化：工作台指标由硬编码“今日已完成 8”改为 Store 的“已完成”数量；创建弹窗明确显示实际执行者选择，并将目录冲突文案标为 Mock 提示。其他页面保持原有层级、间距和状态配色，未发现非预期裁切或横向溢出。
- 团队图在画布宽度小于 640px 时改为纵向布局，保留节点拖动和菜单；E2E 检查 4 个节点都在画布范围内，并将末位节点滚动到视口后打开菜单。窄窗口下右侧详情面板可纵向滚动以查看完整图形。
- 本轮视觉检查只确认已列出的页面、主题与视口；不代表未列出的浏览器/操作系统字体栅格化完全一致。
