# 测试与构建报告

## 1. 本轮结果（2026-10-22）

环境：macOS，Node.js v24.22.1，pnpm 9.15.9；E2E 浏览器为本机 Google Chrome。

| 检查 | 命令 / 环境 | 结果 |
|---|---|---|
| 锁文件安装 | `pnpm install --frozen-lockfile` | **未完成**：pnpm registry metadata 请求失败，报 `ERR_PNPM_META_FETCH_FAIL` / `connect EPERM 127.0.0.1:7897`；未改锁文件，使用本机已有依赖 |
| 类型检查 | `pnpm typecheck` | **通过**（`vue-tsc --noEmit`） |
| 生产构建 | `pnpm build` | **通过**（Vue 类型检查 + Vite 6.4.5 build） |
| 单元测试 | `pnpm test` | **通过**（3 个文件，18 个测试） |
| Playwright E2E | 见下文的系统 Chrome 命令 | **通过**（102/102；完整执行 3 个视口的任务流程、筛选、导航、组织图与截图用例） |
| 视觉截图与对照 | `tests/e2e/visual.spec.ts` + 人工基线核对 | **通过本轮范围**（75 张截图；无横向溢出；第 4 节列明人工核对页面和预期差异） |

### E2E 环境说明

标准 `pnpm test:e2e` 所需的 Playwright Chromium 未安装成功。默认浏览器目录创建锁文件时报 `EPERM`；改用 `/private/tmp/vocamesh-playwright-browsers` 后，浏览器下载仍因代理连接 `127.0.0.1:7897` 被拒绝。沙箱中的 Vite 监听也曾因 `listen EPERM` 失败。

本轮改用已安装的 Google Chrome，临时 Playwright 配置将浏览器设为 `channel: "chrome"`、Vite 地址固定为 `127.0.0.1`，随后在允许本地监听的执行环境完整运行测试。实际命令为：

```bash
pnpm exec playwright test --config=playwright.chrome.config.ts --workers=3
```

第一次按默认并发运行时，有 1 项在 Chrome 创建测试上下文阶段超时；降至 3 个 worker 后完整重跑，结果 **102 passed**。套件包含自定义任务保持 `draft` 待配置、且详情页不显示调度和对话入口的三视口用例。临时配置已在本轮结束前移除，未改变项目默认 Playwright 浏览器配置。要复跑标准命令，先在网络和文件权限可用的环境执行 `pnpm exec playwright install chromium`。

### 视觉检查范围

- 自动截图覆盖 11 个页面与创建任务弹窗，在 1280×800、1024×720、900×650 的亮/暗主题下共生成 72 张截图；自定义待配置详情另生成 3 张截图。每张截图前检查页面主体可见，并断言文档没有横向溢出。
- 人工与 `UI/qa/evidence/INDEX.md` 中的基线对照：1280×800 的工作台、全部任务、任务详情、创建弹窗、Agent 团队、设置；暗色额外核对工作台和任务详情。1024×720 与 900×650 检查上述六页的当前布局。
- 窄画布下团队关系图纵向排布。E2E 检查所有节点保持在图表画布内，并验证末位节点可滚动到达且菜单可用。
- 人工查看了三个视口下的自定义待配置详情截图；未见文字裁切或横向溢出。
- 预期差异是工作台完成指标改为 Store 计数，以及创建弹窗显式展示实际执行者和 Mock 边界；未发现所核对页面的非预期布局偏差。截图保存在 `frontend/test-results/visual/{1280x800,1024x720,900x650}/`（该目录由测试生成）。

## 2. 覆盖范围

- 单元测试：任务状态语义、Mock Service 执行者参数校验、状态事件同步与订阅清理、加载期间事件竞态、状态映射（含待配置）、组合筛选和时间边界。
- E2E：创建 → 排队 → 执行 → 暂停 → 继续 → 停止；团队和单 Agent 的选定执行者；自定义任务待配置边界；状态/时间/搜索筛选及清空；导航、组织图菜单和多视口截图。
- 状态/创建流程依赖当前 Mock Services。通过 E2E 仅证明浏览器中的前端行为，不证明 Rust Core、Tauri、CLI、真实权限或 Agent 执行。

## 3. 仓库交付状态

- 仓库内没有 `.github/workflows/`，本轮没有配置 CI。
- 没有访问或修改 GitHub 分支保护设置。
- 标准 Playwright Chromium 安装未完成；完整测试使用系统 Chrome 通过，CI 仍需在可安装浏览器的环境执行标准 `pnpm test:e2e`。
