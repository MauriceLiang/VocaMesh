# VocaMesh v0.7 ProviderRunner 与 Codex 执行闭环实施报告

> 状态：统一 ProviderRunner、Codex fake CLI Adapter、Provider 独立 readiness、共享 Runtime 生命周期及本地自动化 **PASS**；Claude 隔离 PoC 与原生 GUI 验收 **BLOCKED**；真实 Provider 模型任务 **NOT RUN**。生产真实执行继续 fail-closed。

## 基线与范围

- 分支：feature/v0.7-provider-runner-codex；基线：origin/main / 76cafd0266ef15ec88c609eb3e1847b5156670f1。fetch 后 origin/main 未前移。
- 保留原有 schema v3、单活跃 Run、SQLite Task/Run/Event、快照、停止/超时和 Unknown Run 恢复路径；没有新增 GitHub Actions workflow。
- 保留开始前未跟踪项：.DS_Store、.idea/、UI/.write-test、crates/.DS_Store、docs/.DS_Store；这些文件未纳入改动。

## 实施结果

- **M1 — PASS：Claude 认证状态解析。** 官方 JSON/退出码契约经一次明确授权，以固定只读 claude auth status 对本机 Claude Code 2.1.296 做状态探测，退出码 0，脱敏结果为 authenticated。只使用 authMethod 和退出码；原始 stdout/stderr 未保存，未读取凭据文件或发送 Prompt。测试覆盖全部已知非 none authMethod、none、矛盾退出码、未知枚举与畸形 JSON。
- **M2 — PASS：统一 ProviderRunner。** 静态分发接入 ClaudeCodeRunner 与 CodexCliRunner；保留一个公共 Runtime 调度与持久化协调器，不复制状态机或数据库 schema。
- **M3 — PASS：Codex fake Adapter。** Codex CLI 0.162.1 的版本及 codex exec --help 仅做本地只读能力核对。Runner 以固定独立参数启动 codex exec --json，Prompt 通过 stdin；只解析白名单 JSONL。fake CLI 覆盖成功、Provider 失败、缺少协议完成、停止、超时、流分块/EOF/非 UTF-8/超长事件，并验证 Task/Run/Event 生命周期。
- **M4 — PASS：独立执行策略。** Claude 的 execution.managed-policy-unverified 未改变；Codex 使用 execution.codex-safety-unverified，生产入口在创建 Task/Run 前 fail-closed。测试策略只在 cfg(test) 编译。Codex Adapter 存在不代表真实安全边界已验证。
- **M5 — PASS：Runtime、Tauri 与 UI 接线。** Provider-aware readiness 参数贯穿 ExecutionService、Tauri command、Runtime 与创建任务界面；不同 Provider 读取自己的 readiness。竞态回归测试覆盖 Provider 切换时迟到 readiness 不覆盖当前状态；requestId 幂等冲突现在也比较 providerId。数据库 schema v3 和既有 Run/event IPC 不变。
- **M6 — BLOCKED / NOT RUN：Claude 隔离 PoC。** 威胁模型及路线比较已记录；没有一次性测试用户/隔离设备和可审查系统级拒绝证据，未运行沙箱、Hook、文件/网络哨兵或真实 CLI 任务。
- **M7 — BLOCKED / NOT RUN：macOS 原生 GUI 验收。** 本轮构建了 app bundle，但没有 Finder/Launchpad 启动、窗口、目录选择、停止/恢复的人机验收证据。

## 本地质量门禁

首次以默认线程数运行 Runtime fake CLI 集成测试时，多个测试同时启动合成 CLI 探测，偶发触发 5 秒探测超时。只在测试中为会启动 fake CLI 的生命周期用例增加互斥，保留生产探测时限与并发代码；默认线程数下 Runtime 51 项测试及最终完整门禁随后通过。\n\n最终执行 ./scripts/verify-local.sh，退出码 0：

