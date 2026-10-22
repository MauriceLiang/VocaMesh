import type { LocalTool } from "@/types";

/**
 * 本地执行工具服务（CLI / 适配器）。
 *
 * 用途：回答“这台电脑上有哪些可运行的执行环境”，并维护认证状态。
 * 浏览器实现：MockToolService。
 * 桌面实现：Tauri commands 与 Rust Agent；认证状态仅使用官方确认安全的只读探测。
 */
export interface ToolService {
  /**
   * 扫描本机可执行工具。
   *
   * 调用时机：S01 首次启动、S07 重新扫描、安装后复检。
   * 返回：工具集合；只有认证状态可靠确认后才可为 ready。
   * 错误：`tool.scan-failed`（应降级为“无法检测”，不要假定可用）。
   * Mock：1.2s 后返回固定 4 条演示结果并广播“扫描完成”事件。
   * Tauri：返回三个 Provider 的本机发现结果，不读取配置中的密钥。
   */
  scanTools(): Promise<LocalTool[]>;

  /**
   * 验证指定工具的认证状态。
   *
   * 调用时机：S07「测试认证 / 验证登录 / 重新检测」。
   * 返回：更新后的工具；仍无法确认时保持 undetectable 并说明原因。
   * 错误：`tool.verify-timeout`、`tool.verify-failed`。
   * Mock：演示 auth-required → ready 状态迁移。
   * Tauri：只调用经过审核的固定版本/认证探测；认证不支持时不得假定成功。
   */
  verifyTool(id: string): Promise<LocalTool>;

  /**
   * 手动指定可执行文件路径。
   *
   * 调用时机：S07「手动指定路径」保存。
   * 说明：保存后状态变为 undetectable，必须再次验证；不会因为填写路径就假定可用。
   * 错误：`tool.invalid-path`。
   * Tauri：校验绝对路径、普通文件和可执行位；保存后仍需固定版本参数复核。
   */
  setToolPath(id: string, path: string): Promise<LocalTool>;

  /**
   * 设置默认执行工具。
   *
   * 调用时机：S07「设为默认」。
   * Tauri：只接受已验证 ready 的工具并写入非敏感应用设置。
   */
  setDefaultTool(id: string): Promise<void>;

  /** 读取当前默认工具 id。 */
  getDefaultToolId(): Promise<string | null>;
}
