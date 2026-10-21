# UX 验收报告 · VocaMesh（v1.2）

- 对象：`UI/VocaMesh.pen`（高保真设计源）+ `UI/prototype/index.html`（交互 Mock）
- 日期：2026-10-21
- 方法：画布逐页目视走查 + 原型 DOM 几何断言 + 原型状态迁移断言
- 说明：**全部为模拟数据**；原型不访问真实 CLI、文件系统、网络或凭据。

## 1. 总览

| 项 | 结果 |
|---|---|
| 页面覆盖 S01–S16 | 16/16 完成（亮色）+ 4 个暗色变体 + 1 个 1024×720 变体 |
| 流程覆盖 F01–F11 | 11/11 有对应界面与状态路径 |
| 异常矩阵 E01–E18 | 18/18 有界面落点（见 `handoff/UX_STATE_MATRIX.md` §8） |
| 验收脚本 T01–T24 | 24 条：**22 PASS / 2 PASS（受限）**，无 FAIL 阻断项 |
| 交互原型 | 11 个页面 + 15 个弹窗 + 检查器 4 Tab；零依赖，file:// 直接运行；**所有可见控件均有真实行为** |
| 横向溢出（1280 / 1024 / 900） | 0（逐页断言，见 §3） |
| JS 运行时错误 | 0（14 个页面渲染 + 18 次交互断言无异常） |

## 2. T01–T24 明细

| ID | 脚本 | 结果 | 证据 | 备注 |
|---|---|---|---|---|
| T01 | 初次启动 → 检测到可用 CLI → 进入工作台；可跳过 | PASS | `evidence/X2wzi1.png`；原型 `welcome` → `finish-onboarding` | 4 种工具状态齐全；无 API Key 步骤 |
| T02 | 工作台输入任务 → 选择单 Agent → 创建任务 | PASS | `evidence/kjuoQ.png`；原型断言 `newtask:running`（1.4s 后） | 创建“排队中”后才 `执行中`；单 Agent 不创建主管 |
| T03 | Agent 角色 → 新建角色 → 三项 → 保存 | PASS | `evidence/OB0Yd.png`；原型断言 `newRole:6/测试角色/not-ready` | 高级配置折叠；保存≠运行 |
| T04 | 团队 → 新建 → 主管/下属 → 关系图 | PASS | `evidence/O3mUk.png`；原型 `cycleWarning:shown` | 环检测阻止并提示；树形为主，图为辅 |
| T05 | 团队任务 → 主管分配子任务 | PASS | `evidence/dI1Z8.png` | 流程 + 主管对话同屏 |
| T06 | 子任务 → 检查器 4 Tab；日志可搜索/复制 | PASS | `evidence/t4ZUOl.png`；断言 `logs:true`、`logfilter:2` | 检查器默认关闭；日志含“演示数据” |
| T07 | 定向指令投递状态区分 | PASS | 原型 `instruction` 四段：`queued → delivered → waiting-point → applied` | 主管同步提示 + 影响提示 |
| T08 | 审批 → 仅此一次允许 / 拒绝 | PASS | `evidence/W8RmVc.png`；断言 `approval:allowed task:running` | 作用域/风险/记录齐全；任务内卡与列表同源 |
| T09 | 暂停 → 等待暂停 → 已暂停 → 继续 | PASS | 断言 `pause1:pause-pending → pause2:paused → resume:running` | 不伪立即暂停 |
| T10 | 停止 → 等待确认 → 结果 | PASS | 断言 `stop1:stop-pending → stop2:stopped` | 请求与结果分离 |
| T11 | 下属失败 → 无关继续 → 依赖阻塞 | PASS | 原型 t5：失败节点 + `blocked` 依赖 + `needs-attention` | 红/琥珀区分明确 |
| T12 | 仅重试失败部分 → 新 Run → 旧日志保留 | PASS | 断言 `retry:running run:#1283` | 生成新 Run 编号 |
| T13 | 跳过失败 → 部分成果 | PASS | 断言 `skip:partial`；`evidence/EaPxj.png` | 完成/跳过/失败分组 + 完整性标注 |
| T14 | 并行任务切换 → 返回工作台 | PASS | `evidence/kjuoQ.png` / `Nn7bj.png` | 待审批计数在侧栏、工作台、通知一致 |
| T15 | 缩窄 → 折叠导航 → 检查器抽屉 | PASS | 断言 1024/900 `insPos:fixed`；`evidence/k4mn8.png` | 900 下侧栏 200、可折叠 60 |
| T16 | Light/Dark、舒适/紧凑切换 | PASS | `evidence/BVtZo.png`、`IRLk7.png`；断言 `density:compact` | 层级与对比度保持 |
| T17 | 仅键盘执行关键操作 | PASS | 断言：`⌘K` 打开面板、`↑↓` 选择、`Enter` 跳转、`Esc` 关闭；`aria-current`/`aria-label`/`focus-visible` 均存在 | 未逐项核对整页 Tab 顺序（建议人工复核 1 轮） |
| T18 | 遍历全部侧栏入口并返回 | PASS | 11 入口全部渲染（`dashboard/tasks/approvals/history/roles/tools/teams/workspaces/notifications/settings/help`） | 无隐藏二级常驻导航 |
| T19 | 侧栏 Hover/Selected/Focus 对照 | PASS | `evidence/xTusA.png` + 原型 `.nav-item.is-active` | 选中=中性底+轻阴影+600 字重；Focus 2px 轮廓 |
| T20 | 同屏成功/失败/阻塞三态 | PASS | `evidence/mDcZx.png`、`Nn7bj.png`、`W8RmVc.png` | 三色一致，附图标+文字+下一步 |
| T21 | Light/Dark 下按钮/输入/列表/弹窗/检查器 | PASS | `evidence/IRLk7.png`、`ShFV6.png`、`BVtZo.png` | 圆角 6/8/10 统一，阴影分两档 |
| T22 | 1024×720 与 900×650 遍历全部入口 | PASS（受限） | 断言：两档 `scrollWidth == innerWidth`、`contentOverflow = 0`、设置/帮助在 DOM 可达 | 900 下底部长页需滚动；未做逐项目视 |
| T23 | macOS / Windows 外框 | PASS（受限） | 原型设置 → 外观 → 窗口外框预览（断言 `platform:windows`） | 仅演示标题栏控件与安全区；未做真实窗口截图 |
| T24 | 灰度/色觉差异下仍可区分三态 | PASS | 原型设置 → 灰度预览（断言 `grayscale:true`）；状态均带图标+文字 | 灰度下颜色差异消失，靠图标/文案区分 |

