# Claude Code CLI 隔离威胁模型与 PoC 状态

日期：2026-10-23
适用实现：VocaMesh v0.7 ProviderRunner
结论：**隔离未证明，Claude Code 仍不可执行。**

## 范围与安全目标

本文件评估 VocaMesh 将 Claude Code CLI 启动在过滤后的工作区快照中时，能否证明 CLI 及其附加执行逻辑只能读取已授权内容、不能改写源工作区或访问未授权秘密、不能留下未受控进程，也不能通过网络外传超出用户确认范围的数据。模型服务所需的请求本身会把 Prompt 和允许读取的文件发送给 Provider；“只读”不代表数据留在本机。

评估对象包括 VocaMesh Runtime、Claude Code CLI、组织托管策略、Hooks/插件、模型服务、操作系统凭据代理和本地 SQLite 历史。攻击者或故障来源包括恶意/误配置的 Hook、CLI 更新改变默认行为、工作区并发变更、路径替换、脱离原进程组的子进程、未知 managed settings、Provider 网络/遥测行为和解析器未识别的新协议事件。

## 资产与边界

| 资产 | 需要保护的内容 | 当前已知边界 |
| --- | --- | --- |
| 原始工作区 | 用户源文件与工作区中未授权路径 | Runtime 先验证路径，再复制过滤快照；CLI 的工作目录是快照。现有复制校验不等于整个 CLI 进程的 OS 级文件沙箱。 |
| 快照与任务历史 | 已授权输入、Prompt、输出、Run/Event 元数据 | SQLite schema v3 复用 Task/Run/Event；事件输出有长度上限和脱敏。数据库正文仍保留任务 Prompt、摘要与用户批准的路径。 |
| 认证材料 | CLI 登录态、Keychain/凭据服务、凭据 helper 与环境变量 | 认证由本机 CLI 管理。VocaMesh 不复制或解析凭据正文；CLI 与 Hooks 的实际凭据可见范围尚未被隔离 PoC 证明。 |
| 管理配置 | MDM、managed settings、远端策略和 Hook 定义 | VocaMesh 不读取管理配置正文；safe-mode 不证明 managed hooks 已停用。 |
| 进程树 | CLI、Hook、插件及其子进程 | Unix 通过进程组停止与退出检查；脱离进程组的后代不能由 PGID 单独证明已退出，未知状态保持 unknown。 |
| 网络与 Provider 数据 | 模型请求、遥测、Hook 产生的外连 | Provider 模型通信需要联网；VocaMesh 当前没有全进程出口白名单或对 Hook 网络行为的证明。 |

## 威胁与现有控制

| 威胁面 | 可能的失败方式 | 当前控制 | 结论 |
| --- | --- | --- | --- |
| 文件系统 | Hook/子进程读取快照外文件、读取凭据哨兵、改写用户源目录；symlink、挂载点或并发替换绕过路径检查 | Runtime 使用过滤快照；Unix 复制采用目录描述符相对打开、openat、O_NOFOLLOW 和元数据检查；快照完成前不创建 Run | 只能证明快照构造的部分性质，不能证明 CLI/Hooks 无法读取 Home、系统文件或其它可访问路径，也没有本轮源文件哈希对抗验收 |
| 凭据 | Hook、生成命令或插件读取 Keychain、凭据文件、helper 输出或环境变量并发送出去 | 任务进程使用 allowlist 环境；不把 Key/API token 环境变量传入；不复制认证文件 | Claude CLI 仍需本机认证能力，HOME/CLI 配置入口可能使 CLI 可访问账户能力。没有验证 Hook 侧无法调用凭据代理或读取授权材料 |
| 进程 | Hook 启动 shell/子进程、改变进程组、退出时遗留后台工作 | Provider 参数限制工具；Unix 独立 PGID、TERM/KILL 与组状态检查；无法确认时记为 unknown 并保留快照 | 参数不是 OS 进程沙箱；脱离 PGID 的进程、系统服务或 helper 的边界没有通过 PoC 证明 |
| 网络 | CLI、Hook、MCP 或插件连接未允许端点或外传快照/凭据 | 当前 Runtime 未配置全进程网络隔离；MCP 工具在 Claude 固定参数中禁用 | 无法证明网络出口范围。模型服务流量与 Hook 流量不能只凭 Prompt 区分 |
| 配置来源 | managed settings/MDM/远端策略变化，CLI 忽略普通配置禁用选项或新版本行为变化 | 保持 ManagedPolicyStatus::Unverified；Runtime 每次启动重检，不提供生产开关解除状态 | 继续 fail-closed |
| 数据与授权 | 用户以为目录“本地处理”，实际 Prompt/文件会发送给 Provider；权限改变后旧确认仍被复用 | UI 明示当前 Provider 与数据发送范围；只有后端 Runtime readiness 可放行；Run 保存用户确认时间 | 本轮 Codex 与 Claude 均无可放行的真实策略；真实桌面确认未验收 |
| 平台差异 | macOS、Windows、Linux 的进程/沙箱能力不同，或打包后行为改变 | Windows 真实执行仍关闭；Linux 未在本轮运行；macOS GUI/隔离 PoC 未执行 | 不能将 Unix PGID 或当前本机 fake 测试推广成跨平台隔离通过 |

