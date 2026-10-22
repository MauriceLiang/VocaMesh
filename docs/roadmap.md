# Roadmap

## 已交付基础

- Vue + Tauri 2 桌面壳、Rust workspace/Core/Agent 分层和工具发现 IPC。
- Codex、Claude Code、OpenCode 的路径/版本发现；发现结果不等于可执行认证。

## v0.3 单 Agent 受控执行闭环（实施中）

已实现代码：

- 新增 Runtime crate、本地 SQLite Task/Run/Event 存储、请求 ID 唯一性、状态与事件序号、启动时 active Run 恢复为 `unknown`。
- Claude Code 固定认证探测与只读 CLI 参数、JSONL allowlist/脱敏、Unix 进程组停止/超时。
- 原生目录选择、拒绝宽目录、跳过常见敏感文件与链接的过滤快照。
- Tauri 任务创建授权 UI；桌面 Task/Run/日志切换成本机服务，浏览器维持 Mock。
- 安全、架构、IPC、CLI 与开发文档更新。GitHub Actions workflow 按维护者要求移除。

尚未通过的阶段门禁：真实 Claude CLI 登录/任务手工验收、macOS GUI/目录选择/停止与重启验收、Windows 进程树策略与桌面执行、Linux 实际运行。Unix fake CLI 测试覆盖成功、非零退出、停止、事件与重复请求；远端 CI workflow 按维护者要求移除。实施状态仍为“进行中”，不能标为 v0.3 完成；详见 `implementation-report.md`。

## 后续阶段

- 先补齐受控执行生命周期与平台验收，再考虑可审核写权限、文件变更审批和 Codex/OpenCode Adapter。
- 多 Agent 编排依赖经过验证的单 Agent 生命周期、停止、日志、持久化与安全边界。
