# VocaMesh 前端（Vue 3 + TypeScript，Mock 阶段）

> 视觉基线：`UI/prototype/`（可交互原型）+ `UI/VocaMesh.pen`（高保真画布）。
> 本工程是**高保真工程化还原**，不重新设计；不包含 Rust / Tauri / 真实 Agent 执行。

## 快速开始

```bash
cd frontend
pnpm install
pnpm dev          # http://localhost:5273
```

| 命令 | 作用 |
|---|---|
| `pnpm dev` | 开发服务器（hash 路由，直接访问 `/#/dashboard`） |
| `pnpm build` | `vue-tsc -b` 类型检查 + `vite build` |
| `pnpm typecheck` | 仅类型检查 |
| `pnpm test` | Vitest 单元测试（Service 契约 + 状态语义） |
| `pnpm test:e2e` | Playwright（需先 `pnpm dev`，或由配置自动启动） |

## 目录

```text
src/
├── app/                 # 预留（当前外壳在 components/layout）
├── router/              # 16 条路由，与原型页面一一对应
├── components/
│   ├── ui/              # VocaButton / VocaInput / VocaSelect / VocaDialog / VocaDropdown / …
│   └── layout/          # AppShell / AppTitleBar / SidebarNav / TopBar / VocaInspector / CommandPalette
├── features/            # 按页面拆分：dashboard tasks agents teams approvals workspaces history tools settings notifications help states onboarding inspector
├── services/
│   ├── contracts/       # 10 个业务接口（含 JSDoc：用途/时机/参数/返回/错误/Mock/Rust 职责）
│   ├── mock/            # MockDb（唯一事实来源）+ 全部 Mock Service
│   └── adapters/        # Service 注册与 Tauri 适配入口（待实现）
├── stores/              # Pinia：app / tasks / agents / tools / teams / approvals / workspaces / runs / notifications
├── composables/         # 预留
├── lib/                 # cn() / 状态语义映射（status.ts, instruction.ts）
├── styles/              # tokens.css（与原型完全一致）+ index.css（Tailwind 4 + 组件类）
└── types/               # 领域类型与 ServiceError
```

## 页面（16）

`/welcome` · `/` 工作台 · `/tasks` · `/tasks/:id` 任务详情（含检查器）· `/approvals` · `/history` · `/agents` · `/tools` · `/teams`（关系图/树形）· `/workspaces` · `/settings` · `/notifications` · `/help` · `/states`

## 关键约束（来自设计规范，勿改）

- 左侧导航**固定展开 240px，无折叠按钮**；分组：工作 / Agent 与协作 / 应用；底部设置、帮助与反馈。
- 页面内边距 26、列表列 320、右栏 340、分区导航 200、列间距 12；列表卡片 12、详情面板 16、导航容器 8。
- 黑白灰主色；成功绿 / 失败红 / 阻塞琥珀；选中 = 中性底 + 轻阴影 + 字重；Focus = 2px 轮廓。
- 状态文案与颜色只来自 `lib/status.ts`，页面不得自行判定配色。
- 创建任务 ≠ 开始执行；暂停 = 等待暂停 → 已暂停；停止 = 请求 → 结果；重试 = 新 Run。

## 当前状态

- 页面、组件、Store、Service、Mock 数据、单元/E2E 测试脚本均已落盘。
- **当前验证**：`pnpm build` 与 `pnpm test` 已通过；Playwright E2E 因本机缺少 Chromium 浏览器尚未完成，见 `docs/06-test-report.md`。
- Mock 与真实能力的边界见 `docs/05-handoff.md`。