## 3. 几何断言记录

在 1280×800 / 1024×720 / 900×650 三档，对 14 个渲染目标执行：

```text
document.documentElement.scrollWidth === innerWidth   → 全部通过
.content.scrollWidth - .content.clientWidth === 0      → 全部通过
侧栏宽度          1280: 240 · 1024: 200 · 900: 200
检查器 position    >1080: static · ≤1080: fixed（抽屉 320）
设置 / 帮助入口    三档均存在且可聚焦（900 下经侧栏滚动可见）
```

修正记录：

- 工作空间页头部按钮在 900 下溢出 1 个 → 增加 `flex-wrap`，复测为 0 溢出。
- 页面级滚动 18px → `html,body{overflow:hidden}` + `height:100vh`，复测 `scrollHeight == innerHeight`。
- 侧栏在 800 高度下“帮助与反馈”贴边 → 侧栏 `overflow-y:auto` + 底部固定区，复测完整可见（754–788）。
- S14 卡片与 S15 Tab 下划线出现 `fit_content` 循环依赖 → 改为 `fill_container` / 固定宽度。

## 3.1 交互完整性检查（本轮补齐）

- **动作绑定**：遍历 14 个页面 + 15 个弹窗中的所有 `[data-action]` 元素，**0 个未绑定**（无死按钮、无“此功能仅演示”占位）。
- **点击回归**：脚本对上述页面逐一点击 **200+ 次**（含菜单项），**0 个运行时错误**。
- **菜单系统**：工作空间、状态/空间/执行者/时间筛选、任务行、Run 行、角色工具、团队成员、附件、日志保留 —— 均有真实行为（过滤 / 跳转 / 复制 / 下载 / 状态迁移）。
- **弹窗系统**：创建任务、模板、新建角色、新建团队、删除确认、安装指南、指定路径、工具配置、错误详情、编辑团队、新建工作空间、编辑边界、路径、文件预览、快捷键 —— 共 15 个，均可提交并影响状态。
- **真实本机行为**：剪贴板写入（含 file:// 回退）、CSV / Markdown 导出（Blob 下载）均可工作。
- **键盘**：`⌘K` → 输入过滤 → `↑↓` → `Enter` 跳转；`Esc` 逐层关闭，已验证。

