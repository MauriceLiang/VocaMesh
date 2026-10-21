import type { LocalTool } from "@/types";

/**
 * 本地执行工具服务（CLI / 适配器）。
 *
 * 用途：回答“这台电脑上有哪些可运行的执行环境”，并维护认证状态。
 * 当前实现：MockToolService。
 * 未来实现：Rust Core 的探测与认证状态读取；**不得**读取或保存凭据内容。
 */
export interface ToolService {
  /**
   * 扫描本机可执行工具。
   *
   * 调用时机：S01 首次启动、S07 重新扫描、安装后复检。
   * 返回：工具集合，状态为 ready / pending / auth-required / missing / incompatible / undetectable。
   * 错误：`tool.scan-failed`（应降级为“无法检测”，不要假定可用）。
   * Mock：1.2s 后返回固定 4 条结果并广播一次“扫描完成”事件。
   * Rust：读取 PATH、版本号、CLI 自身报告的登录状态；禁止读取配置中的密钥。
   */
  scanTools(): Promise<LocalTool[]>;

  /**
   * 验证指定工具的认证状态。
   *
   * 调用时机：S07「测试认证 / 验证登录 / 重新检测」。
   * 返回：更新后的工具；仍无法确认时保持 undetectable 并说明原因。
   * 错误：`tool.verify-timeout`、`tool.verify-failed`。
   * Mock：auth-required → ready；其余状态给出对应提示。
   * Rust：调用各适配器的 probe；超时不得视为成功。
   */
  verifyTool(id: string): Promise<LocalTool>;

  /**
   * 手动指定可执行文件路径。
   *
   * 调用时机：S07「手动指定路径」保存。
   * 说明：保存后状态变为 undetectable，必须再次验证；不会因为填写路径就假定可用。
   * 错误：`tool.invalid-path`。
   * Rust：校验路径存在与可执行位（只读校验，不执行任意命令）。
   */
  setToolPath(id: string, path: string): Promise<LocalTool>;

  /**
   * 设置默认执行工具。
   *
   * 调用时机：S07「设为默认」。
   * Rust：写入应用设置（settings.defaultTool）。
   */
  setDefaultTool(id: string): Promise<void>;

  /** 读取当前默认工具 id。 */
  getDefaultToolId(): Promise<string>;
}
