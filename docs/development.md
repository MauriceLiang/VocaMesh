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

Tauri 配置位于 `frontend/src-tauri/tauri.conf.json`。首次任务选择本机目录、查看 Claude Code readiness、确认只读范围和数据发送后，才会向 Runtime 提交任务。Runtime 在 Unix 上启动过滤快照；Windows 目前显示不可执行。

## 本地质量门禁

```bash
pnpm --dir frontend install --frozen-lockfile
pnpm --dir frontend typecheck
pnpm --dir frontend test
pnpm --dir frontend build
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo check --workspace --all-targets --all-features
```

本地脚本由开发者运行。按仓库维护者要求，当前没有 GitHub Actions workflow；推送后不会自动产生 CI 结果。Playwright E2E 需要显式安装浏览器并单独记录结果。

## 真实 CLI 与平台验收

自动化用例覆盖纯解析、过滤快照、SQLite 状态/容量/恢复、Unix fake CLI 成功/非零退出/停止/超时/畸形输出协议与幂等，以及前端 Mock/Tauri 服务边界。Fake CLI 不访问真实账号或模型。真实登录、模型请求、费用、桌面 GUI/目录选择、停止和文件哈希对比需在开发者明确确认后手工运行；不得把 fake CLI 单测或 CLI 进程创建当作 GUI/真实执行验收。Windows 当前明确禁用真实执行；未实际验证的平台必须在 `implementation-report.md` 中单独列出。