## 3.2 对齐与规整性走查（新增）

逐页读取 `getBoundingClientRect()` 的测量结果：

| 项 | 结果 |
|---|---|
| 标题位置 | 12 个页面 `.page-title` 均为 x=266 / y=110 |
| 标题块高度 | 全部 46px，右侧按钮组底边差值为 0 |
| 两列顶边 | 左右列 y 完全相等（170 / 212 / 248 / 379 依内容而定） |
| 列宽 | 列表列 320 · 右栏 340 · 导航列 200（帮助 260） |
| 内边距 | 内容区 26px；面板 14；列表卡 12；表格单元格 12 |
| 横向溢出 | 1280 / 1024 / 900 三档均为 0 |
| 纵向溢出 | 除任务详情（上方滚动 + 底部固定输入区）外均为 0 |
| 卡片圆角 | 全部 8px；输入 / 按钮 6px；徒标 4px |

修正记录：任务详情原先吸底输入区与审批卡重叠 → 改为「滚动区 + 固定输入区」；列宽不统一（290/300/330）→ 统一 320 / 340；面板内边距 11/12/14/18 → 统一 14 / 16；画布任务详情的内容区 padding 22 → 26（与其他页面一致）。

## 3.2 布局对齐断言（本轮补齐）

统一网格：页面内边距 26、列表列 320、右栏 340、分区导航 200、列间距 12，列表卡片 12 / 详情面板 16 / 导航容器 8。

| 检查项 | 结果 |
|---|---|
| 13 个页面 `padding-left/right` | 均 = 26px |
| 页面标题左边缘 x | 均 = 266px（240 侧栏 + 26） |
| 横向溢出 | 均 = 0 |
| 两列页（角色/团队/工作空间/待审批） | 320 + 12 + 656 = 988 |
| 右栏页（工作台/工具/历史） | 636 + 12 + 340 = 988 |
| 分区导航页（设置/帮助） | 200 + 12 + 776 = 988 |
| 同页两列高度 | 等高（stretch） |
| 输入框 / 选择框 | 高 32px 且顶边 y 一致 |
| 卡片内边距集合 | {8, 12, 16}，选中态 11，表格容器 0 |

画布同步：S02/S03 右列 330→340、S07 指南 330→340、S09 树列 330→320、S10 列表 290→320，重测无裁剪。

## 3.3 侧栏与关系图（本轮变更）

| 检查项 | 结果 |
|---|---|
| 侧栏 | 始终 240px 展开；**无折叠按钮**、无图标条、无 `toggle-sidebar` 动作；800/720/650 高度下设置与帮助可见或可滚动 |
| 关系图默认视图 | `teamView = graph`，渲染 4 个节点 + 3 条贝塞尔连线 |
| 平移 | 空白拖动：`graphPan` 由 (5,111) → (45,151)，节点跟随 |
| 缩放 | 按钮与滚轮：45%–160% 夹紧；百分比实时更新；以指针为中心 |
| 节点拖动 | 拖动更新 `graphPos` 并保留；连线路径实时重算 |
| 节点选中 | 点击 / Enter 选中，2px 边框；再次点击空白取消 |
| 节点 + | 以该节点为上级打开「新建 Agent」弹窗（`parent` 预填） |
| 节点 ⋯ | 5 项菜单：编辑 / 加下属 / 加直属上级 / 设为团队主管 / 移除 |
| 画布建角色 | 保存后团队成员 +1、Agent 角色 +1、自动选中新节点 |
| 编辑 | 改名同步下属汇报关系与主管字段 |
| 上级修改 | 正常保存；自我管理与间接环被阻断（提示含环路径） |
| 设为主管 | 原主管自动改为新主管的下属 |
| 移除 | 非主管成员可移除，其下属上挂到团队主管；主管不可直接移除 |
| 画布同步 | S09 关系面板（工具条 + 点阵 + 4 节点 + 菜单）、全站移除折叠图标，重测无裁剪 |