| 检查 | 结果 |
| --- | --- |
| cargo fmt --all -- --check | PASS |
| cargo clippy --workspace --all-targets --all-features --offline -- -D warnings | PASS |
| cargo test --workspace --offline --quiet | PASS，99 项（Agent 43、Core 3、Desktop 2、Runtime 51） |
| cargo check --workspace --all-targets --all-features --offline | PASS |
| pnpm --dir frontend typecheck | PASS |
| pnpm --dir frontend test | PASS，11 个文件、46 项 |
| pnpm --dir frontend build | PASS，Vite production build |
| git diff --check | PASS |

另执行 pnpm --dir frontend tauri build --bundles app --no-sign，退出码 0。产物为 target/release/bundle/macos/VocaMesh.app（14.61 MiB）；签名按要求跳过。app bundle 构建不证明 Finder 启动或 GUI 验收通过。

## 未验收范围与风险

- Claude 认证 authenticated 只表示 CLI 状态分类，不解除 managed hooks 门禁；Codex 未运行认证命令，未运行真实 Provider/模型任务，没有模型调用或 Prompt 上传。
- Claude 隔离 PoC、macOS GUI、Windows/Linux 本机验收、Playwright E2E 和远端 CI：BLOCKED 或 NOT RUN；未以本地自动化结果替代。
- Codex 的只读参数及过滤环境变量不是全进程 OS 沙箱证明。生产策略必须继续阻断，直到独立的配置/凭据/文件/进程/网络安全证据通过并由维护者重新决策。
- app 未签名且未启动；bundle 构建不提供 GUI 验收证据。

# VocaMesh v0.5 Local Agent Integration 实施报告

> 状态：M1–M6 实现与本地自动化门禁 PASS；M7 macOS GUI BLOCKED。没有进行真实 CLI 认证命令或模型任务，因此本报告不代表真实执行验收。

## 基线、分支与环境

