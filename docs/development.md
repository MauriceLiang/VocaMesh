# 开发与验证

## 环境

- Node.js 与 pnpm 9；Rust stable 与 Cargo。
- Tauri 2 桌面依赖：macOS 完整 Xcode 或 Windows C++ Build Tools/WebView2。
- 自动化测试不要求真实 Claude CLI 登录，不触发模型请求或费用。

## 浏览器 Mock

```bash
pnpm --dir frontend install --frozen-lockfile
pnpm --dir frontend dev
```

Vite 在 `127.0.0.1:5273` 运行。浏览器使用完整 Mock，不访问目录选择器、Tauri IPC 或真实 CLI。

## 桌面模式

```bash
pnpm --dir frontend tauri dev
pnpm --dir frontend tauri build
```

Tauri 配置位于 `frontend/src-tauri/tauri.conf.json`。开始任务前需通过策略、CLI 版本和认证 readiness，并确认只读范围与数据发送。当前 Unix/macOS 因 managed hooks 无法可靠核验而 fail-closed；Windows 也显示不可执行。Unix 快照使用已验证的工作区目录描述符；符号链接、特殊文件或复制错误会阻止启动。

## 本地质量门禁

在仓库根目录运行：

```bash
./scripts/verify-local.sh
```

PowerShell：

```powershell
./scripts/verify-local.ps1
```

脚本逐条记录 PASS/FAIL/BLOCKED，并在任何失败或缺少命令时以非零状态退出。执行检查为：

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features --offline -- -D warnings`
- `cargo test --workspace --offline --quiet`
- `cargo check --workspace --all-targets --all-features --offline`
- `pnpm --dir frontend typecheck`
- `pnpm --dir frontend test`
- `pnpm --dir frontend build`
- `git diff --check`

脚本不安装依赖、不登录、不调用真实模型。按仓库维护者要求，当前没有 GitHub Actions workflow；推送后不会自动产生 CI 结果。Playwright E2E 需要显式安装浏览器并单独记录结果。PowerShell 脚本仅在 Windows PowerShell 环境实际执行后才能记为通过。

## 真实 CLI 与平台验收

自动化用例覆盖纯解析、目录描述符快照、实际复制上限、SQLite v1→v2 migration/历史保留/回滚/future schema 拒绝、Unknown Run 检查/恢复、进程组状态与清理失败、Unix fake CLI 成功/失败/停止/超时/孤儿进程组/诊断脱敏，以及前端 readiness 与恢复面板 IPC/确认流程。Fake CLI 不访问真实账号或模型。定向运行：

```bash
cargo test -p vocamesh-runtime --offline
pnpm --dir frontend test -- --run
```

真实模型请求只有在 M1–M3 策略门禁通过且开发者另行明确授权后才能手工运行；当前 managed hooks 风险未能可靠排除，真实任务验收应标记 BLOCKED。GUI/目录选择、桌面停止和重启恢复须在有完整 Xcode 和可验收桌面会话的机器上单独验收：使用受控 fixture 创建 Unknown Run，分别检查仍运行、退出候选、无法核实和缺少 PGID；验证确认前不归档、确认后 Run 记为失败、快照清理和再次启动门禁行为。不得把 fake CLI 单测或 CLI 进程创建当作 GUI/真实执行验收。Windows 当前明确禁用真实执行；未实际验证的平台必须在 `implementation-report.md` 中单独列出。
