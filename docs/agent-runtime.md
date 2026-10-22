# 本机 Agent CLI 发现

## Provider 与核实来源

| ID | CLI | 固定版本参数 | 认证探测 |
| --- | --- | --- | --- |
| `claude` | `claude` / Claude Code | `--version` | 不支持：官方文档没有确认满足本项目安全门槛的只读、非交互状态命令。 |
| `codex` | `codex` / Codex CLI | `--version` | 不支持：官方资料中出现的状态检查语境不足以确认不会刷新身份凭据或依赖认证文件。 |
| `opencode` | `opencode` / OpenCode | `--version` | 不支持：官方 `auth list` 会检查存放 provider 凭据的文件，按本项目边界不调用。 |

官方资料：

- [Tauri 2 + Vite 配置](https://v2.tauri.app/start/frontend/vite/)；[Tauri Core ACL 权限](https://v2.tauri.app/reference/acl/core-permissions/)。
- [Claude Code CLI reference](https://code.claude.com/docs/en/cli-usage)：文档列出 `claude --version`；当前设备核实版本为 `2.1.296`。
- [OpenAI Codex 官方安装器](https://github.com/openai/codex/blob/main/scripts/install/install.sh)：安装器通过 `codex --version` 校验安装；当前设备核实版本为 `codex-cli 0.162.1`。该命令出现一条因沙箱权限无法创建 PATH alias 的警告，退出码为 0。
- [OpenCode CLI](https://docs.opencode.ai/docs/cli/)：列出全局 `--version` 标志；[Provider 认证说明](https://docs.opencode.ai/docs/providers/)说明凭据文件位置。当前设备未安装 OpenCode。

以上版本是 2026-10-22 在本机以固定 `--version` 参数观察到的版本，不是 CI 安装的版本。Tauri 前端 API 和 CLI 包均解析到 `2.12.4`。

## 路径

先按当前进程 `PATH` 顺序查找，不启动交互 Shell、不扫描磁盘。macOS 补充 `/opt/homebrew/bin`、`/usr/local/bin`、`~/.local/bin` 和 `~/.bun/bin`。手动路径必须是规范化的绝对普通文件并具有可执行权限；文件消失或变成不可执行时显示 `undetectable`。

Windows 使用当前 `PATH` 与 `PATHEXT` 查找 `.exe`、`.cmd` 等候选。`.cmd` / `.bat` shim 不经 `cmd.exe` 启动，显示无法安全探测；不通过任意脚本启动器绕过边界。CI 在 Windows 上执行 shim 分类单元测试。

## 进程与输出

每个 CLI 适配器均仅传入固定 `--version` 参数，不用 `sh -c` 或 `bash -c`。stdin 关闭，进程硬超时 3 秒；stdout、stderr 分别最多捕获 8 KiB，超限、非零退出、启动失败、超时、非 UTF-8 或版本解析失败均映射为安全说明。原始输出只暂存在内存用于版本解析，不进入日志、DTO、事件或设置。

目前尚无官方资料确认三家 CLI 都提供符合“只读、非交互、不刷新凭据、不启动任务”的认证查询，因此认证状态标为 `unsupported` 或 `unknown`；绝不把 CLI 文件或版本存在视作已登录。`verify_tool` 仍可重读版本，但不会尝试认证。

## 持久化

应用配置目录只保存 `runtime-settings.json`：`schemaVersion`、`manualPaths`、`defaultToolId`。写入用同目录临时文件、同步后原子替换。损坏文件或不支持的 schema 返回 `settings.read-failed`；未知字段拒绝解析。认证结果、令牌、环境变量和命令输出不持久化。进程重启后先重新扫描，之前的默认值在未证实 `ready` 前不会被返回或使用。
