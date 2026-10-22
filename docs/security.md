# 安全边界

## 用户确认与数据发送

真实执行前，桌面 UI 展示 Claude Code Provider、用户选择的目录、只读工具、禁止的操作和数据说明。用户必须勾选确认并主动点击“开始只读任务”。Prompt、被读取的文件和输出会由用户本机 Claude Code CLI 发送至其配置的模型提供方。Prompt、结果和规范化目录路径保存在本机应用数据；不要在 Prompt 中放入密码或 Token。

VocaMesh 不读取、复制、记录或上传 API Key、令牌、Cookie、钥匙串内容、SSH 私钥或认证文件正文。认证探测只解析允许的 `authMethod` 与退出码；任务进程环境采用 allowlist，API key/helper/third-party 认证状态不会被标成可执行。

## 只读工作区

Runtime 每次都规范化并验证所选目录，拒绝文件系统根、Home 根和已知系统目录；不扫描磁盘。开始前把目录复制到应用数据下的临时快照，跳过符号链接、`.git`、`.ssh`、`.aws`、`.config`、`.claude`、`.codex`、`.opencode`、凭据目录、`.env`、密钥扩展名、常见凭据文件及构建/依赖目录。快照限制 100,000 个文件、512 MiB，任务正常退出后删除。历史路径不构成后续授权。

Claude 启动使用 `--restricted`、`--safe-mode`、`--permission-mode plan`、`--permission-prompts none` 和仅 `Read,Glob,Grep` 的工具集合，并禁用 MCP 工具。项目/用户自定义 Hook、插件、Agent、Shell、写入工具不加载或不可用。官方 CLI 说明 `--restricted` 限制内置文件工具到工作目录；managed policy 仍可应用，`--safe-mode` 不会关闭组织部署的 managed hooks。VocaMesh 无法阻止组织管理员配置的策略 Hook，此限制已在 UI/实施报告中记录。

文件排除名和输出脱敏是防御性过滤，不保证识别所有秘密。文件系统在复制时发生恶意并发改名/链接替换未做基于目录文件描述符的抗竞态隔离；不可将本阶段视为对不可信共享目录的安全沙箱。

## 进程、输出和恢复

- 不使用 Shell 字符串，不允许 Edit/Write/Bash/MCP/子 Agent，不自动重试。
- stdin 关闭，stderr 只排空并丢弃；stdout 仅解析白名单 JSONL 字段。未知 JSON 和原始工具输入不转发。
- 可见文本疑似包含已知 Token/Key 格式时遮蔽整条事件；正则/标记不能识别全部秘密。
- 单条事件最多 64 KiB，每 Run 最多保存 2 MiB；输出到上限后只保存一次截断事件，仍继续排空管道。
- 并发 Run 上限为 1，硬超时 10 分钟。Unix 进程组停止先 SIGTERM，等待 2 秒后 SIGKILL，只有确认进程组退出才显示停止。
- 正常关闭请求停止并等待最多 5 秒。重启把残留 active Run 标记为 `unknown`，不重跑；只要仍有无法确认进程退出的记录，readiness 会阻止新任务，直到数据库得到人工恢复。若应用被强制终止，无法证明的子进程不会自动恢复；进程组身份的跨重启回收尚未实现。
- Windows readiness 固定为不可执行，因为当前没有经验证的 Windows 进程树终止实现。

数据库损坏或 schema 版本更新时保留原文件并返回稳定错误，不静默删除或重建。Task/Run 历史、Prompt、摘要和目录路径可能包含敏感内容，均留在本机应用数据目录。

## Tauri 权限

主窗口 capability 只授予事件监听与取消监听，以及系统目录选择器 `dialog:allow-open`。Rust 只暴露固定的工具、目录选择和执行/历史命令；没有 Shell 插件、任意命令执行或任意文件 IPC。
