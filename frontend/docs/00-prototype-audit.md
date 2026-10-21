# Phase 0 · 原型审计报告

- 审计对象：`UI/prototype/`（零依赖可交互 Mock，2026-10 版本）+ `UI/VocaMesh.pen`（高保真设计源）
- 审计方式：**实际运行原型**（Chromium 1280×800 / 1024×720 / 900×650；亮色与暗色）；DOM 几何断言；逐页点击全部交互元素（脚本化回归 200+ 次点击，0 运行时错误）；同时阅读源码交叉核对。
- 审计结论：原型可作为唯一视觉与交互基线。本工程按“1:1 还原”目标实现，不做设计改动。

> 视觉基线截图：`UI/qa/evidence/*.png`（由画布 1:1 导出，文件→页面映射见 `UI/qa/evidence/INDEX.md`）、`UI/qa/evidence/export.pdf`（16 页合集）。
> 运行时截图限制：本机集成浏览器面板仅 400px 宽，长截图存在拼接伪影，故**以画布导出图作为视觉基线**；运行时仅用于交互与几何断言（已在 `UI/qa/UX_QA_REPORT.md` 说明）。

## 1. 页面清单（16 个路由 + 状态页）

| # | 路由 | 页面 | 关键结构 |
|---|---|---|---|
| S01 | `/welcome` | 首次启动/欢迎 | 窗口栏 + 居中卡片（自动检测 4 种工具状态）+ 跳过/开始 |
| S02 | `/` | 工作台 | 任务输入（工作空间/执行方式/附件）→ 指标卡 4 个 → 进行中任务列表 + 右栏（待审批 + 最近完成） |
| S03 | `/`（dialog） | 创建任务 | 描述、工作空间、执行方式三卡、授权边界、高级选项折叠 |
| S04 | `/tasks/:id` | 任务详情 | 顶部状态+动作 → 阻塞横幅 → 摘要条 → 执行流程 → 主管对话（审批卡置顶）→ sticky 输入区 |
| S05 | `/tasks/:id?inspector=` | 执行检查器 | 右栏 360：活动 / 日志 / 成果 / 信息 |
| S06 | `/tasks` | 全部任务 | 4 维筛选 + 表格（状态/任务/空间/执行者/Run/更新/操作） |
| S07 | `/tools` | 本地工具 | 4 指标 + 工具卡（路径/版本/认证动作）+ 右栏安装指南 |
| S08 | `/agents` | Agent 角色 | 左列表 320 + 右编辑器（名称/工具/描述/指令/权限/执行参数） |
| S09 | `/teams` | Agent 团队 | 左树 320 + 关系图（Vue Flow：平移/缩放/拖节点/加下属/改上级/建角色）或成员表 |
| S10 | `/workspaces` | 工作空间 | 左列表 320 + 右详情（路径/读写边界/冲突/相关任务/团队） |
| S11 | `/approvals` | 待审批 | 左队列 320 + 右详情（命令、diff、风险、作用域、决定、记录） |
| S12 | `/history` | 历史记录 | 表格 + 右栏 Run 详情（部分成果分组、导出、重试） |
| S13 | `/settings` | 设置 | 左分区 200 + 右面板（外观/执行/通知/权限/关于） |
| S14 | `/states` | 异常与空状态合集 | 3×3 状态卡（E01–E18 子集），每卡可直接跳转 |
| S15 | `/notifications` | 通知 | Tabs（全部/未读/审批）+ 列表（查看、移除、全部已读） |
| S16 | `/help` | 帮助与反馈 | 左文章 200 + 正文 + 反馈表单 |

## 2. 交互清单（必须在工程实现中保留）

**导航与外壳**：侧栏 11 入口（固定展开 240，不折叠）；顶栏搜索→命令面板（⌘K、↑↓、Enter）、主题切换、快捷键弹窗、通知；`Esc` 逐层关闭；窗口缩放后侧栏保持 240、检查器 ≤1080 变抽屉。

**工作台**：模板弹窗→回填描述；工作空间菜单；执行方式三选一（提示联动）；创建任务→`排队中`→1.4s→`执行中`；附件 chip 可移除；指标卡→带筛选跳转全部任务；任务行→详情。

**任务**：暂停（等待暂停→已暂停）、继续、停止（请求→已停止）、仅重试失败部分（新 Run）、跳过生成部分成果、任务 `⋯`（复制链接/导出摘要/跳历史/归档）、`@` 定向指令四段投递、发送消息→主管回复、审批卡允许/拒绝。

**检查器**：4 Tab；日志搜索/级别筛选/自动滚动/复制/加载更早；成果预览弹窗；信息页“在时间线定位”高亮流程节点。

**审批**：队列筛选、允许/拒绝、记录时间线、已处理项不再提供批准入口。

