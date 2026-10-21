# UI 设计交接说明 · VocaMesh（v1.2）

> 面向后续正式前端工程（Tauri + Vue 或等价栈）。本文件只描述**设计侧**已确定的界面、组件、状态与交互；不包含任何业务实现。

## 1. 交付物地图

| 类型 | 路径 | 说明 |
|---|---|---|
| 可编辑设计源 | `UI/VocaMesh.pen` | 画布：封面、令牌与排版、组件源与状态板、IA 地图、低保真、S01–S16 高保真、暗色变体、1024 变体 |
| 可交互原型 | `UI/prototype/index.html` | 零依赖 Mock，11 个页面 + 弹窗 + 检查器 + 状态机模拟 |
| 令牌 | `UI/design-system/tokens.json` / `tokens.md` | 明暗两套 + 密度 + 对比度 |
| 组件规范 | `UI/design-system/components.md` | 尺寸、状态矩阵、禁用项 |
| 流程与 IA | `UI/flows/IA_AND_USER_FLOWS.md` | F01–F11 + 导航规则 |
| 状态矩阵 | `UI/handoff/UX_STATE_MATRIX.md` | 任务/子任务/工具/Run/审批 + 反馈契约 |
| 决策记录 | `UI/handoff/UX_DECISIONS.md` | 已定/备选/未决 |
| 验收 | `UI/qa/UX_QA_REPORT.md`、`UI/qa/REVIEW_NOTES.md` | T01–T24 结果与限制 |

## 2. 画布坐标索引（`VocaMesh.pen`）

| 区域 | 顶层框架 | 坐标（画布） |
|---|---|---|
| 封面与交付索引 | `00 · 封面与交付索引` | (0,0) 1280×700 |
| 系统组件库（可复用源） | `01 · 系统组件库` | (0,780) 1280 宽 |
| 设计令牌与排版 | `02 · 设计令牌与排版` | (1360,780) 1440 |
| 组件状态板 | `03 · 组件状态板` | (2880,780) 1400 |
| 应用骨架模板 | `TEMPLATE · 应用骨架 1280×800` | (4360,780) |
| 低保真线框 | `05 · 低保真线框 S01–S16` | (5720,1800) 1280 宽 |
| IA 与导航地图 | `06 · 信息架构与导航地图` | (5720,780) 1520 宽 |
| S01–S16 高保真 | `S0x · …` | 行 1 y=2260（x=0/1400/2800/4200），行 2 y=3180，行 3 y=4100，行 4 y=5020 |
| 暗色变体 | `S02-D / S04-D / S07-D / S11-D` | y=5940 |
| 紧凑窗口 | `S02-1024 · 紧凑窗口 1024×720` | (0,6860) |

> 画布排布已做无重叠校验：`02 · 设计令牌与排版` 高 1345（底边 y=2125），与第一行页面（y=2260）保持 135px 间距；行间 120px；横向列间 120px。

> 高保真页面均为 1280×800 窗口；暗色变体通过框架 `theme: {mode: "dark"}` 切换变量取值生成，非另画一套。

## 3. 从画布到代码的映射

| 画布元素 | 代码实现 |
|---|---|
| 变量（`bg-surface`、`status-blocked`…） | `styles.css` 中 `--bg-surface`、`--status-blocked`；`tokens.json` 为规范值 |
| `Badge/*` 组件 | `.badge` + `.is-success / .is-failure / .is-blocked / .is-neutral` |
| `Button/*` 组件 | `.btn` + `.is-primary / .is-secondary / .is-ghost / .is-danger` |
| `Nav/Item`、`Nav/Item Selected` | `.nav-item` / `.nav-item.is-active` |
| `Segmented`、`Switch/On` | `.segmented`（按钮 `.is-on`）、`.switch`（`.is-on`） |
| 应用骨架（侧栏 + 工具栏 + 内容） | `.app > .titlebar + .body > .sidebar + .main > .toolbar + .workspace > .content + .inspector` |
| 检查器 | `.inspector`（≤1080px 变 `position: fixed` 抽屉） |
| 状态徽标语义 | `ui.js` → `UI.taskStateLabel` / `UI.subStateLabel` 单一映射 |

