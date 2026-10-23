import type { AppSettings } from "@/types";

/**
 * 应用设置服务。
 *
 * 用途：外观、密度、窗口行为、并发、日志保留、通知偏好。
 * 浏览器实现：MockSettingsService（内存 + localStorage 可选）。
 * Tauri：外观设置由 Rust Catalog 持久化；尚未实现的桌面能力以稳定错误返回。
 */
export interface SettingsService {
  /** 读取设置。调用时机：应用启动、进入 S13。 */
  getSettings(): Promise<AppSettings>;

  /** 局部更新设置。错误：`settings.invalid`。 */
  updateSettings(patch: Partial<AppSettings>): Promise<AppSettings>;

  /** 重置为默认（仅外观相关）。 */
  resetAppearance(): Promise<AppSettings>;

  /** 重置演示数据（仅 Browser Mock 支持；Tauri 返回未实现错误）。 */
  resetDemoData(): Promise<void>;
}
