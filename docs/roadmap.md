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

## v0.4 本地 Catalog 与原生桌面去 Mock

基于 `origin/main` 基线 `30707916b0d407d910a3550ac1e849018b6e9881` 的 `feature/v0.4-native-catalog` 实施。SQLite schema v3 在原数据库内增加 Agent 角色、权限配置草稿、Workspace、Team/成员关系、审计和外观偏好；v1 连续迁移到 v3，v2→v3 事务失败会回滚，v1/v2 的 Task、Run、Event 和恢复字段保留。原生 Tauri Services 显式接入 Catalog，失败显示错误并可重试，不回落到 Mock；浏览器 Mock 仍保留。

桌面页面现在以真实 Catalog 显示 Agent、Workspace、Team 与关系，审批/通知保持空状态，设置仅持久化支持的外观字段。Agent/Workspace 权限只保存为未激活配置，`ManagedPolicyStatus::Unverified` 与 `execution.managed-policy-unverified` 执行门禁未放宽；本阶段没有启动 CLI 或真实模型任务，也未恢复 GitHub Actions。

本轮 `./scripts/verify-local.sh` 退出码 0：Rust workspace 73 项测试（Agent 25、Core 2、Runtime 46），前端 11 个文件/41 项测试，以及 fmt、Clippy、Cargo check、typecheck、Vite build、diff 检查均 PASS。Tauri 编译包含在 workspace check 中。macOS 原生 GUI 因缺少完整 Xcode / 可验收窗口标记 BLOCKED；Windows、Linux、Playwright E2E 未验收。详细限制及逐项 GUI 清单见 `implementation-report.md` 与 `v0.4-gui-acceptance.md`。本地构建和自动化不能替代重启后的原生桌面验收，也不代表多 Agent 执行已实现。

## v0.5 本机 Agent 工具接入

`feature/v0.5-local-agent-integration` 基于 `origin/main` 的 `77d4bb22ac9e6fca8fe6c58e5e0a8e2c8d2a23e2`。建立 Provider 能力表及结构化发现/版本/认证/适配器/Runtime 状态；扩展有限 PATH 与 macOS 标准候选目录、规范化路径和 Provider 文件名校验；保存偏好不再依赖 readiness。Claude 认证探测复用既有安全实现，Codex/OpenCode 保持认证未知/不支持。Tauri 扫描、验证及路径保存均读取 Runtime readiness；浏览器 Mock 独立，schema v3 与 `execution.managed-policy-unverified` 未改变。

`./scripts/verify-local.sh` 退出码 0：Rust workspace 88 项测试（Agent 37、Core 3、Tauri 2、Runtime 46）、前端 11 个文件/44 项测试，以及 fmt、Clippy、Cargo check、typecheck、Vite build、diff 检查全部 PASS。本阶段没有运行任何认证状态命令或真实模型请求。macOS GUI 因缺少完整 Xcode/可验收窗口为 BLOCKED；Windows、Linux、Playwright E2E 和远端 CI 为 NOT RUN。逐项证据与当前分支提交/PR 状态见 `implementation-report.md`。

`pnpm --dir frontend tauri build --bundles app --no-sign` 另行成功生成 macOS `.app` bundle；跳过代码签名，且没有启动窗口，因此不改变 GUI BLOCKED 状态。

## v0.6 认证诊断与执行就绪度

`feature/v0.6-auth-readiness-diagnostics` 基于 `origin/main` 的 `76cafd0266ef15ec88c609eb3e1847b5156670f`。Claude Code 认证解析现在要求 `loggedIn`、`authMethod` 和退出码一致，并将未知格式、超时、失败和参数不兼容分别表达；普通扫描不会触发认证命令。Runtime readiness 返回结构化 checks/blockers，分别展示认证与 managed policy 状态；UI 会清除过期认证成功状态并在策略阻断时保持任务禁用。

本机固定只读 `claude auth status` 探测通过：Claude Code `2.1.296`，退出码 0，JSON object，`loggedIn=true`、`authMethod=oauth_token`。没有发送模型请求。生产 `execution.managed-policy-unverified` 保持 fail-closed；SQLite schema v3、只读执行边界和 Provider 范围未变。

`./scripts/verify-local.sh` 退出码 0：Rust workspace 99 项测试（Agent 40、Core 4、Tauri 2、Runtime 53），前端 11 个文件/47 项测试，以及 fmt、Clippy、Cargo check、typecheck、Vite build、diff 检查全部 PASS。macOS GUI 因 Computer Use 未获准访问应用为 BLOCKED；Windows、Linux、Playwright E2E、远端 CI 和真实模型任务本轮未运行。实施报告和 ADR 记录逐项证据。

随后执行 `pnpm --dir frontend tauri build --bundles app --no-sign` 成功，生成 arm64 `target/release/bundle/macos/VocaMesh.app`（14.63 MiB，未签名）。没有启动窗口，GUI 仍为 BLOCKED。

代码提交 `864f2c1dfdd1ce6a56a64d052c2ad700009b8129` 已推送；PR [#9](https://github.com/MauriceLiang/VocaMesh/pull/9) 已创建，待审阅。