**团队/关系图**：平移、缩放（45–160%）、适应视图、自动整理、拖动节点、节点 `+` 建下属、节点 `⋯`（编辑/加下属/改上级/设为主管/移除）、环检测阻断、画布内 `+ 添加 Agent`；视图切换**仅 1 处**（团队卡片标题行）。

**其余**：工具验证/设默认/配置/指定路径/安装指南/错误详情；角色编辑/删除确认/权限开关；工作空间新建/编辑边界/复制路径/解决冲突；历史筛选/加载更多/导出 CSV/行菜单；设置全部分区；通知筛选/移除/全部已读；帮助搜索/9 篇正文/反馈提交。

## 3. 组件清单（原型 → 工程组件）

| 原型 | 工程组件 | 说明 |
|---|---|---|
| `.btn`（4 变体 × 6 状态） | `VocaButton` | primary/secondary/ghost/danger，`sm`，loading/disabled |
| `.icon-btn` | `VocaIconButton` | 28px 热区，必填 aria-label |
| `.input` / `.select` / `.textarea` | `VocaInput` / `VocaSelect` / `VocaTextarea` | 高 32、圆角 6、2px 焦点轮廓 |
| `.segmented` | `VocaSegmented` | 视图内切换，不作为全局导航 |
| `.badge` | `VocaBadge` / `VocaStatus` | 颜色+图标+文字；`VocaStatus` 承载语义映射 |
| `.switch` | `VocaSwitch` | 34×20，role=switch |
| `.card` / `.panel` | `VocaCard` | 12/16 padding 两档 |
| `.menu-pop` | `VocaDropdown` | Reka DropdownMenu 封装，样式覆盖为原型 |
| `.dialog` | `VocaDialog` | Reka Dialog，760 宽，焦点陷阱 |
| `.toast` | `VocaToasts` | 右下，最多 3 条，去重 |
| `.tabs` | `VocaTabs` | 检查器/通知 |
| `.inspector` | `VocaInspector` | 360，≤1080 覆盖抽屉 |
| 侧栏 | `VocaSidebar` | 固定 240，分组 + 计数 |
| 关系图 | `features/teams/OrgChart.vue` | Vue Flow + 自定义节点 |

## 4. 设计 Token（提取自原型，禁止另立）

```text
布局：页面内边距 26/20(compact)；列表列 320；右栏 340；分区导航 200；列间距 12；
      列表卡片 12；详情面板 16；导航容器 8；控件 32/28；行高 34/30；标题左边缘 = 266
颜色（light）：canvas #F7F7F7 · surface #FFFFFF · subtle #F1F1F1 · selected #FFFFFF · inset #FAFAFA
              text #171717/#5A5A5A/#6D6D6D · border #DEDEDE/#BABABA · primary #1B1B1B
              success #187447 · failure #B13A43 · blocked #8B5A00 · neutral #666666
颜色（dark）：  canvas #101010 · surface #191919 · subtle #232323 · selected #292929 · inset #1F1F1F
              text #F4F4F4/#B9B9B9/#A0A0A0 · border #383838/#666666 · primary #F3F3F3
              success #76D2A0 · failure #FF9299 · blocked #EBC16B · neutral #B5B5B5
状态底色：    success #EAF4EE/#14261D · failure #FBECED/#2A1719 · blocked #FBF2E2/#2A2114 · neutral #F1F1F1/#232323
圆角：badge 4 · 控件 6 · 卡片 8 · 对话框 10
阴影：selected 0 1px 3px 10% + 0 4px 10px 5%；overlay 0 8px 24px 12%；dialog 0 16px 48px 18%
字体：系统栈 + 等宽栈；正文 13/辅助 11.5/元信息 10.5/标题 22–25
动效：120–160ms，上限 180ms；减少动态效果时 0ms
```

## 5. 视觉基线对照方式

1. 同一视口（1280×800、1024×720、900×650）、同一主题（light/dark）、同一数据（Mock seed 固定）。
2. Playwright 截图 → 与 `UI/qa/evidence/` 画布导出图并排比对（脚本见 `tests/e2e/visual.spec.ts`）。
3. 允许的差异：字体栅格化、系统 WebView 抗锯齿；**不允许**的差异：间距、字号层级、圆角、颜色、控件尺寸、层级顺序、溢出。

## 6. 初次交付环境说明（历史记录）

- 工程首次交付时，环境**无法执行 `pnpm install` / `vite build`**（沙箱限制）。当时：
  - 所有源码、配置、测试、文档均已落盘；
  - **未运行**构建、类型检查、单元测试与 Playwright；
  - 文档中所有“已验证”仅指原型侧（已实际运行并断言），工程侧标为“待本机 CI/开发环境验证”。
- 复现命令（本机）：
  ```bash
  cd frontend
  pnpm install
  pnpm typecheck && pnpm test
  pnpm dev        # http://localhost:5273
  pnpm build
  pnpm test:e2e   # 需先 pnpm dev
  ```
- 2026-10-21 的当前构建与测试状态见 `docs/06-test-report.md`。
