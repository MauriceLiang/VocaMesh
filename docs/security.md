# 安全边界

## 本阶段允许的操作

| 能力 | 调用位置 | 目的 |
| --- | --- | --- |
| 固定 `--version` 子进程探测 | `vocamesh-agent`，由本地工具页触发 | 确认 CLI 是否可执行以及读取版本；3 秒超时，stdout/stderr 各限 8 KiB。 |
| 读取当前进程 `PATH` / `PATHEXT` | `vocamesh-agent` | 定位固定的三个 CLI 名称；不运行 Shell 初始化文件、不递归扫描用户目录。 |
| 读取/写入专属 `runtime-settings.json` | `vocamesh-agent` | 保存手动 CLI 路径和默认工具；临时文件 + 原子替换，拒绝未知字段。 |
| `tool-scan` 事件监听 | Tools 页，通过 `core:event:allow-listen` 和 `core:event:allow-unlisten` | 显示扫描开始、完成或失败；不携带 CLI 输出或认证材料。 |

## 明确不开放

- 不添加 Tauri Shell 插件、任意命令、Shell 字符串、任务进程、安装/更新/登录命令。
- 不读取 JSON/SQLite/keychain/配置认证文件、API Key、令牌、Cookie、SSH 私钥或环境变量内容。
- 不保存认证结果或 CLI 完整输出；不将探测结果发送到网络。
- 不把 `--version` 成功当成认证成功；不启动真实 Agent 工作模式。

手动路径由用户指定，执行前仍要求普通文件和可执行权限校验；随后只传固定版本参数。Windows 脚本 shim 拒绝启动。

## Tauri 权限

唯一 capability 绑定 `main` 窗口，只授予监听和取消监听事件的 core 权限。应用只注册 `scan_tools`、`verify_tool`、`set_tool_path`、`get_default_tool_id`、`set_default_tool`。前端不直接调用任意原生 IPC，Rust Core 不依赖 Tauri。
