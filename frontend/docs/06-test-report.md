# 测试与构建报告

## 1. 本次检查（2026-10-21）

| 项目 | 命令 | 结果 |
|---|---|---|
| 类型检查与生产构建 | `pnpm build` | **通过**（`vue-tsc --noEmit`、Vite build） |
| 单元测试 | `pnpm test` | **通过**（2 个测试文件，9 个测试） |
| Playwright E2E | `pnpm test:e2e` | **未完成**（本机缺少 Playwright Chromium 可执行文件，浏览器未能启动） |

构建首次检查发现并修复了路由名类型、团队图事件分发、未使用事件总线和 Node 类型依赖问题。`@types/node` 已加入开发依赖和锁文件。

E2E 执行器发现了 78 个用例，但启动浏览器时报告缺少 `chromium_headless_shell`；用例未进入页面交互断言，因此不能将 E2E 视为通过。浏览器可用后运行：

```bash
pnpm exec playwright install chromium
pnpm test:e2e
```

本次未执行完整 `pnpm install`；已有依赖可用于构建和单元测试。

## 2. 覆盖范围

- 单元测试覆盖任务状态迁移、暂停与工具状态边界，以及 Mock Service 行为。
- E2E 脚本覆盖侧栏与页面导航、命令面板、任务流程、团队关系图和多视口视觉截图；因浏览器缺失，本次未验证这些行为。
- 视觉基线对照仍需参见 `docs/04-visual-diff.md` 的待核对清单。
