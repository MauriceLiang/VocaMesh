# 开发与验证

## 环境

- Node.js 22 或兼容当前项目的版本；pnpm 9。
- Rust stable 和 Cargo。
- Tauri 2 的 macOS/Windows 桌面开发依赖。macOS 需 Xcode（完整桌面构建）；Windows 需 Microsoft C++ Build Tools 和 WebView2。
- 不需要安装、登录或配置 Codex、Claude Code 或 OpenCode 来运行测试。

## 浏览器 Mock

```bash
pnpm --dir frontend install --frozen-lockfile
pnpm --dir frontend dev
```

Vite 绑定 `127.0.0.1:5273`。浏览器不会调用 Tauri IPC 或扫描本机 CLI。

## Tauri 桌面

从仓库根目录执行：

```bash
pnpm --dir frontend tauri dev
pnpm --dir frontend tauri build
```

Tauri 配置位于 `frontend/src-tauri/tauri.conf.json`；`frontendDist: ../dist` 相对 Tauri 配置目录指向 `frontend/dist`。

## 检查

```bash
pnpm --dir frontend typecheck
pnpm --dir frontend test
pnpm --dir frontend build
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

独立 Core 检查可用 `cargo test -p vocamesh-core`。Agent 单测注入假进程探测器，不依赖本机 CLI 或认证。E2E 浏览器测试需要单独安装 Playwright browser；当前 CI 不把单元测试冒充 E2E。

## 平台验证

GitHub Actions 在 macOS 与 Windows 执行 Rust 格式、clippy、workspace tests 和构建检查；Ubuntu 执行前端 typecheck/unit/build。Windows `.cmd` shim 路径测试不运行脚本。开发者必须分别记录本地平台与 CI 实际结果；Windows CI 不能替代 macOS GUI 启动检查。