## 隔离路线比较

| 路线 | 可提供的边界 | 未解决事项 | 当前决定 |
| --- | --- | --- | --- |
| A. 保持 Claude CLI fail-closed | 不增加 CLI 权限；既有源工作区和 Unknown Run 保护继续生效 | 真实 Claude 任务不可用 | **维持为默认策略** |
| B. OS 沙箱或独立 VM | 可研究限制可见文件、写入、进程和网络出口 | 必须证明 CLI 认证仍可用且 Hook 无法滥用凭据；还需逐系统审查更新、打包与进程树。没有采用未验证的私有 macOS 沙箱参数 | 可在隔离测试用户/设备上重新评估；本轮没有 PoC 证据 |
| C. 直接 API + VocaMesh 受控工具面 | 不启动 Claude Code CLI，也不执行其 managed hooks；VocaMesh 自己定义工具边界 | 需要维护者批准新的凭据、计费、网络和数据保留责任，并另行设计 | 仅作后续方案，不视为本轮授权 |

Codex CLI 是独立 Provider。其 sandbox read-only 只表达 Codex 自身命令沙箱设置，不能替代对 CLI 配置、凭据、插件、MCP、Hook、子进程、可读文件和网络出口的独立证明。因此，Codex 不会成为 Claude 门禁的替代开关。

## 本轮 PoC 记录

**状态：BLOCKED / NOT RUN。** 本机当前没有经确认的一次性测试用户或隔离测试设备；本轮没有创建或更改系统用户，没有向 Hook 注入认证材料，没有运行沙箱命令，没有访问系统级文件哨兵，也没有执行网络哨兵或以真实 Claude CLI 执行任务。因而没有可审查的 OS 拒绝日志、Hook 子进程证据、认证材料不可见证据、费用/网络证据或停止后代回收证据。

现有 fake CLI 生命周期测试只使用合成脚本与临时快照，证明 Runtime 的可重复本地状态映射；它们不运行 Claude 模型，也不证明 macOS OS 沙箱能力。构建 app bundle 也不覆盖 Finder 启动、窗口、目录选择或真实 Tauri IPC。

## 重新评估所需证据

未来 PoC 应在一次性测试用户或独立测试设备上，仅使用合成工作区、合成凭据哨兵和可审计的受限网络端点，并逐条保留：

1. macOS/OS 版本与隔离组件版本、实际生效配置及来源。
2. Hook 尝试读取受保护文件、写入源工作区、连接非允许端点、启动/脱离进程组的可复核拒绝记录。
3. CLI 认证仍可工作的证据，以及 Hook/模型生成命令无法读取或外传认证材料的证据。
4. 源文件哈希、快照生命周期、停止/强杀/重启后的全部进程与 Unknown Run 状态。
5. CLI 正常模型通信、代理、遥测、升级、资源消耗和打包兼容性的验证。
6. 未被覆盖的风险、人工操作与维护成本，并由维护者审阅新的 ADR 决策。

在这些证据与维护者新的 ADR 决策出现前，Claude Code 真实执行不得启用。当前生产 ManagedPolicyStatus::Unverified、execution.managed-policy-unverified 保持不变；测试用策略只存在于 cfg(test)，不通过 Tauri IPC、环境变量或前端暴露。
