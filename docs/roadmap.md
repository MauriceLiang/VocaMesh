# Roadmap

## 已交付基础

- Vue + `Tauri` 2 桌面壳、Rust workspace/Core/Agent 分层和工具发现 IPC。
- Codex、`Claude Code`、OpenCode 的路径/版本发现；发现结果不等于可执行认证。

## v0.3 单 Agent 受控执行闭环（阶段验收未完成）

实现已通过 PR #3 合入 `main`（合并提交 `e6539a21cd79f49dbe47624d621af97ab77b35bf`）。已实现 `Runtime`、本地 SQLite `Task/Run/Event` 存储、幂等启动、`Claude` 固定认证探测和只读 CLI 参数、`JSONL allowlist/脱敏`、Unix 进程组停止/超时、过滤快照，以及桌面 `Tauri` Task/Run/Log 接入；浏览器仍使用完整 `Mock`。GitHub Actions workflow 按维护者要求移除。

本轮本地门禁通过：Rust workspace 37 项测试、`Clippy`、`Cargo check`、前端 typecheck、27 项 `Vitest` 和生产构建。Fake CLI 新增 `timeout` 持久化和畸形输出协议测试。**这些结果不代表真实 CLI 或 GUI 验收通过。**

真实 CLI 版本 `2.1.296`，认证安全分类结果为已认证。经开发者一次明确授权，通过 `Runtime` 对只含合成 README 的隔离目录发起一次只读任务；进程退出码为 1，Run 记录为 `execution.cli-failed`，无事件和可验证结果，本轮未重试。临时输入执行后保持不变。

尚未通过的阶段门禁：排查并成功验收真实 CLI 任务、macOS 原生窗口/目录选择/停止/重启恢复、Windows 完整桌面构建与进程树行为、Linux 本机编译/运行。完整 Windows 检查受当前 macOS 主机缺少 MSVC 工具链阻塞；`Linux target` 未安装；本机只有 `Command Line Tools`，`Tauri` 开发进程未呈现可验收窗口。没有 GitHub Actions 或 `Playwright E2E` 结果。v0.3 继续标记为未完成，详见 `implementation-report.md`。

### v0.3 验收闭环与 Runtime 加固（本轮）

基于 `b743cb3c9a44dbbb7b162645f086b8e98b699e76` 的 `feature/v0.3-acceptance-closure` 分支，本轮统一 CLI 版本/认证/任务环境，增加有界 stderr 安全分类，修复 unknown Run 的快照保留和 Unix 进程组退出确认，并以目录描述符和 `openat`/`O_NOFOLLOW` 加固快照复制。managed hooks 无法可靠核验时，生产 readiness 与任务入口均 fail-closed；Windows 真实执行仍禁用。

本轮 `./scripts/verify-local.sh` 八项本地门禁全部通过：Rust 52 项测试、前端 7 个文件/28 项测试，以及 fmt、Clippy、Cargo check、typecheck、生产构建和 diff 检查。新增前端用例验证策略 readiness 阻止启动。没有运行真实 CLI 或模型请求；M5 因 managed policy 无法核验标记 BLOCKED。macOS 原生 GUI 因缺少完整 Xcode/可验收桌面环境标记 BLOCKED；Windows、Linux、Playwright E2E 和远端 CI 均未在本轮验收。v0.3 阶段仍未完成，详见 `implementation-report.md`。

### v0.3.1 Runtime 恢复与安全边界

`feature/v0.3.1-runtime-recovery` 基于 `eb94b54b0e3b51e4e6c685a2caa3833a63f14930`。实现了 SQLite schema v2 和保留历史的 v1→v2 事务迁移、Unknown Run 恢复审计、Unix PGID 后端检查、用户人工核验后重新检查并失败归档、快照清理失败诊断、可区分的 readiness 原因码，以及桌面恢复面板。新建 ADR 0001，决定在可验证的 managed hooks 安全边界出现前继续保持 CLI fail-closed。生产 managed policy 门禁未变；没有启动真实 CLI 任务、调用模型或加入 GitHub Actions。

本轮 `./scripts/verify-local.sh` 退出码 0：Rust workspace 65 项测试（Agent 25、Core 2、Runtime 38）、前端 8 个文件/32 项测试，以及 fmt、Clippy、Cargo check、typecheck、生产构建和 diff 检查全部 PASS。macOS GUI 验收因当前只有 Command Line Tools、缺少完整 Xcode 标记 BLOCKED；真实 CLI/模型、Windows、Linux、Playwright E2E 与远端 CI 未验收。v0.3 真实执行阶段仍未完成。详细证据见 `implementation-report.md`。

## 后续阶段

- 先补齐受控执行生命周期与平台验收，再考虑可审核写权限、文件变更审批和 Codex/OpenCode Adapter。
- 多 Agent 编排依赖经过验证的单 Agent 生命周期、停止、日志、持久化与安全边界。
