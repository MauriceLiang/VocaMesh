# Runtime Foundation v0.2 实施报告

> 所有验证项只记录已实际执行的证据。

## 仓库与基线

- 仓库：`MauriceLiang/VocaMesh`
- 分支：`feature/runtime-foundation`
- 基线：`origin/main`，`e6d6482041114278989912c4133d50bd8a502f92`，已合入本分支。
- 实施提交 SHA：`b77a510ecc1582da75715a50d06a237fbe7af38e`；集成提交：`ff452e2c4143fa8c4a39d50009952407ddfa7fb2`
- PR：[#2 Runtime Foundation v0.2](https://github.com/MauriceLiang/VocaMesh/pull/2)

## 实际工具版本

- Node `v24.22.1`；pnpm `9.15.9`
- Rust `1.98.1`；Cargo `1.98.1`
- `@tauri-apps/api` `2.12.4`；`@tauri-apps/cli` `2.12.4`
- Rust crates: `tauri` `2.12.5`；`tauri-build` `2.7.4`
- Claude Code `2.1.296`；Codex CLI `0.162.1`；OpenCode 未安装
- 当前机器只有 Command Line Tools，`xcodebuild -version` 报告需要完整 Xcode。

## 完成范围

- 阶段 A：代码与配置已实施；前端继续保留原结构。`tauri dev` 完成 Rust 编译并报告启动 `target/debug/vocamesh-desktop`；桌面应用清单未显示 VocaMesh 窗口，因此没有完成窗口视觉验收。当前机器只有 Command Line Tools，缺少完整 Xcode。
- 阶段 B：三家 Provider 的 PATH/标准路径、固定版本探测、手动路径、非敏感设置和 IPC 已实施。认证探测对三家均保守标为 unsupported/unknown；没有可用工具会因版本存在而显示 `ready`。
- 未实现真实 Agent 任务执行、日志、取消、调度或认证文件读取。

## 测试与平台范围

- `cargo fmt --all -- --check`：通过。
- `cargo test --workspace`：通过，18 个单元测试全部通过；桌面 crate 与文档测试无用例。
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`：通过。
- `pnpm --dir frontend typecheck`：通过。
- `pnpm --dir frontend test`：通过，6 个测试文件、25 个测试全部通过。
- `pnpm --dir frontend build`：通过，Vite 生产构建完成。
- `git diff --check`：通过。
- `pnpm --dir frontend tauri dev`：Rust 应用编译并报告启动；未观察到可验收的原生窗口。

尚未运行浏览器 E2E、macOS GUI 视觉验收、Windows 桌面运行和真实 CLI 登录态验证。CI 远端结果需在推送后查看；单测和开发进程启动不代表这些检查通过。

---

# VocaMesh v0.3 单 Agent 执行闭环实施报告

> 状态：**实现已合并；阶段验收未通过。** PR 合并、自动化测试、真实 CLI、GUI 和平台证据分别记录，不能相互替代。

## 仓库与合并状态

- 仓库：`MauriceLiang/VocaMesh`。
- PR [#3 VocaMesh v0.3 controlled single-agent execution](https://github.com/MauriceLiang/VocaMesh/pull/3) 已合并；合并提交：`e6539a21cd79f49dbe47624d621af97ab77b35bf`。
- 本轮复核分支：`feature/v0.3-acceptance-evidence`，基于该合并提交，承载 Runtime 测试辅助和验收文档更新。
- `.github/workflows/ci.yml` 按维护者要求移除；本阶段没有 GitHub Actions 运行结果，也不记录为 CI 通过。

## 本轮实施与自动化验证

- `ExecutionRuntime` 保留生产默认 10 分钟超时；新增仅供单元测试使用的私有短超时入口。
- Fake CLI 补充超时被持久化为失败，以及畸形 JSONL 协议错误只保存固定文本、不保存原始输出的覆盖。
- `cargo fmt --all -- --check`：通过。
- `cargo test --workspace --offline --quiet`：通过，37 项测试（Agent 21、Core 2、`Runtime` 14）。
- `cargo clippy --workspace --all-targets --all-features --offline -- -D warnings`：通过。
- `cargo check --workspace --all-targets --all-features --offline`：通过。
- `pnpm --dir frontend typecheck`：通过。
- `pnpm --dir frontend test`：通过，6 个测试文件、27 项测试。
- `pnpm --dir frontend build`：通过。
- `git diff --check`：通过。

## 真实 CLI 验收

- 本机 `Claude Code` 版本为 `2.1.296`。`claude auth status` 已通过安全分类器判定为已认证；完整认证输出未展示或持久化。
- 开发者明确授权后，通过 VocaMesh `Runtime` 在 `/private/tmp` 的隔离临时目录启动了一次真实只读任务。工作区只有一份合成 `README.md`；任务只要求返回其中的合成词组。未读取 VocaMesh 仓库作为任务输入。
- 任务未通过：Run 终态为 `Crashed`，outcome 为 `failed`，稳定错误码 `execution.cli-failed`，CLI 退出码 1，事件数 0。没有得到可验证的模型结果；是否已联系模型提供方或产生费用无法确认。本轮没有重试。
- 任务后校验临时输入仍只有原 `README.md`，内容与执行前创建的合成内容一致（`SHA-256`：d3930b8c136fb905434cb7e415c81e4b3828de0d635dd9553c9489d0d9493954）。

## GUI 与平台范围

- `pnpm --dir frontend tauri dev` 曾完成 Vite 监听及桌面 Rust 二进制编译/启动，但桌面窗口清单未发现 VocaMesh；当前机器只有 `Command Line Tools`，缺少完整 Xcode。原生窗口、目录选择及桌面交互未验收。
- Windows 上 `vocamesh-core` 与 `vocamesh-agent` 的目标检查通过。完整 workspace 的 Windows 检查未通过：在线编译在 Apple clang 编译 bundled SQLite 时缺少 Windows MSVC 标准库/工具链；离线检查也曾因缓存缺少 `webview2-com` 失败。因此这不是 Windows 原生构建通过的证据。Windows 真实执行仍禁用。
- `Linux target` 未安装，未进行 Linux 编译或运行验收。
- 未运行 `Playwright E2E`、原生目录选择、真实任务成功/停止/重启恢复等 GUI 手工验收。

## 未完成门禁与已知限制

- 需要另一次明确授权后，再排查真实 CLI 退出码 1 的原因并完成成功结果验收；不能用 Fake CLI 结果替代。
- macOS 原生 GUI 和目录选择、停止与重启恢复尚未验收；Windows 完整桌面构建/进程树策略与 Linux 本机运行也尚未验收。
- managed policy 仍可能应用组织部署的 hooks；过滤快照与日志凭据规则是启发式防护，不构成通用文件系统沙箱。
- 文件系统在快照期间并发变化的竞态，以及崩溃后 `unknown Run` 的进程回收/人工解除流程，仍是待解决限制。
- 上述真实 CLI 与平台门禁未通过前，v0.3 阶段不能标记完成。
