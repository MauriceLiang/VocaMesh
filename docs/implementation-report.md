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

> 状态：**实施中，尚未达到阶段完成门禁。** 代码验证、真实 CLI、GUI、跨平台和 CI 证据分别记录，不相互替代。

## 分支与范围

- 仓库：`MauriceLiang/VocaMesh`；分支：`feature/single-agent-execution-v0.3`。
- 基线：`origin/main`，`3ef3176061c1260ee70a247a037a5912b7c77544`；v0.2 PR #2 已合并。
- 工作区含本阶段代码和文档改动；`.github/workflows/ci.yml` 按维护者要求删除。无 `git push --force`。
- 本地自动检查通过；提交与 PR 状态在交付时回填。

## 已实施代码

- 新增 `vocamesh-runtime`：SQLite schema v1、Task/Run/事件记录、幂等 request ID、活跃 Run 唯一约束、事件分页/2 MiB 上限、损坏库保留和 active Run 恢复为 `unknown`。状态未知且不能证明进程结束时，readiness 会阻止新启动。
- Core 增加 Execution、Task、Run 和 Event DTO；Agent 增加固定 Claude 认证探测、版本门槛、只读参数和 JSONL allowlist/脱敏。
- Tauri 增加系统目录选择和固定执行/历史 Commands。Unix 任务在过滤快照中运行，并用进程组停止/10 分钟超时；Windows readiness 固定为不可执行。
- 桌面 Task、Run 与日志接入 Tauri 服务；浏览器保持完整 Mock。新增用户确认和数据发送说明，历史日志按序号补齐/分页。
- 更新架构、IPC、CLI、安全、开发、路线图和实施报告。CLI 参数核对依据官方文档，尤其 `--restricted` 的文件工具边界和 `--permission-prompts none` 的版本要求。

## 自动化验证

- `cargo fmt --all -- --check`：通过。
- `cargo clippy --workspace --all-targets --all-features --offline -- -D warnings`：通过。
- `cargo test --workspace --offline --quiet`：通过，35 项测试（Agent 21、Core 2、Runtime 12）。
- Runtime fake CLI 生命周期测试：成功结果与事件持久化、同请求复用 Run、非零退出映射失败、停止后确认进程组退出。
- `cargo check --workspace --all-targets --all-features --offline`：通过。
- `pnpm --dir frontend typecheck`：通过。
- `pnpm --dir frontend test`：通过，6 个文件、27 项测试。
- `pnpm --dir frontend build`：通过，Vite 生产构建完成。
- `git diff --check`：通过。
- GitHub Actions workflow 已依照维护者要求移除，因此没有 CI workflow/check；不得把本地测试称为 CI 通过。

## 未验证项、限制与风险

- 未运行真实 `claude auth status` 输出到会话，也未触发真实模型任务；没有真实 CLI 手工登录/结果/费用证据。
- Fake CLI 自动化覆盖成功、非零退出、停止、输出事件持久化与重复请求；没有覆盖 fake timeout、启动失败、协议损坏、进程状态未知、崩溃恢复后的人工解除流程，也没有跨平台进程树测试。
- 未验收 macOS 原生窗口、目录选择、实时 CLI 日志、停止、重启与文件哈希对比。此前基线报告记录当前机器只有 Command Line Tools，缺完整 Xcode。
- Windows 真实执行保持禁用，未验证 Job Object/进程树终止；Linux 也未做本机实际运行验收。
- managed policy hook 仍可能由 Claude Code 应用；快照秘密文件排除和日志凭据规则属于启发式防护，不构成对任意共享目录或所有秘密的完整沙箱。
- 强制崩溃后残留 Run 进入 `unknown` 并阻止后续真实任务；跨重启安全回收进程组和人工解除状态流程未实现。
- 由于真实 CLI、GUI、跨平台与生命周期验收尚缺，v0.3 不能标记完成；是否创建 PR 需与阶段门禁一致，PR 不得宣称已通过未完成项。