## 4. 页面与路由建议

| 路由 | 页面 | 关键 query |
|---|---|---|
| `/welcome` | S01 首次启动 | — |
| `/` | S02 工作台 | — |
| `/tasks` | S06 全部任务 | `?state=&ws=&q=` |
| `/tasks/:id` | S04 任务详情 | `?focus=flow|chat&subtask=` |
| `/tasks/:id?inspector=logs` | S05 检查器状态 | `tab=activity|logs|results|info` |
| `/approvals` | S11 待审批 | `?approval=id&filter=pending` |
| `/history` | S12 历史记录 | `?run=#1244` |
| `/roles`、`/tools`、`/teams`、`/workspaces` | S08/S07/S09/S10 | `?id=` |
| `/settings` | S13 设置 | `?section=appearance` |
| `/notifications` | S15 通知 | `?filter=unread` |
| `/help` | S16 帮助 | `?article=` |
| `/states` | S14 状态合集（设计走查页，可不进正式产品） | — |

## 5. 需要工程侧实现的能力（当前为模拟）

1. **状态机**：任务/子任务/Run/审批的真实状态迁移；尤其 `等待暂停 → 已暂停` 需要运行时安全边界信号。
2. **Run 模型**：不可覆盖历史；重试生成新 Run 并建立“新旧关系”。
3. **审批**：作用域模型（文件/目录/命令/单次/任务内）、撤销能力、超时过期（不自动批准）。
4. **定向指令**：投递确认、增量输入能力探测、影响预览。
5. **工具探测**：路径/版本/认证状态的可靠读取；失败必须落到“无法检测”。
6. **日志流**：增量、虚拟滚动、可暂停自动滚动、复制。
7. **窗口行为**：关闭窗口 vs 退出应用的真实进程语义。
8. **关系图编辑器**：当前为 DOM/SVG 实现的轻量画布（平移、缩放、节点拖动、菜单、环检测）。正式工程可替换为图渲染库，但必须保留：节点位置持久化、`+`/`⋯` 交互、环检测阻断提示、键盘可达。
9. **并发与队列**：资源不足排队原因、目录冲突检测与隔离建议。

## 6. 设计约束（实现时不要“优化”掉）

- 不以彩色表达选中；不以绿色表达“已提交”。
- 不把 `等待暂停` 显示为 `已暂停`；不把 `正在请求停止` 显示为 `已停止`。
- 不在日志区使用 `aria-live` 自动播报；不在对话流里灌入原始日志。
- 不做全屏模态确认高风险操作（用内联审批卡 + 记录）。
- 侧栏不提供折叠；关系图不提供“仅拖拽连线”的隐性操作（所有关系变更都必须有显式菜单/表单入口）。
- 不新增第二常驻侧栏；不隐藏“待审批/历史记录/帮助”。
- 进度用计数表达，不使用虚构百分比。

## 7. 无障碍实现要点

- 焦点环：`outline: 2px solid var(--focus-outline); outline-offset: 2px`，不可移除。
- 侧栏折叠：按钮可聚焦、`aria-label`，项悬停/聚焦显示 Tooltip，`aria-current="page"` 标记当前页。
- 弹窗/抽屉：焦点移入 → 循环 → 关闭后回到触发元素（原型实现了基础版）。
- 状态徽标：文本已在 DOM 中，非仅颜色；图标 `aria-hidden`。
- 目标尺寸：交互元素热区 ≥28px；行高 34（紧凑 30）。
- 减少动效：`prefers-reduced-motion` 与设置项双通道。

## 8. 未决问题（不阻塞交接）

1. 侧栏最终宽度 240 还是 248。
2. `部分完成` 使用琥珀还是中性灰。
3. 检查器基准宽度 340 / 360 / 380。
4. 关系图正式渲染方案（本轮为静态可视化）。
5. 企业组织的入口位置（当前仅在团队与设置内预留）。