- 仓库：`MauriceLiang/VocaMesh`；分支：`feature/v0.5-local-agent-integration`；基线：`origin/main` / `77d4bb22ac9e6fca8fe6c58e5e0a8e2c8d2a23e2`。
- 实施提交：`919a95970a6a532f39fb80488989aa7ad6e832cf`；PR [#8](https://github.com/MauriceLiang/VocaMesh/pull/8) 已创建，当前 open、未合并。
- Claude Code `2.1.296`、Codex CLI `0.162.1`；OpenCode CLI 未安装。只执行版本探测；没有运行 `claude auth status`、`codex login status`、`opencode auth list` 或真实模型任务。
- macOS 当前只有 Command Line Tools，缺少完整 Xcode；没有可验收的 VocaMesh 原生窗口。无关未跟踪项 `.DS_Store`、`.idea/`、`UI/.write-test`、`crates/.DS_Store`、`docs/.DS_Store` 保持未改动且不会纳入提交。

## 实施结果

- **M1 — PASS（状态契约）。** 建立 Core Provider 能力表并同步 Rust/TypeScript；发现、版本、认证、适配器和 Runtime readiness 独立表达，note 不参与决策。没有 SQLite 持久化需求，schema v3 未变。
- **M2 — PASS（有限发现实现）。** PATH 优先级固定，保留原 macOS 标准目录顺序并补充经过测试的常见包管理器 shim 目录；规范化去重并分别记录展示路径与 canonical 目标。直接固定 `--version`、超时/输出上限/稳定错误码、Provider 文件名校验和 Windows `.cmd`/`.bat` 拒绝均保留。没有 Shell、profile、递归 Home 扫描或凭据文件访问。
- **M3 — PASS（安全衔接实现）。** Claude Code 复用既有 `ClaudeCli` 固定参数、清理环境及严格认证字段解析；认证超时、探测失败、未知、未登录与已认证有不同状态/原因码。Codex/OpenCode 保持 unsupported/unknown；未运行其认证命令，也未读取凭据。官方资料与决策依据记录在 `docs/adr/0001-agent-execution-boundary.md`。
- **M4 — PASS（偏好与 UI）。** 默认工具仅表示偏好；扫描不会清除保存值。无执行器的 Provider 不能新设为偏好；Tauri 扫描、验证和保存路径响应都叠加 Runtime readiness，任务入口仍由 readiness 控制。Tauri 不回退 Mock，浏览器 Mock 独立。
- **M5 — PASS（决策门禁保持）。** `execution.managed-policy-unverified` 和既有 fail-closed 决策未改变；未增加生产绕过开关、执行后端或 schema 迁移。

## 本地质量门禁

最终执行 `./scripts/verify-local.sh`，脚本退出码 `0`。首次运行发现 Rust 格式未应用且 Clippy 的参数数量警告；格式化并将状态值收拢为结构体后重跑，八项全部通过。

| 命令 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets --all-features --offline -- -D warnings` | PASS |
| `cargo test --workspace --offline --quiet` | PASS，88 项测试（Agent 37、Core 3、Tauri 2、Runtime 46） |
| `cargo check --workspace --all-targets --all-features --offline` | PASS |
| `pnpm --dir frontend typecheck` | PASS |
| `pnpm --dir frontend test` | PASS，11 个文件、44 项测试 |
| `pnpm --dir frontend build` | PASS，Vite production build 完成 |
| `git diff --check` | PASS |

另运行 `pnpm --dir frontend tauri build --bundles app --no-sign`：PASS，产物 `target/release/bundle/macos/VocaMesh.app`，14.58 MiB。该构建跳过代码签名；仅证明 app bundle 成功生成，不代表窗口启动或 GUI 验收通过。

## 原生、平台与真实 CLI 验收

- **macOS GUI：BLOCKED。** `.app` 已构建，但当前只有 Command Line Tools，且没有可验收原生窗口；未操作 Finder/Launchpad、手动路径、重启持久性、Runtime 门禁或 IPC 错误 UI。
- **Windows：NOT RUN。** 没有在 Windows 环境构建或运行。
- **Linux：NOT RUN。** 没有在 Linux 环境构建或运行。
- **真实 CLI 认证：NOT RUN。** 没有调用 Claude/Codex/OpenCode 登录状态命令。
- **真实模型请求：NOT RUN。** 没有创建模型任务、发送 Prompt 或读取工作区数据。
- Playwright E2E 与远端 CI：NOT RUN。

本阶段交付本机工具发现、版本和能力状态接线，不代表真实 Claude Code 任务已验收、Codex/OpenCode 已接入执行器或多 Agent 执行已交付。自动化门禁完成后仍须保留上述 GUI/平台/真实服务边界。

---

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

---

# VocaMesh v0.3 验收闭环与 Runtime 加固报告

> 状态：**M1–M4 本地实现与自动化门禁通过；M5 真实 CLI 验收 BLOCKED；M6 GUI 验收 BLOCKED。v0.3 阶段仍未完成。** Fake CLI、静态检查与构建结果不替代真实 CLI、桌面 GUI 或跨平台验收。

## 基线与环境

- 仓库：`MauriceLiang/VocaMesh`；分支：`feature/v0.3-acceptance-closure`。
- 验证时 `HEAD`：`b743cb3c9a44dbbb7b162645f086b8e98b699e76`，与本轮实施分支基线一致。初始存在的无关未跟踪项 `.DS_Store`、`.idea/`、`UI/.write-test` 保持未修改。
- 系统：macOS 15.6，Apple Silicon arm64；Node `v24.22.1`、pnpm `9.15.9`、Rust `1.98.1`、Cargo `1.98.1`。
- 已安装 Rust targets：`aarch64-apple-darwin`、`x86_64-pc-windows-msvc`；Linux target 未安装。
- 当前开发者目录为 `/Library/Developer/CommandLineTools`；`xcodebuild -version` 失败并提示需要完整 Xcode。
- 本轮未运行 `claude --version`、`claude auth status` 或真实模型任务；报告中的 Claude Code `2.1.296` 和此前真实任务 exit code 1 均为上一轮既有记录，未在本轮重新验证。

## 实施结果

- **M1 — PASS（诊断实现）；原真实 exit code 1 原因仍未查明。** 版本、认证和任务命令共享同一份环境 allowlist。任务 stderr 在内存最多保留 16 KiB 并继续排空；分类器只产生固定错误码，不保存或展示原文。完整固定参数集合有精确测试。
- **M2 — PASS（自动化生命周期覆盖）。** 正常退出时检查 Unix 进程组，残留后代会被终止；无法确认退出、终态写入失败或结果为 `unknown` 时保留 PID/进程组信息和快照，并由 SQLite 状态阻止后续任务。终态成功写入后清理快照；清理失败不改写已持久化结果。重启后不会自动重跑未知任务。
- **M3 — PASS（实现与 fake/本地门禁）；生产执行仍受政策阻塞。** Unix/macOS 快照复制从已验证的工作区根目录描述符开始，以 `openat`/`O_NOFOLLOW`、`fstatat`/`fstat` 校验对象，按实际读取限制字节；符号链接、特殊文件、替换竞态和复制错误均阻止任务。managed policy 状态无法可靠核实时，生产 readiness 与任务入口返回 `execution.managed-policy-unverified` 并拒绝启动。Windows 真实执行仍关闭。
- **M4 — PASS。** Rust lifecycle/diagnostic/snapshot/store 用例、前端 readiness 门禁与本地验证脚本均已补齐。
- **M5 — BLOCKED，未执行。** 因 managed Claude Code hooks 是否会运行无法通过受支持的非交互方式可靠核实，按照 fail-closed 规则停止真实 CLI 成功验收。本轮未创建模型请求，也没有重试；上一轮 exit code 1 的根因仍未知。
- **M6 — BLOCKED，未执行 GUI 操作。** 当前只有 Command Line Tools，没有完整 Xcode；本轮未验证原生窗口、目录选择、真实任务交互、桌面停止或重启恢复。Windows、Linux 也未在本轮进行桌面运行验收。

## 本轮本地质量门禁

执行 `./scripts/verify-local.sh`，脚本退出码 `0`，逐项结果如下：

| 命令 | 结果 | 退出码 |
| --- | --- | ---: |
| `cargo fmt --all -- --check` | PASS | 0 |
| `cargo clippy --workspace --all-targets --all-features --offline -- -D warnings` | PASS | 0 |
| `cargo test --workspace --offline --quiet` | PASS，52 项测试（Agent 25、Core 2、Runtime 25） | 0 |
| `cargo check --workspace --all-targets --all-features --offline` | PASS | 0 |
| `pnpm --dir frontend typecheck` | PASS | 0 |
| `pnpm --dir frontend test` | PASS，7 个文件、28 项测试 | 0 |
| `pnpm --dir frontend build` | PASS | 0 |
| `git diff --check` | PASS | 0 |

新增的前端 readiness 定向用例最终通过。测试编写期间首次因 `VocaDialog` 使用 Teleport 而查询了错误的 DOM 容器，修正为挂载并检查 `document.body` 后，定向用例及完整前端套件均通过。Playwright E2E、GitHub Actions/远端 CI、PowerShell 脚本和 Linux/Windows 本机运行均未执行；仓库没有 `.github/workflows`，也未新增 workflow。

## 剩余限制

- 诊断分类器为后续受控诊断提供安全类别，但本轮没有重新启动真实 CLI，因此不能声称已解释此前的 exit code 1。
- 官方 CLI 的 managed settings/hooks 行为依据见 [CLI reference](https://code.claude.com/docs/en/cli-reference)、[settings files and precedence](https://code.claude.com/docs/en/settings)、[hooks guide](https://code.claude.com/docs/en/hooks-guide)。VocaMesh 当前没有受支持且不读取配置/凭据正文的手段可靠排除 managed hooks，真实执行保持 fail-closed。
- 当前没有跨重启的进程身份核验和人工解除 `unknown` 的 UI；PID 仅保留为诊断/恢复所需信息，不会单凭旧 PID 终止进程。
- 非 Unix 快照复制路径仍为保守实现；Windows 真实执行禁用，Linux 无本机验证。文件排除和输出脱敏仍是防御性过滤，不构成通用操作系统沙箱。
- 本地自动化全部通过，但不能据此标记真实任务、GUI、平台或 CI 验收通过；v0.3 阶段继续未完成。

---

# VocaMesh v0.3.1 Runtime 恢复与安全边界实施报告

> 状态：Runtime、数据库迁移、恢复 IPC、前端面板、ADR 与本地自动化 **PASS**；macOS 原生 GUI **BLOCKED**；真实 CLI/模型、Windows、Linux、Playwright E2E 和远端 CI 未验收。v0.3 真实执行仍保持 fail-closed，不能据此标记真实执行验收完成。

## 基线与工作区

- 基线：`origin/main` / `eb94b54b0e3b51e4e6c685a2caa3833a63f14930`；分支：`feature/v0.3.1-runtime-recovery`。
- 开始前重新运行 `./scripts/verify-local.sh`，作为本轮基线验证；结果通过。没有覆盖或清理既有未跟踪项 `.DS_Store`、`.idea/`、`UI/.write-test`。
- 系统：macOS Darwin 24.6 arm64。`xcode-select -p` 为 `/Library/Developer/CommandLineTools`；`xcodebuild -version` 明确报错：当前 developer directory 是 Command Line Tools，`xcodebuild` 需要完整 Xcode。

## 实施结果

- **M1 — PASS：ADR 0001。** 记录 Claude Code CLI、直接 API Provider、OS 沙箱/VM 和用户风险开关四种路线；当前选择继续 CLI fail-closed，其他真实执行路线等待维护者决策。依据链接指向 Anthropic 官方 CLI、settings 与 hooks 文档。
- **M2 — PASS：schema v2。** 新库初始化 v2；v1→v2 migration 在事务内增加 PGID、恢复状态和最近检查时间，不删除历史；未来 schema 版本拒绝打开。启动恢复会将 active Run 置为 `unknown`、保留原错误码并写固定审计事件；缺失 PGID 的旧记录标为 `manual_required`。
- **M3 — PASS：后端协议。** 提供列出、检查、恢复命令。Unix 仅将 `ESRCH` 作为退出候选；PGID 缺失、仍存在、权限/系统调用失败或不支持平台都不能归档。归档由 `start_gate` 串行化，要求用户确认，后端再次核验，事务将 Run 置为失败并审计，提交后才清理后端派生的快照路径。清理失败保持失败归档并写固定诊断，不重跑、不标成功、不解除 managed policy 门禁。
- **M4 — PASS：诊断与面板。** Readiness 增加稳定原因码；恢复面板接入创建入口和原生 Unknown Run 详情页。只展示短 Run ID、标题、稳定错误码、状态、快照存在情况和检查时间。PGID 不经 IPC 暴露；Mock 恢复列表为空且不调用 Tauri。
- **M5 — PASS：自动化。** 覆盖空库 v2、v1 历史迁移、migration 事务回滚、future schema 拒绝、错误码保留、恢复审计、PGID 状态分类、人工确认、进程组仍存在时阻止归档、数据库审计失败回滚、目录描述符安全清理及清理失败；前端覆盖人工处理状态、退出候选二次确认、IPC 错误保留数据和 Tauri/Mock 服务边界。
- **M6 — BLOCKED：未进行原生 GUI 验收。** 当前仅有 Command Line Tools 且没有完整 Xcode；未启动可验收原生窗口，未操作目录选择、恢复面板桌面流程或重启恢复。组件测试和构建不替代 GUI 验收。

## 本轮本地质量门禁

最终执行 `./scripts/verify-local.sh`，脚本退出码 `0`：

| 命令 | 结果 | 证据 |
| --- | --- | --- |
| `cargo fmt --all -- --check` | PASS | 脚本退出码 0 |
| `cargo clippy --workspace --all-targets --all-features --offline -- -D warnings` | PASS | 脚本退出码 0 |
| `cargo test --workspace --offline --quiet` | PASS | Agent 25、Core 2、Runtime 38，共 65 项 |
| `cargo check --workspace --all-targets --all-features --offline` | PASS | 脚本退出码 0，包含桌面 crate 检查 |
| `pnpm --dir frontend typecheck` | PASS | `vue-tsc --noEmit` |
| `pnpm --dir frontend test` | PASS | 8 个文件、32 项测试 |
| `pnpm --dir frontend build` | PASS | Vite 生产构建 |
| `git diff --check` | PASS | 脚本退出码 0 |

测试编写阶段，新 schema 测试首先按预期因空库仍为 v1 失败；实现迁移后通过。完整门禁第一次发现 3 条 Clippy 错误，完成最小修复后最终整套门禁再次通过。

## 未执行与剩余限制

- 本轮没有启动真实 CLI 任务、发起模型请求或产生新的模型调用费用；生产 readiness 仍由 `execution.managed-policy-unverified` 阻断。上一轮真实 CLI exit code 1 记录没有被本轮重试或覆盖。
- macOS 原生 GUI 为 **BLOCKED**（缺完整 Xcode/可验收窗口）；Windows、Linux、Playwright E2E、PowerShell 验证脚本和远端 CI 未执行。仓库没有新增 GitHub Actions workflow。
- PGID 不存在仅是必要检查之一，不能证明脱离原进程组的后代已退出；需要用户在系统进程管理工具中人工核验并确认。无法确认时记录、快照和新任务阻塞都保留。
- 本地 Rust、前端测试、静态检查和构建通过，不代表真实模型成功、桌面 GUI 验收或跨平台运行通过。v0.3 真实执行验收仍未完成。

---

# VocaMesh v0.4 Real Backend 实施报告

> 状态：Catalog 数据层、Tauri IPC、原生服务与页面联动及本地自动化 **PASS**；macOS 原生 GUI、Windows/Linux 桌面运行 **BLOCKED / 未验收**。真实 Agent 执行仍保持 fail-closed，v0.4 不代表多 Agent 执行能力已经交付。

## 基线与范围

- 仓库：`MauriceLiang/VocaMesh`；分支：`feature/v0.4-native-catalog`。
- 基线：`origin/main` / `30707916b0d407d910a3550ac1e849018b6e9881`，与本轮开始时的 `HEAD` 一致。
- 实施提交：`12c42d94b4ded046cf3f1d886c2b797ee1bdd1a2`；PR [#7 v0.4 Real Backend Catalog](https://github.com/MauriceLiang/VocaMesh/pull/7) 已创建，当前未合并。
- 保留初始存在的无关未跟踪项 `.DS_Store`、`crates/.DS_Store`、`.idea/`、`UI/.write-test`；本轮没有改动或暂存这些项。
- 本阶段实现共享 SQLite 文件中的 Agent、Workspace、Team、关系、审计和外观偏好配置；没有创建第二套 Runtime/数据库，也没有开发 Team 调度或 Agent 执行。

## 实施结果

- **schema v3 与迁移 — PASS。** 新库直接创建 v3；v1 通过原 v1→v2 和新增 v2→v3 连续迁移，v2→v3 采用事务。测试验证 v1/v2 的 Task、成功/失败/unknown Run、Event、未知 Run 的恢复字段及进程组 ID 保留；失败迁移回滚、未来 schema 拒绝打开；新 Catalog 表不含演示种子。
- **Rust Catalog — PASS。** Agent 角色及权限草稿、Workspace 与读/写/禁止范围、Team、主管和成员关系、外观偏好通过现有 `ExecutionRuntime`/SQLite 保存；新增关闭后重新打开 SQLite Store 的持久化测试覆盖实体、关系、范围草稿和外观设置。后端校验名称、Provider、路径、范围、外键、引用归档、版本冲突和组织环；重命名保持稳定 ID 关联。目录丢失时保留记录并把 `pathAvailable` 标为 false，且拒绝把失效目录绑定到新 Team 或作为改绑目标。
- **Tauri IPC / 原生 Services — PASS。** 增加 Catalog Commands 并注册到 Tauri；配置 CRUD 在 execution readiness 为 false 时仍可用。原生 `createServices()` 显式组装服务，Catalog IPC 或存储失败显示稳定错误并允许重试，不回退到 Mock；Approval/Notification 保持真实空状态。浏览器 Mock 服务及其演示数据仍可用。
- **桌面页面 — PASS（自动化范围）。** Agent、Workspace、Team 使用已保存记录及稳定 ID；Agent 状态区分“已配置”和“执行未开放”；原生 Workspace 支持系统目录选择器、改名、范围草稿和归档；Dashboard/CreateTask 不再用 Catalog Mock 种子绕过 readiness。原生隐藏模拟交通灯、Mock 标签和未实现的审批授权；即使 Pinia 中残留旧审批项，原生审批页面仍强制显示安全空态。任务筛选由实际 Workspace/Task 数据生成，Team 过滤器仅在 Mock 显示；原生 Task 不再伪造子任务。设置仅持久化外观字段，失败时保留草稿并提供重试；Workspace 编辑把载入时 revision 传到后端，避免旧草稿覆盖。
- **执行安全边界 — PASS（静态检查与自动化）。** `ManagedPolicyStatus::Unverified`、`execution.managed-policy-unverified` 与现有任务启动检查未放宽；权限和 Workspace 范围只作为未启用配置。没有启动真实 CLI、发起真实模型请求、读取凭据正文或新增 GitHub Actions。

## 本轮本地质量门禁

最终执行 `./scripts/verify-local.sh`，退出码 `0`，所有脚本步骤 PASS：

| 命令 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets --all-features --offline -- -D warnings` | PASS |
| `cargo test --workspace --offline --quiet` | PASS：73 项（Agent 25、Core 2、Runtime 46） |
| `cargo check --workspace --all-targets --all-features --offline` | PASS，包含 Tauri 桌面 crate |
| `pnpm --dir frontend typecheck` | PASS |
| `pnpm --dir frontend test` | PASS：11 个文件、41 项测试 |
| `pnpm --dir frontend build` | PASS：Vite 生产构建 |
| `git diff --check` | PASS |

新增测试覆盖迁移历史与回滚、Catalog 校验及关系、readiness 被阻断时的 Catalog 可用性、原生 Services 不回退 Mock、原生空状态、Pinia 错误/重试与外观草稿。通过后的测试结果均来自本轮本机运行；不代表 GUI 或平台验收。

## GUI 与平台验收边界

- **macOS 原生 GUI：BLOCKED。** 本机 `xcode-select -p` 指向 `/Library/Developer/CommandLineTools`，`xcodebuild -version` 要求完整 Xcode。没有可验收的原生窗口，因此未操作实际窗口交通灯、重启持久性、系统目录选择、完整跨页组织关系，也没有 GUI 截图。逐项状态记录于 `docs/v0.4-gui-acceptance.md`。
- **Windows / Linux：未验收。** 本轮未在这些平台构建或运行原生桌面应用。
- **真实 CLI / 模型：未运行。** 没有真实任务、模型费用或凭据正文访问；fake CLI 和 Rust 单测不是生产 Agent 验收。
- **Playwright E2E / 远端 CI：未运行。** 本轮只记录本地脚本实际执行结果；未新增或恢复 GitHub Actions。

## 结论与剩余限制

v0.4 的本地 Catalog 与桌面原生数据接线已完成，自动化质量门禁全部通过。实际桌面关闭/重启再打开、目录选择和跨页面生命周期尚未由人工 GUI 验收；Windows/Linux、Playwright E2E 也没有结果。原生功能面因此不能记为完整手工验收通过。`execution.managed-policy-unverified` 继续阻止真实任务，不能将此阶段描述为真实单 Agent 或多 Agent 执行已完成。