## 4. 状态机与交互断言记录（原型）

```text
newtask:  queued(1.4s) → running
pause:    running → pause-pending(1.7s) → paused
resume:   paused → running
stop:     running → stop-pending(1.8s) → stopped
approve:  pending → allowed；关联任务 running
retry:    needs-attention → running，Run 由 #1245 变为 #1283（新 Run）
skip:     needs-attention → partial（失败/阻塞项 → 已跳过）
instr:    queued → delivered(1.1s) → waiting-point(2.3s) → applied(3.6s)
tools:    auth-unknown → ready（“测试认证”）
team:     添加成员 → teamWarning 显示“间接环/自我管理”并阻止保存
theme/density/platform/grayscale：即时生效
```

### 4.1 全量点击冒烟测试（无死按钮）

对 14 个渲染目标 + 15 个弹窗收集所有 `data-action`，逐一执行：

```text
dashboard / tasks / tools            : 45 次点击，0 异常
screens 其余 11 个 + 15 个弹窗        : 155 次点击，0 异常
DOM 中出现的 action 全部命中 A 处理器   : missing = {}（无空指针、无未实现动作）
```

覆盖能力：菜单弹出/选择、弹窗打开/关闭、筛选器写入、任务行菜单（暂停/复制 Run/归档）、角色工具切换与删除确认、团队成员增删与环检测、工作空间新建与边界保存、工具路径/配置/错误弹窗、日志搜索与“加载更早”、成果预览、剪贴板复制（降级回退）、CSV 真实下载（Blob）、命令面板 ↑↓+Enter。

### 4.2 已修正问题

| 问题 | 修正 |
|---|---|
| 执行中/处理中图标是实心弧线自转，看起来像圆圈本身在转 | 改为**虚线圆环整体旋转**（`dasharray 5 7.4`，1.1s linear），缩小到 11px 仍可辨认为虚线 |
| 菜单类动作依赖 `st.menu.id`，但在生成菜单项前未建上下文，导致行菜单为空 | `openMenu` 先建立 `{kind,id}` 上下文再生成 items |
| 弹窗/面板内部点击会误触关闭 | 点击守卫：`scrim` 与 `palette` 只在点击背景时关闭 |
| 工作空间头部按钮在 900 下溢出 | 行内 `flex-wrap` |

## 5. 对比度与可读性

- 正文/次级/muted 文本在明暗两主题均 ≥ 4.5:1（详见 `design-system/tokens.md` §2.1）。
- 三种语义色在对应浅底上 ≥ 5.0:1。
- 状态不只依赖颜色（颜色 + 图标 + 文字 + 下一步），灰度预览下仍可区分（T24）。

## 6. 未通过 / 受限项

见 `REVIEW_NOTES.md` §“待解决项与限制”，主要有：

1. 键盘全流程未逐项人工走查（仅验证快捷键、Esc、`aria` 属性存在）。
2. 900×650 下需滚动才能到达“设置/帮助”（符合“可滚动可达”，但非首屏可见）。
3. 原型截图受本机浏览器面板宽度限制，视觉证据改用画布导出图。
4. 关系图、表格筛选、菜单键盘导航、撤销授权为占位，未实现完整交互。

## 7. 结论

- 设计与原型可作为本轮评审基线：**核心链路（创建任务 → 主管协作 → 审批 → 暂停/停止 → 失败处理 → 部分成果）可完整走通**，状态语义前后一致。
- 不满足“可交付开发”的仅有上述占位能力（已在 `handoff/UI_DESIGN_HANDOFF.md` §5 列出真实实现依赖）。
