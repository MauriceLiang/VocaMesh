# Roadmap

## 已交付基础

- Vue + `Tauri` 2 桌面壳、Rust workspace/Core/Agent 分层和工具发现 IPC。
- Codex、`Claude Code`、OpenCode 的路径/版本发现；发现结果不等于可执行认证。

## v0.3 单 Agent 受控执行闭环（阶段验收未完成）

实现已通过 PR #3 合入 `main`（合并提交 `e6539a21cd79f49dbe47624d621af97ab77b35bf`）。已实现 `Runtime`、本地 SQLite `Task/Run/Event` 存储、幂等启动、`Claude` 固定认证探测和只读 CLI 参数、`JSONL allowlist/脱敏`、Unix 进程组停止/超时、过滤快照，以及桌面 `Tauri` Task/Run/Log 接入；浏览器仍使用完整 `Mock`。GitHub Actions workflow 按维护者要求移除。

本轮本地门禁通过：Rust workspace 37 项测试、`Clippy`、`Cargo check`、前端 typecheck、27 项 `Vitest` 和生产构建。Fake CLI 新增 `timeout` 持久化和畸形输出协议测试。**这些结果不代表真实 CLI 或 GUI 验收通过。**

真实 CLI 版本 `2.1.296`，认证安全分类结果为已认证。经开发者一次明确授权，通过 `Runtime` 对只含合成 README 的隔离目录发起一次只读任务；进程退出码为 1，Run 记录为 `execution.cli-failed`，无事件和可验证结果，本轮未重试。临时输入执行后保持不变。

尚未通过的阶段门禁：排查并成功验收真实 CLI 任务、macOS 原生窗口/目录选择/停止/重启恢复、Windows 完整桌面构建与进程树行为、Linux 本机编译/运行。完整 Windows 检查受当前 macOS 主机缺少 MSVC 工具链阻塞；`Linux target` 未安装；本机只有 `Command Line Tools`，`Tauri` 开发进程未呈现可验收窗口。没有 GitHub Actions 或 `Playwright E2E` 结果。v0.3 继续标记为未完成，详见 `implementation-report.md`。

## 后续阶段

- 先补齐受控执行生命周期与平台验收，再考虑可审核写权限、文件变更审批和 Codex/OpenCode Adapter。
- 多 Agent 编排依赖经过验证的单 Agent 生命周期、停止、日志、持久化与安全边界。
